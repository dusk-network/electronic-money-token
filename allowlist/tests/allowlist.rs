// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::ContractError;
// use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
// use dusk_core::signatures::bls::{
//     PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
// };
// use dusk_core::transfer::data::ContractCall;
// use dusk_core::transfer::MoonlightTransactionEvent;
//
// use rand::rngs::StdRng;
// use rand::SeedableRng;
//
// use emt_core::token::error;
// use emt_core::token::events;
use emt_core::allowlist::error;
use emt_core::allowlist::{Address, Role};
use emt_core::{
    Account,
    ZERO_ADDRESS,
    // AccountInfo, ZERO_ADDRESS
};

pub mod instantiate;
use instantiate::{
    TestSession,
    // HOLDER_ID, INITIAL_EMT_BALANCE
};

#[test]
fn deploy() {
    TestSession::new();
}

#[test]
fn double_init() -> Result<(), ContractError> {
    let mut session = TestSession::new();

    // create new init arguments
    let new_user = Address::from(&[42u8; 32]);
    let new_ownership = Account::from(*TestSession::PK_1);

    // call init on the allowlist in an attempt to override the state without
    // ownership approval
    session
        .allowlist_init(
            &*TestSession::SK_1,
            vec![(new_user, *TestSession::ROLE_0)],
            new_ownership,
        )
        .unwrap_err();

    // check ownership didn't update
    assert_eq!(
        session.allowlist_ownership()?.data,
        Account::from(*TestSession::PK_0)
    );
    // check users didn't update
    assert_eq!(
        session
            .allowlist_has_role(&*TestSession::ADDRESS_0)
            .expect("call should be successfull")
            .data,
        Some(*TestSession::ROLE_0),
    );
    assert_eq!(
        session
            .allowlist_has_role(&*TestSession::ADDRESS_1)
            .expect("call should be successfull")
            .data,
        Some(*TestSession::ROLE_1),
    );

    Ok(())
}

#[test]
fn is_allowed() -> Result<(), ContractError> {
    let mut session = TestSession::new();

    // test address_0 and address_1 are allowed
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_0)?.data,
        true
    );
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_1)?.data,
        true
    );

    // test unregistered user address isn't allowed
    let unregistered: [u8; 32] = rand::random();
    let unregistered = Address::from(&unregistered);
    assert_eq!(session.allowlist_is_allowed(&unregistered)?.data, false);

    Ok(())
}

#[test]
fn has_role() -> Result<(), ContractError> {
    let mut session = TestSession::new();

    // assert that the registered user roles are correct
    assert_eq!(
        session
            .allowlist_has_role(&*TestSession::ADDRESS_0)
            .expect("call should be successfull")
            .data,
        Some(*TestSession::ROLE_0),
    );
    assert_eq!(
        session
            .allowlist_has_role(&*TestSession::ADDRESS_1)
            .expect("call should be successfull")
            .data,
        Some(*TestSession::ROLE_1),
    );

    // assert that unregistered user has no role
    let unregistered: [u8; 32] = rand::random();
    let unregistered = Address::from(&unregistered);
    assert_eq!(session.allowlist_has_role(&unregistered)?.data, None);

    Ok(())
}

#[test]
fn register() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // create new unregistered user address
    let new_user: [u8; 32] = rand::random();
    let new_user = Address::from(&new_user);
    assert_eq!(session.allowlist_is_allowed(&new_user)?.data, false);

    // register user
    let role: [u8; 32] = rand::random();
    let role = Role::from(&role);
    session.allowlist_register(ownership_sk, new_user, role)?;

    // make sure user is allowed after registration
    assert_eq!(session.allowlist_is_allowed(&new_user)?.data, true);
    assert_eq!(session.allowlist_has_role(&new_user)?.data, Some(role));

    Ok(())
}

#[test]
fn register_wrong_ownership_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let wrong_ownership_sk = &*TestSession::SK_1;

    // create new unregistered user address
    let new_user: [u8; 32] = rand::random();
    let new_user = Address::from(&new_user);
    assert_eq!(session.allowlist_is_allowed(&new_user)?.data, false);

    // attempting to register user from an account not registered as ownership
    let role: [u8; 32] = rand::random();
    let role = Role::from(&role);
    let contract_err = session
        .allowlist_register(wrong_ownership_sk, new_user, role)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // make sure user is not allowed after registration
    assert_eq!(session.allowlist_is_allowed(&new_user)?.data, false);
    assert_eq!(session.allowlist_has_role(&new_user)?.data, None);

    Ok(())
}

#[test]
fn register_existing_user_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // attempt to register a user address that is already registered
    let contract_err = session
        .allowlist_register(
            ownership_sk,
            *TestSession::ADDRESS_1,
            *TestSession::ROLE_0,
        )
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::DUPLICATE_USER);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }

    // make sure user is still allowed and has correct role
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_1)?.data,
        true
    );
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_1)?.data,
        Some(*TestSession::ROLE_1)
    );

    Ok(())
}

