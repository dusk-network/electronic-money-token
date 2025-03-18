// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::ContractError;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::MoonlightTransactionEvent;

use rand::rngs::StdRng;
use rand::SeedableRng;

use ttoken_types::admin_management::PAUSED_MESSAGE;
use ttoken_types::governance::arguments::TransferGovernance;
use ttoken_types::governance::UNAUTHORIZED_ACCOUNT;
use ttoken_types::sanctions::arguments::Sanction;
use ttoken_types::sanctions::{BLOCKED, FROZEN};
use ttoken_types::supply_management::SUPPLY_OVERFLOW;
use ttoken_types::*;

pub mod instantiate;
use instantiate::{
    ContractSession, HOLDER_ID, INITIAL_BALANCE, INITIAL_HOLDER_BALANCE,
    INITIAL_SUPPLY,
};

#[test]
fn deploy() {
    ContractSession::new();
}

#[test]
fn empty_account() {
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = AccountSecretKey::random(&mut rng);
    let pk = AccountPublicKey::from(&sk);

    let account = session.account(pk);
    assert_eq!(
        account,
        AccountInfo::EMPTY,
        "An account never transferred to should be empty"
    );
}

/// Test a token transfer from the deploy account to the test account.
#[test]
fn transfer() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let receiver_pk = *ContractSession::TEST_PK_2;

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(receiver_pk).balance,
        0,
        "The account to transfer to should have no balance"
    );

    let transfer = Transfer::new(receiver_pk, TRANSFERRED_AMOUNT);

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(receiver_pk).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
}

/// Test a token transfer from the deploy account to the test contract account.
#[test]
fn transfer_to_contract() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let transfer = Transfer::new(HOLDER_ID, TRANSFERRED_AMOUNT);

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );

    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE + TRANSFERRED_AMOUNT,
        "The contract transferred to should have the transferred amount added"
    );
}

/// Test a token transfer from the HOLDER_ID contract account to the deploy
/// account.
#[test]
fn transfer_from_contract() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let transfer =
        Transfer::new(*ContractSession::TEST_PK_1, TRANSFERRED_AMOUNT);

    let receipt = session.call_holder::<_>(
        &*ContractSession::TEST_SK_1,
        "token_send",
        &transfer,
    );

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    receipt.events.iter().for_each(|event| {
        if event.topic == "moonlight" {
            let transfer_info =
                rkyv::from_bytes::<MoonlightTransactionEvent>(&event.data)
                    .unwrap();

            assert!(
                transfer_info.sender == *ContractSession::TEST_PK_1,
                "The tx origin should be the deploy pk"
            )
        } else if event.topic == "transfer" {
            let transfer_event =
                rkyv::from_bytes::<TransferEvent>(&event.data).unwrap();

            assert!(
                transfer_event.sender == HOLDER_ID.into(),
                "The sender should be the contract"
            );
            assert!(
                transfer_event.receiver == (*ContractSession::TEST_PK_1).into(),
                "The receiver should be the deploy account"
            );
            assert_eq!(
                transfer_event.value, TRANSFERRED_AMOUNT,
                "The transferred amount should be the same"
            );
        }
    });

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE + TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount added"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE - TRANSFERRED_AMOUNT,
        "The contract transferred to should have the transferred amount subtracted"
    );
}

/// Test approval of deploy account to test account.
#[test]
fn approve() {
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let test_account = Account::from(*ContractSession::TEST_PK_2);

    assert_eq!(
        session.allowance(*ContractSession::TEST_PK_1, test_account),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new(test_account, APPROVED_AMOUNT);
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "approve", &approve);

    if let Err(e) = receipt.data {
        panic!("Approve should succeed, err: {e}");
    }

    assert_eq!(
        session.allowance(*ContractSession::TEST_PK_1, test_account),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );
}

