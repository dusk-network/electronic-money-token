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

use emt_core::admin_management::PAUSED_MESSAGE;
use emt_core::governance::UNAUTHORIZED_ACCOUNT;
use emt_core::sanctions::{BLOCKED, FROZEN};
use emt_core::supply_management::SUPPLY_OVERFLOW;
use emt_core::*;

pub mod instantiate;
use instantiate::{
    TestSession, HOLDER_ID, INITIAL_BALANCE, INITIAL_HOLDER_BALANCE,
    INITIAL_SUPPLY,
};

#[test]
fn deploy() {
    TestSession::new();
}

#[test]
fn empty_account() {
    let mut session = TestSession::new();

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

// Test that the token contract can not be initialized when it already carries
/// data.
#[test]
fn double_init() {
    const INSERT_VALUE: u64 = INITIAL_BALANCE + 42;

    let mut session = TestSession::new();

    // generate new keys to insert with init functions
    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = AccountSecretKey::random(&mut rng);
    let pk = AccountPublicKey::from(&sk);
    session
        .call_token::<(Vec<(Account, u64)>, Account), ()>(
            &*TestSession::SK_0,
            "init",
            &(
                vec![(Account::External(pk), INSERT_VALUE)],
                Account::External(pk),
            ),
        )
        .expect_err("Call should not pass");

    assert_eq!(
        session.account(pk).balance,
        0,
        "The new account should have 0 balance"
    );

    assert_ne!(
        session.governance(),
        Account::External(pk),
        "The token-contract owner shouldn't have changed"
    );
}

/// Test a token transfer from the deploy account to the test account.
#[test]
fn transfer() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = TestSession::new();

    let receiver_account = Account::from(*TestSession::PK_2);

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(receiver_account).balance,
        0,
        "The account to transfer to should have no balance"
    );

    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(receiver_account, TRANSFERRED_AMOUNT),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(receiver_account).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
}

/// Test a token transfer from the deploy account to the test contract account.
#[test]
fn transfer_to_contract() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = TestSession::new();

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let contract_account = Account::from(HOLDER_ID);

    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(contract_account, TRANSFERRED_AMOUNT),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
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

    let mut session = TestSession::new();
    let account_1 = Account::from(*TestSession::PK_1);

    assert_eq!(
        session.account(account_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let receipt = session
        .call_holder::<_, ()>(
            &*TestSession::SK_1,
            "token_send",
            &(account_1, TRANSFERRED_AMOUNT),
        )
        .expect("Call should pass");

    receipt.events.iter().for_each(|event| {
        if event.topic == "moonlight" {
            let transfer_info =
                rkyv::from_bytes::<MoonlightTransactionEvent>(&event.data)
                    .unwrap();

            assert!(
                transfer_info.sender == *TestSession::PK_1,
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
                transfer_event.receiver == (account_1).into(),
                "The receiver should be the deploy account"
            );
            assert_eq!(
                transfer_event.value, TRANSFERRED_AMOUNT,
                "The transferred amount should be the same"
            );
        }
    });

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
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

    let mut session = TestSession::new();

    let test_account = Account::from(*TestSession::PK_2);

    assert_eq!(
        session.allowance(*TestSession::PK_1, test_account),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "approve",
            &(test_account, APPROVED_AMOUNT),
        )
        .expect("Call should pass");

    assert_eq!(
        session.allowance(*TestSession::PK_1, test_account),
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

    let mut session = TestSession::new();
    let spender_account = Account::from(*TestSession::PK_2);
    let owner_account = Account::from(*TestSession::PK_1);

    assert_eq!(
        session.account(owner_account).balance,
        INITIAL_BALANCE,
        "The owner account should have the initial balance"
    );
    assert_eq!(
        session.account(spender_account).balance,
        0,
        "The account to transfer to should have no balance"
    );
    assert_eq!(
        session.allowance(owner_account, spender_account),
        0,
        "The spender account should not be allowed to spend tokens from the owner account"
    );

    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "approve",
            &(spender_account, APPROVED_AMOUNT),
        )
        .expect("Call should pass");

    assert_eq!(
        session.allowance(owner_account, spender_account),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );

    session
        .call_token::<_, ()>(
            &*TestSession::SK_2,
            "transfer_from",
            &(owner_account, spender_account, TRANSFERRED_AMOUNT),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(spender_account).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
    assert_eq!(
        session.allowance(*TestSession::PK_1, spender_account),
        APPROVED_AMOUNT - TRANSFERRED_AMOUNT,
        "The account should have the transferred amount subtracted from its allowance"
    );
}

/// Test transfer of governance to test account.
#[test]
fn transfer_governance() {
    let mut session = TestSession::new();
    let new_governance = Account::from(*TestSession::PK_2);

    session
        .call_token::<_, ()>(
            &*TestSession::SK_0,
            "transfer_governance",
            &new_governance,
        )
        .expect("Call should pass");

    assert_eq!(session.governance(), new_governance);
}

/// Test TransferGovernance, RenounceGovernance with wrong governance
/// and check for correct error message.
///
/// TODO: Squash wrong sk case with transfer governance & renounce governance
/// tests functions as the other tests (mint, burn etc) do it.
#[test]
fn governance_fails() {
    let mut session = TestSession::new();

    let wrong_governance_sk = &*TestSession::SK_2;
    let new_governance = Account::from(*TestSession::PK_2);

    let receipt = session.call_token::<_, ()>(
        &wrong_governance_sk,
        "transfer_governance",
        &new_governance,
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }

    let receipt = session.call_token::<_, ()>(
        wrong_governance_sk,
        "renounce_governance",
        &(),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }

    assert_eq!(session.governance(), Account::from(*TestSession::PK_0));
}

/// Test renounce governance.
#[test]
fn renounce_governance() {
    let mut session = TestSession::new();

    session
        .call_token::<_, ()>(&*TestSession::SK_0, "renounce_governance", &())
        .expect("Call should pass");

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
    let mut session = TestSession::new();
    let mint_amount = 1000;

    // Note: Direct usage of PublicKey here fails during rkyv deserialization.
    // TODO: Consider changing call_token to support types implementing
    // Into<Account> by somehow detecting the types the fn expects.
    let mint_receiver = Account::from(*TestSession::PK_0);

    assert_eq!(session.total_supply(), INITIAL_SUPPLY);

    // mint with governance sk
    let receipt = session
        .call_token::<_, ()>(
            &*TestSession::SK_0,
            "mint",
            &(mint_receiver, mint_amount),
        )
        .expect("Call should pass");

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

    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "mint",
        &(mint_receiver, too_much),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, SUPPLY_OVERFLOW);
    } else {
        panic!("Expected a panic error");
    }

    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "mint",
        &(mint_receiver, mint_amount),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }
}

