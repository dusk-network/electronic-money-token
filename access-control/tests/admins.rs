// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    MultisigSignature, PublicKey as AccountPublicKey,
};
use emt_core::access_control::{error, events, signature_messages};
use emt_core::{Account, AccountInfo, ZERO_ADDRESS};
use emt_tests::utils::rkyv_serialize;

pub mod common;
use common::instantiate::{
    TestKeys, TestSession, ACCESS_CONTROL_ID, INITIAL_BALANCE, TOKEN_ID,
};
use common::{admin_signature, test_keys_signature};

const ADMIN: usize = 10;
const OPERATOR: usize = 10;
const TEST: usize = 10;

#[test]
fn set_token_contract() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let mut admin_nonce = 0u64;

    // generate signature
    let new_token_contract = ContractId::from_bytes([42; CONTRACT_ID_BYTES]);
    let sig_msg = signature_messages::set_token_contract(
        admin_nonce,
        &new_token_contract,
    );
    let signers = vec![0u8, 1, 4, 5, 6, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_token_contract";
    let call_args = (new_token_contract, sig, signers);
    let receipt = session
        .execute_access_control::<(ContractId, MultisigSignature, Vec<u8>), ContractId>(
            &keys.test_sk[0],
            call_name,
            &call_args,
        )
        ?;

    // check that the correct event has been emitted
    let access_control_events: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.source == ACCESS_CONTROL_ID)
        .collect();
    assert_eq!(access_control_events.len(), 1);
    assert_eq!(access_control_events[0].topic, events::UpdateToken::TOPIC);
    // check that the old contract-ID is returned
    assert_eq!(receipt.data, TOKEN_ID);
    // check that the token-contract on the access-control-contract updated
    assert_eq!(
        session
            .query_access_control::<(), ContractId>("token_contract", &())?
            .data,
        new_token_contract,
    );
    // check admin nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );

    Ok(())
}

#[test]
fn set_admins() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let mut admin_nonce = 0u64;

    //
    // test empty admin
    //

    // generate signature
    let new_admins = Vec::new();
    let sig_msg = signature_messages::set_admins(admin_nonce, vec![]);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_admins";
    let call_args = (new_admins, sig, signers);
    let contract_err = session
        .execute_access_control
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (
            &keys.test_sk[0],
            call_name,
            &call_args
        )
        .expect_err("Call should panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::EMPTY_ADMINS);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check admins not updated
    assert_eq!(
        session
            .query_access_control::<(), Vec<AccountPublicKey>>("admins", &())?
            .data,
        keys.admins_pk
    );
    // check nonce is not incremented
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );

    //
    // test valid admin
    //

    // generate signature
    let new_admins: Vec<AccountPublicKey> = keys.test_pk.to_vec();
    let sig_msg = signature_messages::set_admins(admin_nonce, &new_admins);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_admins";
    let call_args = (new_admins, sig, signers);
    let receipt = session
        .execute_access_control
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (
            &keys.test_sk[0],
            call_name,
            &call_args
        )
        ?;

    // check that the correct event has been emitted
    let access_control_events: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.source == ACCESS_CONTROL_ID)
        .collect();
    assert_eq!(access_control_events.len(), 1);
    assert_eq!(
        access_control_events[0].topic,
        events::UpdatePublicKeys::NEW_ADMINS
    );
    // check updated admins
    assert_eq!(
        session
            .query_access_control::<(), Vec<AccountPublicKey>>("admins", &())?
            .data,
        keys.test_pk
    );
    // check nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );

    //
    // test old admin keys don't work anymore
    //

    // sign with old admin keys
    let new_admins = vec![
        keys.admins_pk[0],
        keys.admins_pk[1],
        keys.admins_pk[2],
        keys.admins_pk[3],
    ];
    let sig_msg = signature_messages::set_admins(admin_nonce, &new_admins);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let old_admin_sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_admins";
    let call_args = (new_admins.clone(), old_admin_sig, signers.clone());
    let contract_err = session
        .execute_access_control
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

    // sign with new admin keys
    let new_admin_sig = test_keys_signature(&keys, &sig_msg, &signers);
    let call_args = (new_admins.clone(), new_admin_sig, signers);
    session
        .execute_access_control
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>
        (&keys.test_sk[0], call_name, &call_args)
        ?;

    // check updated admins
    assert_eq!(
        session
            .query_access_control::<(), Vec<AccountPublicKey>>("admins", &())?
            .data,
        new_admins
    );
    // check nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );

    Ok(())
}

