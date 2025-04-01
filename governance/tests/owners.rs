// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    MultisigSignature, PublicKey as AccountPublicKey,
};
use emt_core::error;
use emt_core::governance::signature_messages;
use emt_core::{Account, AccountInfo, ZERO_ADDRESS};
use emt_tests::utils::rkyv_serialize;

pub mod common;
use common::instantiate::{TestKeys, TestSession, INITIAL_BALANCE, TOKEN_ID};
use common::{owner_signature, test_keys_signature};

const OWNER: usize = 10;
const OPERATOR: usize = 10;
const TEST: usize = 10;

#[test]
fn set_token_contract() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut owner_nonce = 0u64;

    // generate signature
    let new_token_contract = ContractId::from_bytes([42; CONTRACT_ID_BYTES]);
    let sig_msg = signature_messages::set_token_contract(
        owner_nonce,
        &new_token_contract,
    );
    let signers = vec![0u8, 1, 4, 5, 6, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_token_contract";
    let call_args = (new_token_contract, sig, signers);
    let old_token_contract = session
        .execute_governance::<(ContractId, MultisigSignature, Vec<u8>), ContractId>(
            &keys.test_sk[0],
            call_name,
            &call_args,
        )
        ?
        .data;

    // check that the old contract-ID is returned
    assert_eq!(old_token_contract, TOKEN_ID);
    // check that the token-contract on the governance-contract updated
    assert_eq!(
        session
            .query_governance::<(), ContractId>("token_contract", &())?
            .data,
        new_token_contract,
    );
    // check owner nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );

    Ok(())
}

#[test]
fn set_owners() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut owner_nonce = 0u64;

    //
    // test empty owner
    //

    // generate signature
    let new_owners = Vec::new();
    let sig_msg = signature_messages::set_owners(owner_nonce, vec![]);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_owners";
    let call_args = (new_owners, sig, signers);
    let contract_err = session
        .execute_governance
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (
            &keys.test_sk[0],
            call_name,
            &call_args
        )
        .expect_err("Call should panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::EMPTY_OWNER);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check owners not updated
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())?
            .data,
        keys.owners_pk
    );
    // check nonce is not incremented
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );

    //
    // test valid owner
    //

    // generate signature
    let new_owners: Vec<AccountPublicKey> = keys.test_pk.to_vec();
    let sig_msg = signature_messages::set_owners(owner_nonce, &new_owners);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_owners";
    let call_args = (new_owners, sig, signers);
    session
        .execute_governance
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (
            &keys.test_sk[0],
            call_name,
            &call_args
        )
        ?;

    // check updated owners
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())?
            .data,
        keys.test_pk
    );
    // check nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );

    //
    // test old owner keys don't work anymore
    //

    // sign with old owner keys
    let new_owners = vec![
        keys.owners_pk[0],
        keys.owners_pk[1],
        keys.owners_pk[2],
        keys.owners_pk[3],
    ];
    let sig_msg = signature_messages::set_owners(owner_nonce, &new_owners);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let old_owner_sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_owners";
    let call_args = (new_owners.clone(), old_owner_sig, signers.clone());
    let contract_err = session
        .execute_governance
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (
            &keys.test_sk[0],
            call_name,
            &call_args
        )
        .expect_err("Call should panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    //
    // test new keys do work
    //

    // sign with new owner keys
    let new_owner_sig = test_keys_signature(&keys, &sig_msg, &signers);
    let call_args = (new_owners.clone(), new_owner_sig, signers);
    session
        .execute_governance
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (&keys.test_sk[0], call_name, &call_args)
        ?;

    // check updated owners
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())?
            .data,
        new_owners
    );
    // check nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );

    Ok(())
}

#[test]
fn set_operators() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut owner_nonce = 0u64;

    //
    // test updating operator works
    //

    // generate signature
    let new_operators: Vec<AccountPublicKey> = keys.test_pk.to_vec();
    let sig_msg =
        signature_messages::set_operators(owner_nonce, &new_operators);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_operators";
    let call_args = (new_operators, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check updated operators
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("operators", &())?
            .data,
        keys.test_pk
    );
    // check owner nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );

    //
    // test new operators can execute operator functions
    //

    // generate signature with the new operator keys
    let mut operator_nonce = 0u64;
    let mint_amount = 1000;
    let mint_receiver = Account::from(keys.test_pk[0]);
    let token_call_name = "mint".to_string();
    let token_call_args = rkyv_serialize(&(mint_receiver, mint_amount));
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![4u8, 5, 6, 7, 8, 9];
    let sig = test_keys_signature(&keys, &sig_msg, &signers);

    let governance_call_name = "operator_token_call";
    let governance_call_args = (token_call_name, token_call_args, sig, signers);

    session.execute_governance::<_, ()>(
        &keys.test_sk[1],
        governance_call_name,
        &governance_call_args,
    )?;

    // check updated balance for mint-receiver
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>("account", &mint_receiver)?
            .data
            .balance,
        INITIAL_BALANCE + mint_amount,
    );
    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );

    Ok(())
}

#[test]
fn transfer_governance() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut owner_nonce = 0u64;

    //
    // test transferring governance on token-contract to a public key works
    //

    // generate signature
    let new_governance = Account::External(keys.test_pk[0]);
    let sig_msg =
        signature_messages::transfer_governance(owner_nonce, &new_governance);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "transfer_governance";
    let call_args = (new_governance, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check owner nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );
    // check governance updated on token-contract
    assert_eq!(
        session.query_token::<(), Account>("governance", &())?.data,
        new_governance,
    );

    Ok(())
}

#[test]
fn renounce_governance() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut owner_nonce = 0u64;

    //
    // test renouncing governance on token-contract works
    //

    // generate signature
    let sig_msg = signature_messages::renounce_governance(owner_nonce);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "renounce_governance";
    let call_args = (sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check owner nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );
    // check governance updated on token-contract
    assert_eq!(
        session.query_token::<(), Account>("governance", &())?.data,
        ZERO_ADDRESS,
    );

    Ok(())
}

#[test]
fn executing_operator_operations_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let owner_nonce = 0u64;

    //
    // test executing toggle-pause on token contract doesn't work
    //

    // generate signature
    let token_call_name = String::from("toggle_pause");
    let token_call_args = vec![];
    let sig_msg = signature_messages::operator_token_call(
        owner_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    let contract_err = session
        .execute_governance::<_, ()>(&keys.test_sk[0], call_name, &call_args)
        .expect_err("Call should not pass");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check owner nonce is not incremented
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        owner_nonce,
    );
    // check token-contract is not paused
    assert_eq!(
        session.query_token::<(), bool>("is_paused", &())?.data,
        false,
    );

    Ok(())
}