/// Test burn with governance sk
/// Test burn with wrong sk
/// Test burn with balance too low / underflow
#[test]
fn test_burn() {
    let mut session = TestSession::new();
    let burn_amount = 1000;

    session
        .call_token::<_, ()>(&*TestSession::SK_0, "burn", &burn_amount)
        .expect("Call should pass");

    assert_eq!(session.total_supply(), INITIAL_SUPPLY - burn_amount);

    // burn more than the governance account has
    let burn_amount = u64::MAX;

    let receipt =
        session.call_token::<_, ()>(&*TestSession::SK_0, "burn", &burn_amount);

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, BALANCE_TOO_LOW);
    } else {
        panic!("Expected a panic error");
    }

    // unauthorized account
    let receipt =
        session.call_token::<_, ()>(&*TestSession::SK_2, "burn", &burn_amount);

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
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

    let mut session = TestSession::new();
    let account_2 = Account::from(*TestSession::PK_2);

    session
        .call_token::<_, ()>(&*TestSession::SK_0, "toggle_pause", &())
        .expect("Call should pass");

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("call to pass")
            .data,
        true
    );

    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_1,
        "transfer",
        &(account_2, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, PAUSED_MESSAGE);
    } else {
        panic!("Expected a panic error");
    }

    assert_eq!(
        session.account(*TestSession::PK_1).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(account_2).balance,
        0,
        "The account to transfer to should have no balance"
    );

    session
        .call_token::<_, ()>(&*TestSession::SK_0, "toggle_pause", &())
        .expect("Call should pass");

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("call to pass")
            .data,
        false
    );

    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(account_2, VALUE),
        )
        .expect("Call should pass");

    // unauthorized account
    let receipt =
        session.call_token::<_, ()>(&*TestSession::SK_2, "toggle_pause", &());

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }
}