/// Test approve from deploy account to test account and
/// transfer from deploy account to test account
/// where sender is deploy account, spender is test account, receiver is test
/// account
#[test]
fn transfer_from() {
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE - 1;
    const TRANSFERRED_AMOUNT: u64 = APPROVED_AMOUNT / 2;

    let mut session = ContractSession::new();
    let spender_account = Account::from(*ContractSession::TEST_PK_2);

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(spender_account).balance,
        0,
        "The account to transfer to should have no balance"
    );
    assert_eq!(
        session.allowance(*ContractSession::TEST_PK_1, spender_account),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new(spender_account, APPROVED_AMOUNT);

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "approve", &approve);
    receipt.data.expect("Approve should succeed");

    assert_eq!(
        session.allowance(*ContractSession::TEST_PK_1, spender_account),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );

    let transfer_from = TransferFrom::new(
        *ContractSession::TEST_PK_1,
        spender_account,
        TRANSFERRED_AMOUNT,
    );
    let receipt = session.call_token(
        &*ContractSession::TEST_SK_2,
        "transfer_from",
        &transfer_from,
    );

    if let Err(e) = receipt.data {
        panic!("Transfer from should succeed, err: {e}");
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(spender_account).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
    assert_eq!(
        session.allowance(*ContractSession::TEST_PK_1, spender_account),
        APPROVED_AMOUNT - TRANSFERRED_AMOUNT,
        "The account should have the transferred amount subtracted from its allowance"
    );
}

/// Test transfer of governance to test account.
#[test]
fn transfer_governance() {
    let mut session = ContractSession::new();
    let new_governance = Account::from(*ContractSession::TEST_PK_2);

    let transfer_governance = TransferGovernance::new(new_governance);
    let receipt = session.call_token(
        &*ContractSession::TEST_SK_0,
        "transfer_governance",
        &transfer_governance,
    );

    if let Err(e) = receipt.data {
        panic!("Transfer governance should succeed, err: {e}");
    }

    assert_eq!(session.governance(), new_governance);
}

/// Test TransferGovernance, RenounceGovernance with wrong governance
/// and check for correct error message.
///
/// TODO: Squash wrong sk case with transfer governance & renounce governance
/// tests functions as the other tests (mint, burn etc) do it.
#[test]
fn governance_fails() {
    let mut session = ContractSession::new();

    let wrong_governance_sk = &*ContractSession::TEST_SK_2;
    let new_governance = Account::from(*ContractSession::TEST_PK_2);

    let transfer_governance = TransferGovernance::new(new_governance);
    let receipt = session.call_token(
        &wrong_governance_sk,
        "transfer_governance",
        &transfer_governance,
    );

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    let receipt =
        session.call_token(wrong_governance_sk, "renounce_governance", &());

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    assert_eq!(
        session.governance(),
        Account::from(*ContractSession::TEST_PK_0)
    );
}

/// Test renounce governance.
#[test]
fn renounce_governance() {
    let mut session = ContractSession::new();

    let receipt = session.call_token(
        &*ContractSession::TEST_SK_0,
        "renounce_governance",
        &(),
    );

    if let Err(e) = receipt.data {
        panic!("Renounce governance should succeed, err: {e}");
    }

    assert_eq!(
        session.governance(),
        // TODO: consider defining this as ZERO_ADDRESS in core?
        Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]))
    );
}

