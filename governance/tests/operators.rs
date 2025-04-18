// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::ContractError;
use emt_core::governance::{error, signature_messages};
use emt_core::{Account, AccountInfo};
use emt_tests::utils::rkyv_serialize;

pub mod common;
use common::instantiate::{
    TestKeys, TestSession, GOVERNANCE_ID, INITIAL_BALANCE,
};
use common::{operator_signature, test_keys_signature};

const OWNER: usize = 10;
const OPERATOR: usize = 10;
const TEST: usize = 10;

/*
 * Test `operator_token_call`
 */

#[test]
fn unregistered_operator_token_call_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let operator_nonce = 0u64;

    // generate signature
    let token_call_name = String::from("unregistered_call");
    let token_call_args = rkyv_serialize(&());
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![3u8];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    let contract_err = session
        .execute_governance::<_, ()>(&keys.test_sk[0], call_name, &call_args)
        .expect_err("Call should not pass");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::TOKEN_CALL_NOT_FOUND);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check operator nonce is not incremented
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );

    Ok(())
}

#[test]
fn freeze_operator_token_call() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut operator_nonce = 0u64;

    // generate signature
    let token_call_name = String::from("freeze");
    let freeze_account = Account::External(keys.test_pk[0]);
    let token_call_args = rkyv_serialize(&freeze_account);
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![7u8];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check account is frozen on token-contract
    assert_eq!(
        session
            .query_token::<Account, bool>("frozen", &freeze_account)?
            .data,
        true,
    );

    Ok(())
}

#[test]
fn invalid_signature_operator_token_call_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let operator_nonce = 0u64;

    // generate signature with the owner keys
    let token_call_name = String::from("freeze");
    let freeze_account = Account::External(keys.test_pk[0]);
    let token_call_args = rkyv_serialize(&freeze_account);
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![7u8];
    let sig = test_keys_signature(&keys, &sig_msg, &signers);

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
    // check operator nonce is not incremented
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );

    Ok(())
}

#[test]
fn burn_operator_token_call() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut operator_nonce = 0u64;

    //
    // test calling burn with less than a super-majority of signers doesn't work
    //

    // generate signature
    let token_call_name = String::from("burn");
    let burn_amount = 1000u64;
    let token_call_args = rkyv_serialize(&burn_amount);
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![1u8, 2, 4, 7, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    let contract_err = session
        .execute_governance::<_, ()>(&keys.test_sk[0], call_name, &call_args)
        .expect_err("Call should not pass");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_NOT_MET);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check operator nonce not incremented
    assert_eq!(
        session
            .query_governance::<_, u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check total-supply didn't change
    // all keys and the governance-contract hold the initial balance at
    // initialization
    let initial_supply =
        (OWNER + OPERATOR + TEST) as u64 * INITIAL_BALANCE + INITIAL_BALANCE;
    assert_eq!(
        session.query_token::<_, u64>("total_supply", &())?.data,
        initial_supply,
    );

    //
    // test calling burn with a super-majority of signers works
    //

    // generate signature
    let token_call_name = String::from("burn");
    let burn_amount = 1000u64;
    let token_call_args = rkyv_serialize(&burn_amount);
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![1u8, 2, 4, 5, 7, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<_, u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check total-supply has decreased
    assert_eq!(
        session.query_token::<_, u64>("total_supply", &())?.data,
        initial_supply - burn_amount,
    );

    Ok(())
}

#[test]
fn force_transfer_operator_token_call() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut operator_nonce = 0u64;

    //
    // test calling forced-transfer with a super-majority of signers works
    //

    // generate signature
    let token_call_name = String::from("force_transfer");
    let obliged_sender = Account::from(keys.test_pk[3]);
    let receiver = Account::from(keys.test_pk[5]);
    let value = 100u64;
    let token_call_args = rkyv_serialize(&(obliged_sender, receiver, value));
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![1u8, 2, 4, 5, 7, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<_, u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check obliged sender funds decreased
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>("account", &obliged_sender)?
            .data
            .balance,
        INITIAL_BALANCE - value,
    );
    // check receiver funds increased
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>("account", &receiver)?
            .data
            .balance,
        INITIAL_BALANCE + value,
    );

    Ok(())
}

/*
 * Test `set_operator_token_call`
 */

#[test]
fn set_operator_token_call() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let mut operator_nonce = 0u64;

    //
    // test updating threshold of token call works
    //

    // generate signature
    let token_call_name = String::from("block");
    let new_threshold = 3u8;
    let sig_msg = signature_messages::set_operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        new_threshold,
    );
    let signers = vec![1u8, 2, 4, 5, 7, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_operator_token_call";
    let call_args = (token_call_name.clone(), new_threshold, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check threshold updated
    assert_eq!(
        session
            .query_governance::<String, u8>(
                "operator_signature_threshold",
                &token_call_name
            )?
            .data,
        new_threshold,
    );
    // check call with previous lower threshold now panics
    let block_account = Account::External(keys.test_pk[0]);
    let token_call_args = rkyv_serialize(&block_account);
    let sig_msg = signature_messages::operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        &token_call_args,
    );
    let signers = vec![4u8];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let call_name = "operator_token_call";
    let call_args = (token_call_name, token_call_args, sig, signers);
    let contract_err = session
        .execute_governance::<_, ()>(&keys.test_sk[0], call_name, &call_args)
        .expect_err("Call should not pass");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_NOT_MET);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    //
    // test adding token call works
    //

    let token_call_name = String::from("transfer");
    let sig_msg = signature_messages::set_operator_token_call(
        operator_nonce,
        token_call_name.as_str(),
        new_threshold,
    );
    let signers = vec![0u8, 3, 4, 5, 8, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_operator_token_call";
    let call_args = (token_call_name.clone(), new_threshold, sig, signers);
    session.execute_governance::<_, ()>(
        &keys.test_sk[0],
        call_name,
        &call_args,
    )?;

    // check operator nonce is incremented
    operator_nonce += 1;
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check threshold is correct
    assert_eq!(
        session
            .query_governance::<String, u8>(
                "operator_signature_threshold",
                &token_call_name
            )?
            .data,
        new_threshold,
    );

    Ok(())
}

/*
 * Test token calls needing owner approval fail
 */

#[test]
fn renounce_governance_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let operator_nonce = 0u64;

    //
    // test renouncing governance on token-contract fails with operator approval
    //

    // generate signature
    let sig_msg = signature_messages::renounce_governance(operator_nonce);
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "renounce_governance";
    let call_args = (sig, signers);
    let contract_err = session
        .execute_governance::<_, ()>(&keys.test_sk[0], call_name, &call_args)
        .expect_err("Call should not pass");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check operator nonce is not incremented
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        operator_nonce,
    );
    // check governance not updated on token-contract
    assert_eq!(
        session.query_token::<(), Account>("governance", &())?.data,
        GOVERNANCE_ID.into(),
    );

    Ok(())
}