/// Test force transfer
/// Test force transfer with balance too low
/// Test force transfer with wrong sk
/// TODO: test force transfer circumventing pause, sanction, etc.
#[test]
fn test_force_transfer() {
    const VALUE: u64 = INITIAL_BALANCE - 1;
    let mut session = TestSession::new();
    let account_1 = Account::from(*TestSession::PK_1);
    let account_2 = Account::from(*TestSession::PK_2);
    let governance_account = Account::from(*TestSession::PK_0);

    // Make a normal transfer from deploy account to the test account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(account_2, VALUE),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(account_1).balance,
        INITIAL_BALANCE - VALUE,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(account_2).balance,
        VALUE,
        "The test account should have the transferred amount"
    );

    // Force transfer from test account to governance account
    let obliged_sender = account_2;
    session
        .call_token::<_, ()>(
            &*TestSession::SK_0,
            "force_transfer",
            &(obliged_sender, governance_account, VALUE),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(account_2).balance,
        0,
        "The test account should have the transferred amount subtracted"
    );

    assert_eq!(
        session.account(governance_account).balance,
        INITIAL_BALANCE + VALUE,
        "The governance account should have the transferred amount added"
    );

    // Force transfer from test account to governance account again (balance
    // will be too low)
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "force_transfer",
        &(obliged_sender, governance_account, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, BALANCE_TOO_LOW);
    } else {
        panic!("Expected a panic error");
    }

    // unauthorized account
    let obliged_sender = Account::from(governance_account);
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "force_transfer",
        &(obliged_sender, account_2, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
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
    let mut session = TestSession::new();
    let account_1 = Account::from(*TestSession::PK_1);
    let blocked_account = Account::from(*TestSession::PK_2);

    // Transfer VALUE to test account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(blocked_account, VALUE),
        )
        .expect("Call should pass")
        .data;

    // Block test account
    session
        .call_token::<_, ()>(&*TestSession::SK_0, "block", &blocked_account)
        .expect("Call should pass");

    assert_eq!(
        session
            .call_token::<_, bool>(
                &*TestSession::SK_2,
                "blocked",
                &blocked_account
            )
            .expect("Querying the state should succeed")
            .data,
        true
    );

    // Unfreeze blocked test account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "unfreeze",
        &blocked_account,
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, "The account is not frozen");
    } else {
        panic!("Expected a panic error");
    }

    // Transfer VALUE to test account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_1,
        "transfer",
        &(blocked_account, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, BLOCKED);
    } else {
        panic!("Expected a panic error");
    }

    // Transfer VALUE from test account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "transfer",
        &(account_1, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, BLOCKED);
    } else {
        panic!("Expected a panic error");
    }

    // Freeze test account
    let frozen_account = blocked_account;
    session
        .call_token::<_, ()>(&*TestSession::SK_0, "freeze", &frozen_account)
        .expect("Call should pass");

    assert_eq!(
        session
            .call_token::<_, bool>(
                &*TestSession::SK_2,
                "frozen",
                &frozen_account
            )
            .expect("Querying the state should succeed")
            .data,
        true
    );

    // Transfer VALUE to test account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(frozen_account, VALUE),
        )
        .expect("Transfer to frozen account should succeed");

    // Transfer VALUE from test account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "transfer",
        &(account_1, VALUE),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, FROZEN);
    } else {
        panic!("Expected a panic error");
    }

    // Unblock frozen test account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "unblock",
        &frozen_account,
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, "The account is not blocked");
    } else {
        panic!("Expected a panic error");
    }

    // Unauthorized account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "unblock",
        &frozen_account,
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }

    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_2,
        "unfreeze",
        &frozen_account,
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
    } else {
        panic!("Expected a panic error");
    }

    // Unfreeze test account
    session
        .call_token::<_, ()>(&*TestSession::SK_0, "unfreeze", &frozen_account)
        .expect("Unfreezing should succeed");

    // Transfer VALUE from test account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_2,
            "transfer",
            &(account_1, VALUE),
        )
        .expect("Transfer should succeed again");
}

#[test]
fn test_partial_freeze() {
    const VALUE: u64 = INITIAL_BALANCE / 2;
    let mut session = TestSession::new();
    let frozen_tokens_acc = Account::from(*TestSession::PK_1);
    let account_2 = Account::from(*TestSession::PK_2);

    // freeze test account tokens
    session
        .call_token::<_, ()>(
            &*TestSession::SK_0,
            "freeze_tokens",
            &(frozen_tokens_acc, 500),
        )
        .expect("Call should pass");

    assert_eq!(
        session
            .call_token::<_, u64>(
                &*TestSession::SK_0,
                "frozen_tokens",
                &frozen_tokens_acc
            )
            .expect("Querying the state should succeed")
            .data,
        500,
    );

    // Transfer VALUE from frozen tokens account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(account_2, VALUE),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(frozen_tokens_acc).balance,
        VALUE,
        "The account transferred to should have the transferred amount"
    );

    // Transfer 1 token from frozen tokens account
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_1,
        "transfer",
        &(account_2, 1),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, BALANCE_TOO_LOW);
    } else {
        panic!("Expected a panic error");
    }

    // unfreeze test account tokens
    session
        .call_token::<_, ()>(
            &*TestSession::SK_0,
            "unfreeze_tokens",
            &(frozen_tokens_acc, 500),
        )
        .expect("Call should pass");

    assert_eq!(
        session
            .call_token::<_, u64>(
                &*TestSession::SK_0,
                "frozen_tokens",
                &frozen_tokens_acc
            )
            .expect("Querying the state should succeed")
            .data,
        0,
    );

    // Transfer 1 token from frozen tokens account
    session
        .call_token::<_, ()>(
            &*TestSession::SK_1,
            "transfer",
            &(account_2, VALUE),
        )
        .expect("Call should pass");

    assert_eq!(
        session.account(frozen_tokens_acc).balance,
        0,
        "The account transferred to should have the transferred amount"
    );

    assert_eq!(
        session.account(account_2).balance,
        INITIAL_BALANCE,
        "The account transferred to should have the transferred amount"
    );

    // freeze test account tokens
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "freeze_tokens",
        &(frozen_tokens_acc, 1),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, "The account has not enough balance");
    } else {
        panic!("Expected a panic error");
    }

    // unfreeze test account tokens
    let receipt = session.call_token::<_, ()>(
        &*TestSession::SK_0,
        "unfreeze_tokens",
        &(frozen_tokens_acc, 1),
    );

    if let ContractError::Panic(panic_msg) = receipt.unwrap_err() {
        assert_eq!(panic_msg, "The account has not enough frozen tokens");
    } else {
        panic!("Expected a panic error");
    }
}