/// Test mint with governance sk
/// Test mint with wrong sk
/// Test mint with overflow
#[test]
fn test_mint() {
    let mut session = ContractSession::new();
    let mint_amount = 1000;

    // Note: Direct usage of PublicKey here fails during rkyv deserialization.
    // TODO: Consider changing call_token to support types implementing
    // Into<Account> by somehow detecting the types the fn expects.
    let mint_receiver = Account::from(*ContractSession::TEST_PK_0);

    assert_eq!(session.total_supply(), INITIAL_SUPPLY);

    // mint with governance sk
    let receipt = session.call_token(
        &*ContractSession::TEST_SK_0,
        "mint",
        &(mint_receiver, mint_amount),
    );

    if let Err(e) = receipt.data {
        panic!("Mint should succeed, err: {e}");
    }

    assert_eq!(receipt.events.len(), 2);

    receipt.events.iter().any(|event| {
        if event.topic == "mint" {
            let transfer_event =
                rkyv::from_bytes::<TransferEvent>(&event.data).unwrap();

            assert!(
                transfer_event.sender == ZERO_ADDRESS,
                "The sender should be the ZERO_ADDRESS"
            );
            assert!(
                transfer_event.receiver == mint_receiver,
                "The receiver should be the mint_receiver"
            );
            assert_eq!(
                transfer_event.value, mint_amount,
                "The transferred amount should be the mint_amount"
            );
            true
        } else {
            false
        }
    });

    assert_eq!(session.total_supply(), INITIAL_SUPPLY + mint_amount);

    // mint overflow
    let too_much = u64::MAX;

    let receipt = session.call_token(
        &*ContractSession::TEST_SK_0,
        "mint",
        &(mint_receiver, too_much),
    );

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, SUPPLY_OVERFLOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    let receipt = session.call_token(
        &*ContractSession::TEST_SK_2,
        "mint",
        &(mint_receiver, mint_amount),
    );

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

/// Test burn with governance sk
/// Test burn with wrong sk
/// Test burn with balance too low / underflow
#[test]
fn test_burn() {
    let mut session = ContractSession::new();
    let burn_amount = 1000;

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "burn", &burn_amount);

    if let Err(e) = receipt.data {
        panic!("Burn should succeed, err: {e}");
    }

    assert_eq!(session.total_supply(), INITIAL_SUPPLY - burn_amount);

    // burn more than the governance account has
    let burn_amount = u64::MAX;

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "burn", &burn_amount);

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BALANCE_TOO_LOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // unauthorized account
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_2, "burn", &burn_amount);

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

/// Test pause
/// Test transfer from deploy_sk while paused
/// Test unpause
/// Test transfer from deploy_sk after unpausing
/// Test pause with wrong sk
#[test]
fn test_pause() {
    const VALUE: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "toggle_pause", &());

    if let Err(e) = receipt.data {
        panic!("Pause should succeed, err: {e}");
    }

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("Querying the pause state should succeed")
            .data,
        true
    );

    let transfer = Transfer::new(*ContractSession::TEST_PK_2, VALUE);
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer);

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, PAUSED_MESSAGE);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(*ContractSession::TEST_PK_2).balance,
        0,
        "The account to transfer to should have no balance"
    );

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "toggle_pause", &());

    if let Err(e) = receipt.data {
        panic!("Unpause should succeed, err: {e}");
    }

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("Querying the pause state should succeed")
            .data,
        false
    );

    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed again, err: {e}");
    }

    // unauthorized account
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_2, "toggle_pause", &());

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

