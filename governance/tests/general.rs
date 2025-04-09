// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId};
use dusk_core::signatures::bls::{
    MultisigSignature, PublicKey as AccountPublicKey,
};

use emt_core::{error, Account, AccountInfo};

mod common;
use common::instantiate::{TestKeys, TestSession, INITIAL_BALANCE, TOKEN_ID};
use common::{operator_signature, owner_signature, test_keys_signature};

const OWNER: usize = 10;
const OPERATOR: usize = 10;
const TEST: usize = 10;

#[test]
fn init() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();

    // check correct initialization of token-contract
    assert_eq!(
        session
            .query_governance::<(), ContractId>("token_contract", &())?
            .data,
        TOKEN_ID,
    );

    // check correct initialization of owners and operators
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())?
            .data,
        keys.owners_pk
    );
    assert_eq!(
        session
            .query_governance::<(), u64>("owner_nonce", &())?
            .data,
        0
    );
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("operators", &())?
            .data,
        keys.operators_pk
    );
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())?
            .data,
        0
    );

    // check initialization of operator's token-contract calls and signature
    // thresholds
    assert_eq!(
        session
            .query_governance::<String, Option<u8>>(
                "operator_signature_threshold",
                &"block".to_string(),
            )?
            .data,
        Some(1)
    );
    assert_eq!(
        session
            .query_governance::<String, Option<u8>>(
                "operator_signature_threshold",
                &"mint".to_string(),
            )?
            .data,
        Some(6)
    );
    assert_eq!(
        session
            .query_governance::<String, Option<u8>>(
                "operator_signature_threshold",
                &"force_transfer".to_string(),
            )?
            .data,
        Some(6)
    );

    // technically not required here since it is checking the behavior of the
    // token-contract, but for making sure the tests are correctly
    // initialized we want to make sure that the keys have the expected
    // token balance
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>(
                "account",
                &Account::from(keys.test_pk[0]),
            )?
            .data
            .balance,
        INITIAL_BALANCE,
    );
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>(
                "account",
                &Account::from(keys.owners_pk[5]),
            )?
            .data
            .balance,
        INITIAL_BALANCE,
    );
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>(
                "account",
                &Account::from(keys.operators_pk[9]),
            )?
            .data
            .balance,
        INITIAL_BALANCE,
    );

    Ok(())
}

#[test]
fn double_init_fails() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();

    // create new init arguments with different owner and operator keys
    let governance_init_args = (
        TOKEN_ID,
        keys.test_pk[..5].to_vec(),
        keys.test_pk[5..].to_vec(),
        vec![],
    );

    // double init should return an error
    session
        .execute_governance::<(
            ContractId,
            Vec<AccountPublicKey>,
            Vec<AccountPublicKey>,
            Vec<(String, u8)>,
        ), ()>(&keys.test_sk[0], "init", &governance_init_args)
        .expect_err("Call should not pass");

    // check that owner didn't change
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())?
            .data,
        keys.owners_pk,
    );

    // check that operator didn't change
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("operators", &())?
            .data,
        keys.operators_pk,
    );

    Ok(())
}

#[test]
fn authorize_owners_passes() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let sig_msg = rand::random::<[u8; 32]>().to_vec();

    // more signers than threshold
    let threshold = 6;
    let signers = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_owners",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // more signers than threshold
    let threshold = 1;
    let signers = vec![0u8, 6, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_owners",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 1;
    let signers = vec![6u8];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_owners",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 10;
    let signers = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_owners",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 6;
    let signers = vec![1u8, 3, 4, 5, 7, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_owners",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    Ok(())
}

#[test]
fn authorize_owners_fails() -> Result<(), ContractError> {
    // we need one less owner key in the test session
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<{ OWNER + 1 }, OPERATOR, TEST> = TestKeys::new();
    let sig_msg = rand::random::<[u8; 32]>().to_vec();

    // threshold is zero
    let threshold = 0;
    let signers = vec![0u8, 1, 2, 3, 4];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_ZERO);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // duplicate signer
    let threshold = 2;
    let signers = vec![0u8, 0];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::DUPLICATE_SIGNER);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // invalid signer index, since we initialized the session with 10 owner
    // keys, the index = 10 is invalid
    let threshold = 1;
    let signers = vec![10u8];
    println!("here");
    let sig = owner_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::SIGNER_NOT_FOUND);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // less signers than threshold
    let threshold = 6;
    let signers = vec![0u8, 1, 2, 3, 4];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_NOT_MET);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // operator signature fails
    let threshold = 6;
    let signers = vec![0u8, 1, 3, 4, 6, 7];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // test-keys signature fails
    let threshold = 1;
    let signers = vec![3u8];
    let sig = test_keys_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_owners",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    Ok(())
}

#[test]
fn authorize_operators_passes() -> Result<(), ContractError> {
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, OPERATOR, TEST> = TestKeys::new();
    let sig_msg = rand::random::<[u8; 32]>().to_vec();

    // more signers than threshold
    let threshold = 6;
    let signers = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_operators",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // more signers than threshold
    let threshold = 4;
    let signers = vec![0u8, 6, 7, 8, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_operators",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 2;
    let signers = vec![6u8, 8];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_operators",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 10;
    let signers = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_operators",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    // signers equal threshold
    let threshold = 6;
    let signers = vec![1u8, 3, 4, 5, 7, 9];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    assert_eq!(
        session
            .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
                "authorize_operators",
                &(threshold, sig_msg.clone(), sig, signers)
            )?
            .data,
        (),
    );

    Ok(())
}

#[test]
fn authorize_operators_fails() -> Result<(), ContractError> {
    // we need one less owner key in the test session
    let mut session = TestSession::new::<OWNER, OPERATOR, TEST>();
    let keys: TestKeys<OWNER, { OPERATOR + 2 }, TEST> = TestKeys::new();
    let sig_msg = rand::random::<[u8; 32]>().to_vec();

    // threshold is zero
    let threshold = 0;
    let signers = vec![0u8];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_ZERO);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // duplicate signer
    let threshold = 5;
    let signers = vec![3u8, 4, 5, 6, 5];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::DUPLICATE_SIGNER);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // invalid signer index, since we initialized the session with 10 operator
    // keys, the index = 11 is invalid
    let threshold = 3;
    let signers = vec![2u8, 4, 11];
    println!("here");
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::SIGNER_NOT_FOUND);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // less signers than threshold
    let threshold = 6;
    let signers = vec![0u8, 1, 2, 3, 4];
    let sig = operator_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::THRESHOLD_NOT_MET);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // owner signature fails
    let threshold = 4;
    let signers = vec![0u8, 3, 4, 7];
    let sig = owner_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // test-keys signature fails
    let threshold = 1;
    let signers = vec![3u8];
    let sig = test_keys_signature(&keys, &sig_msg, &signers);
    let contract_err = session
        .query_governance::<(u8, Vec<u8>, MultisigSignature, Vec<u8>), ()>(
            "authorize_operators",
            &(threshold, sig_msg.clone(), sig, signers),
        )
        .expect_err("Call should panic");
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::INVALID_SIGNATURE);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    Ok(())
}
