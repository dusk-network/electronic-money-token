// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// use dusk_core::abi::ContractError;
use dusk_bytes::Serializable;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    MultisigSignature,
    PublicKey as AccountPublicKey,
    // SecretKey as AccountSecretKey,
};
// use dusk_core::transfer::MoonlightTransactionEvent;
//
// use rand::rngs::StdRng;
// use rand::SeedableRng;
//
// use emt_core::admin_management::PAUSED_MESSAGE;
// use emt_core::governance::arguments::TransferGovernance;
// use emt_core::governance::UNAUTHORIZED_ACCOUNT;
// use emt_core::sanctions::arguments::Sanction;
// use emt_core::sanctions::{BLOCKED, FROZEN};
// use emt_core::supply_management::SUPPLY_OVERFLOW;
// use emt_core::*;

pub mod common;
use common::instantiate::{TestKeys, TestSession, TOKEN_ID};
use common::owner_signature;

const OWNER: usize = 10;
const OPERATOR: usize = 10;
const HOLDER: usize = 10;

/*
 * Test owner functionalities
 */

#[test]
fn set_token_contract() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();

    // generate signature
    let new_token_contract = ContractId::from_bytes([42; CONTRACT_ID_BYTES]);
    let mut sig_msg = Vec::with_capacity(u64::SIZE + CONTRACT_ID_BYTES);
    sig_msg.extend(&0u64.to_be_bytes());
    sig_msg.extend(&new_token_contract.to_bytes());
    let signers = vec![0u8, 1, 4, 5, 6, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_token_contract";
    let call_args = (new_token_contract, sig, signers);
    let bytes = session
        .execute_governance::<(ContractId, MultisigSignature, Vec<u8>)>(
            &keys.holders_sk[0],
            call_name,
            &call_args,
        )
        .data
        .unwrap();
    let old_token_contract = rkyv::from_bytes::<ContractId>(&bytes)
        .expect("failed to deserialize previously serialized fn_arg");

    // check that the old contract-ID is returned
    assert_eq!(old_token_contract, TOKEN_ID);
    // check that the token-contract on the governance-contract updated
    assert_eq!(
        session
            .query_governance::<(), ContractId>("token_contract", &())
            .data,
        new_token_contract,
    );
}

#[test]
fn set_owners() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();
    let mut owner_nonce = 0u64;

    // test valid owner

    // generate signature
    let new_owners = keys.holders_pk.to_vec();
    let mut sig_msg = Vec::with_capacity(
        u64::SIZE + new_owners.len() * AccountPublicKey::SIZE,
    );
    sig_msg.extend(owner_nonce.to_be_bytes());
    new_owners
        .iter()
        .for_each(|pk| sig_msg.extend(&pk.to_bytes()));
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_owners";
    let call_args = (new_owners, sig, signers);
    session
        .execute_governance
        ::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>)>
        (&keys.holders_sk[0], call_name, &call_args)
        .data;

    // check updated owners
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())
            .data,
        keys.holders_pk
    );
    // check nonce is incremented
    owner_nonce += 1;
    assert_eq!(
        session.query_governance::<(), u64>("owner_nonce", &()).data,
        owner_nonce,
    );

    // test empty owner

    // generate signature
    let new_owners = Vec::new();
    let mut sig_msg = owner_nonce.to_be_bytes();
    let signers = vec![0u8, 2, 5, 7, 8, 9];
    let sig = owner_signature(&keys, &sig_msg, &signers);

    // call contract
    let call_name = "set_owners";
    let call_args = (new_owners, sig, signers);
    session
        .query_governance::<(Vec<AccountPublicKey>, MultisigSignature, Vec<u8>), ()>(call_name, &call_args)
        .data;

    // check owners not updated
    assert_eq!(
        session
            .query_governance::<(), Vec<AccountPublicKey>>("owners", &())
            .data,
        keys.holders_pk
    );
    // check nonce is not incremented
    assert_eq!(
        session.query_governance::<(), u64>("owner_nonce", &()).data,
        owner_nonce,
    );
}

/*
#[test]
fn set_operators() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();
}

#[test]
fn transfer_governance() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();
}

#[test]
fn renounce_governance() {
    let mut session = TestSession::new::<OWNER, OPERATOR, HOLDER>();
    let keys: TestKeys<OWNER, OPERATOR, HOLDER> = TestKeys::new();
}
*/

// more tests:
// - owner does not exist
// - too few signatures
