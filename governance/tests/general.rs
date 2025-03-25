// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// use dusk_core::abi::ContractError;
// use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
// use dusk_core::transfer::MoonlightTransactionEvent;
//
// use rand::rngs::StdRng;
// use rand::SeedableRng;

use emt_core::{Account, AccountInfo};
// use emt_core::admin_management::PAUSED_MESSAGE;
// use emt_core::governance::arguments::TransferGovernance;
// use emt_core::governance::UNAUTHORIZED_ACCOUNT;
// use emt_core::sanctions::arguments::Sanction;
// use emt_core::sanctions::{BLOCKED, FROZEN};
// use emt_core::supply_management::SUPPLY_OVERFLOW;
// use emt_core::*;

mod common;
use common::instantiate::{TestKeys, TestSession, INITIAL_BALANCE, TOKEN_ID};

const OWNER: usize = 10;
const OPERATOR: usize = 10;
const HOLDER: usize = 10;

#[test]
fn init() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();

    // check correct initialization of token-contract
    assert_eq!(
        session
            .query_governance::<(), ContractId>("token_contract", &())
            .data,
        TOKEN_ID,
    );

    // check correct initialization of owners and operators
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())
            .data,
        keys.owners_pk
    );
    assert_eq!(
        session.query_governance::<(), u64>("owner_nonce", &()).data,
        0
    );
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("operators", &())
            .data,
        keys.operators_pk
    );
    assert_eq!(
        session
            .query_governance::<(), u64>("operator_nonce", &())
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
            )
            .data,
        Some(1)
    );
    assert_eq!(
        session
            .query_governance::<String, Option<u8>>(
                "operator_signature_threshold",
                &"mint".to_string(),
            )
            .data,
        Some(6)
    );
    assert_eq!(
        session
            .query_governance::<String, Option<u8>>(
                "operator_signature_threshold",
                &"forced_transfer".to_string(),
            )
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
                &Account::from(keys.holders_pk[0]),
            )
            .data
            .balance,
        INITIAL_BALANCE,
    );
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>(
                "account",
                &Account::from(keys.owners_pk[5]),
            )
            .data
            .balance,
        INITIAL_BALANCE,
    );
    assert_eq!(
        session
            .query_token::<Account, AccountInfo>(
                "account",
                &Account::from(keys.operators_pk[9]),
            )
            .data
            .balance,
        INITIAL_BALANCE,
    );
}

// more test:
// double-init
// owners cannot execute operators functions
// operators cannot execute owners functions