#[test]
fn update() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // assign role_1 to address_0
    session.allowlist_update(
        ownership_sk,
        *TestSession::ADDRESS_0,
        *TestSession::ROLE_1,
    )?;
    // check that update was successful
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_0)?.data,
        Some(*TestSession::ROLE_1)
    );

    // check that role the other address didn't change
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_1)?.data,
        Some(*TestSession::ROLE_1)
    );

    Ok(())
}

#[test]
fn update_wrong_ownership_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let wrong_ownership_sk = &*TestSession::SK_1;

    // attempt to assign role_1 to address_0 using an incorrect ownership
    // account
    let contract_err = session
        .allowlist_update(
            wrong_ownership_sk,
            *TestSession::ADDRESS_0,
            *TestSession::ROLE_1,
        )
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that role didn't update
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_0)?.data,
        Some(*TestSession::ROLE_0)
    );

    Ok(())
}

#[test]
fn update_unregistered_user_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // create new unregistered user address
    let unregistered: [u8; 32] = rand::random();
    let unregistered = Address::from(&unregistered);
    assert_eq!(session.allowlist_is_allowed(&unregistered)?.data, false);

    // attempt to update unregistered user
    let contract_err = session
        .allowlist_update(ownership_sk, unregistered, *TestSession::ROLE_0)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::ADDRESS_NOT_FOUND);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that update failed
    assert_eq!(session.allowlist_has_role(&unregistered)?.data, None);

    // check that role the other address didn't change
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_0)?.data,
        Some(*TestSession::ROLE_0)
    );
    assert_eq!(
        session.allowlist_has_role(&*TestSession::ADDRESS_1)?.data,
        Some(*TestSession::ROLE_1)
    );

    Ok(())
}

#[test]
fn remove() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // remove address_0
    session.allowlist_remove(ownership_sk, &*TestSession::ADDRESS_0)?;
    // check that removal was successful
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_0)?.data,
        false
    );
    // check that address_1 is still allowed
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_1)?.data,
        true
    );

    // remove address_1
    session.allowlist_remove(ownership_sk, &*TestSession::ADDRESS_1)?;
    // check that removal was successful
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_1)?.data,
        false
    );

    Ok(())
}

#[test]
fn remove_wrong_ownership_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let wrong_ownership_sk = &*TestSession::SK_1;

    // attempt to assign role_1 to address_0 using an incorrect ownership
    // account
    let contract_err = session
        .allowlist_remove(wrong_ownership_sk, &*TestSession::ADDRESS_0)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that user wasn't removed
    assert_eq!(
        session.allowlist_is_allowed(&*TestSession::ADDRESS_0)?.data,
        true
    );

    Ok(())
}

#[test]
fn remove_unregistered_user_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let ownership_sk = &*TestSession::SK_0;

    // create new unregistered user address
    let unregistered: [u8; 32] = rand::random();
    let unregistered = Address::from(&unregistered);
    assert_eq!(session.allowlist_is_allowed(&unregistered)?.data, false);

    // attempt remove an unregistered user address
    let contract_err = session
        .allowlist_remove(ownership_sk, &unregistered)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::ADDRESS_NOT_FOUND);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that user is still unregistered
    assert_eq!(session.allowlist_is_allowed(&unregistered)?.data, false);

    Ok(())
}

#[test]
fn transfer_ownership() -> Result<(), ContractError> {
    let mut session = TestSession::new();

    // assert that current ownership account is pk_0
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    // create new ownership account
    let new_ownership = Account::from(*TestSession::PK_1);

    // change the ownership on the allowlist-contract
    session
        .allowlist_transfer_ownership(&*TestSession::SK_0, &new_ownership)?;

    // check that allowlist ownership changed
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        new_ownership
    );

    Ok(())
}

#[test]
fn transfer_ownership_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let wrong_ownership_sk = &*TestSession::SK_1;

    // assert that current ownership account is pk_0
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    // create new ownership account
    let new_ownership = Account::from(*TestSession::PK_1);

    // attempt to transfer ownership, using the incorrect ownership account
    let contract_err = session
        .allowlist_transfer_ownership(wrong_ownership_sk, &new_ownership)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that allowlist ownership didn't change
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    Ok(())
}

#[test]
fn renounce_ownership() -> Result<(), ContractError> {
    let mut session = TestSession::new();

    // assert that current ownership account is pk_0
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    // change the ownership on the allowlist-contract
    session.allowlist_renounce_ownership(&*TestSession::SK_0)?;

    // assert that ownership is set to zero-address
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        ZERO_ADDRESS,
    );

    Ok(())
}

#[test]
fn renounce_ownership_panics() -> Result<(), ContractError> {
    let mut session = TestSession::new();
    let wrong_ownership_sk = &*TestSession::SK_1;

    // assert that current ownership account is pk_0
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    // attempt to transfer ownership, using the incorrect ownership account
    let contract_err = session
        .allowlist_renounce_ownership(wrong_ownership_sk)
        .expect_err("Expect contract to panic");

    // check contract panic
    if let ContractError::Panic(panic_msg) = contract_err {
        assert_eq!(panic_msg, error::UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected panic, got error: {contract_err}",);
    }
    // check that allowlist ownership didn't change
    assert_eq!(
        session
            .allowlist_ownership()
            .expect("call should pass")
            .data,
        Account::from(*TestSession::PK_0),
    );

    Ok(())
}