#[test]
fn set_operators() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let mut admin_nonce = 0u64;

    //
    // test updating operator works
    //

    // generate signature
    let new_operators: Vec<AccountPublicKey> = keys.test_pk.to_vec();
    let sig_msg =
        signature_messages::set_operators(admin_nonce, &new_operators);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_operators";
    let call_args = (new_operators, sig, signers);
    let receipt = session.execute_access_control::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check that the correct event has been emitted
    let access_control_events: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.source == ACCESS_CONTROL_ID)
        .collect();
    assert_eq!(access_control_events.len(), 1);
    assert_eq!(
        access_control_events[0].topic,
        events::UpdatePublicKeys::NEW_OPERATORS
    );
    // check updated operators
    assert_eq!(
        session
            .query_access_control::<(), Vec<AccountPublicKey>>(
                "operators",
                &()
            )?
            .data,
        keys.test_pk
    );
    // check admin nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
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

    let access_control_call_name = "operator_token_call";
    let access_control_call_args =
        (token_call_name, token_call_args, sig, signers);

    session.execute_access_control::<_, ()>(
        &keys.test_sk[1],
        access_control_call_name,
        &access_control_call_args,
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
            .query_access_control::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );

    Ok(())
}

#[test]
fn transfer_ownership() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let mut admin_nonce = 0u64;

    //
    // test transferring ownership on token-contract to a public key works
    //

    // generate signature
    let new_ownership = Account::External(keys.test_pk[0]);
    let sig_msg =
        signature_messages::transfer_ownership(admin_nonce, &new_ownership);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "transfer_ownership";
    let call_args = (new_ownership, sig, signers);
    session.execute_access_control::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check admin nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );
    // check ownership updated on token-contract
    assert_eq!(
        session.query_token::<(), Account>("ownership", &())?.data,
        new_ownership,
    );

    Ok(())
}

#[test]
fn renounce_ownership() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let mut admin_nonce = 0u64;

    //
    // test renouncing ownership on token-contract works
    //

    // generate signature
    let sig_msg = signature_messages::renounce_ownership(admin_nonce);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "renounce_ownership";
    let call_args = (sig, signers);
    session.execute_access_control::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check admin nonce is incremented
    admin_nonce += 1;
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );
    // check ownership updated on token-contract
    assert_eq!(
        session.query_token::<(), Account>("ownership", &())?.data,
        ZERO_ADDRESS,
    );

    Ok(())
}

#[test]
fn executing_operator_operations_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<ADMIN, OPERATOR, TEST>();
    let keys: TestKeys<ADMIN, OPERATOR, TEST> = TestKeys::new();
    let admin_nonce = 0u64;

    //
    // test executing toggle-pause on token contract doesn't work
    //

    // generate signature
    let token_call_name = String::from("toggle_pause");
    let token_call_args = vec![];
    let sig_msg = signature_messages::operator_token_call(
        admin_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = admin_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    let contract_err = session
        .execute_access_control::<_, ()>(
            &keys.test_sk[0],
            call_name,
            &call_args,
        )
        .expect_err("Call should not pass");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check admin nonce is not incremented
    assert_eq!(
        session
            .query_access_control::<(), u64>("admin_nonce", &())?
            .data,
        admin_nonce,
    );
    // check token-contract is not paused
    assert_eq!(
        session.query_token::<(), bool>("is_paused", &())?.data,
        false,
    );

    Ok(())
}