/// Test force transfer
/// Test force transfer with balance too low
/// Test force transfer with wrong sk
/// TODO: test force transfer circumventing pause, sanction, etc.
#[test]
fn test_force_transfer() {
    const VALUE: u64 = INITIAL_BALANCE - 1;
    let mut session = ContractSession::new();

    // Make a normal transfer from deploy account to the test account
    let transfer = Transfer::new(*ContractSession::TEST_PK_2, VALUE);
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_1).balance,
        INITIAL_BALANCE - VALUE,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(*ContractSession::TEST_PK_2).balance,
        VALUE,
        "The test account should have the transferred amount"
    );

    // Force transfer from test account to governance account
    let force_transfer = Transfer::new(*ContractSession::TEST_PK_0, VALUE);
    let obliged_sender = Account::from(*ContractSession::TEST_PK_2);
    let receipt = session.call_token(
        &*ContractSession::TEST_SK_0,
        "force_transfer",
        &(force_transfer, obliged_sender),
    );

    if let Err(e) = receipt.data {
        panic!("Force transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(*ContractSession::TEST_PK_2).balance,
        0,
        "The test account should have the transferred amount subtracted"
    );

    assert_eq!(
        session.account(*ContractSession::TEST_PK_0).balance,
        INITIAL_BALANCE + VALUE,
        "The governance account should have the transferred amount added"
    );

    // Force transfer from test account to governance account again (balance
    // will be too low)
    let force_transfer = Transfer::new(*ContractSession::TEST_PK_0, VALUE);

    match session
        .call_token(
            &*ContractSession::TEST_SK_0,
            "force_transfer",
            &(force_transfer, obliged_sender),
        )
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BALANCE_TOO_LOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // unauthorized account
    let force_transfer = Transfer::new(*ContractSession::TEST_PK_2, VALUE);
    let obliged_sender = Account::from(*ContractSession::TEST_PK_0);
    let receipt = session.call_token(
        &*ContractSession::TEST_SK_2,
        "force_transfer",
        &(force_transfer, obliged_sender),
    );

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

/// Test block account
/// Test unfreezing blocked account (fail)
/// Test transfer to blocked account (fail)
/// Test transfer from blocked account (fail)
/// Test freezing blocked account (overriding is allowed)
/// Test transfer to frozen account (allow)
/// Test unblocking frozen account (fail)
/// Test wrong sk for unblock & unfreeze (fail)
/// Test unfreezing frozen account
/// Test transfer after unfrozen
#[test]
fn test_sanctions() {
    // TODO: unify transfer logic in the contract so that this implicitly checks
    // the invariants of transferFrom and any other potential function
    // leading to a "transfer" that updates the balance

    const VALUE: u64 = INITIAL_BALANCE / 3;
    let mut session = ContractSession::new();
    let blocked_account = Account::from(*ContractSession::TEST_PK_2);

    // Transfer VALUE to test account
    let transfer = Transfer::new(blocked_account, VALUE);
    session
        .call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer)
        .data
        .expect("Transfer should succeed");

    // Block test account
    let sanction = Sanction::block_account(blocked_account);
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "block", &sanction);

    if let Err(e) = receipt.data {
        panic!("Block should succeed, err: {e}");
    }

    assert_eq!(
        rkyv::from_bytes::<bool>(
            &session
                .call_token(
                    &*ContractSession::TEST_SK_2,
                    "blocked",
                    &blocked_account
                )
                .data
                .expect("Querying the state should succeed")
        )
        .expect("Deserializing the state should succeed"),
        true
    );

    // Unfreeze test account
    let unsanction = Sanction::unsanction_account(blocked_account);
    match session
        .call_token(&*ContractSession::TEST_SK_0, "unfreeze", &unsanction)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, "The account is not frozen");
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Transfer VALUE to test account
    let transfer = Transfer::new(blocked_account, VALUE);
    match session
        .call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BLOCKED);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Transfer VALUE from test account
    let transfer = Transfer::new(*ContractSession::TEST_PK_1, VALUE);
    match session
        .call_token(&*ContractSession::TEST_SK_2, "transfer", &transfer)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BLOCKED);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Freeze test account
    let frozen_account = blocked_account;
    let sanction = Sanction::freeze_account(frozen_account);
    let receipt =
        session.call_token(&*ContractSession::TEST_SK_0, "freeze", &sanction);

    if let Err(e) = receipt.data {
        panic!("Freeze should succeed, err: {e}");
    }

    assert_eq!(
        rkyv::from_bytes::<bool>(
            &session
                .call_token(
                    &*ContractSession::TEST_SK_2,
                    "frozen",
                    &frozen_account
                )
                .data
                .expect("Querying the state should succeed")
        )
        .expect("Deserializing the state should succeed"),
        true
    );

    // Transfer VALUE to test account
    let transfer = Transfer::new(frozen_account, VALUE);
    session
        .call_token(&*ContractSession::TEST_SK_1, "transfer", &transfer)
        .data
        .expect("Transfer to frozen account should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new(*ContractSession::TEST_PK_1, VALUE);
    match session
        .call_token(&*ContractSession::TEST_SK_2, "transfer", &transfer)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, FROZEN);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Unblock test account
    let unsanction = Sanction::unsanction_account(frozen_account);
    match session
        .call_token(&*ContractSession::TEST_SK_0, "unblock", &unsanction)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, "The account is not blocked");
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Unauthorized account
    let unsanction = Sanction::unsanction_account(frozen_account);
    match session
        .call_token(&*ContractSession::TEST_SK_2, "unblock", &unsanction)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
    match session
        .call_token(&*ContractSession::TEST_SK_2, "unfreeze", &unsanction)
        .data
        .err()
    {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Unfreeze test account
    let unsanction = Sanction::unsanction_account(frozen_account);
    session
        .call_token(&*ContractSession::TEST_SK_0, "unfreeze", &unsanction)
        .data
        .expect("Unfreezing should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new(*ContractSession::TEST_PK_1, VALUE);
    session
        .call_token(&*ContractSession::TEST_SK_2, "transfer", &transfer)
        .data
        .expect("Transfer should succeed again");
}
