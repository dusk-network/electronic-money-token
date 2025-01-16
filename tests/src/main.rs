// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractId, StandardBufSerializer, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{PublicKey, SecretKey};
use dusk_vm::{CallReceipt, ContractData, Error as VMError, Session, VM};

use bytecheck::CheckBytes;

use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use ttoken_types::admin_management::arguments::PauseToggle;
use ttoken_types::admin_management::PAUSED_MESSAGE;
use ttoken_types::ownership::arguments::{RenounceOwnership, TransferOwnership};
use ttoken_types::ownership::UNAUTHORIZED_EXT_ACCOUNT;
use ttoken_types::sanctions::arguments::Sanction;
use ttoken_types::sanctions::{BLOCKED, FROZEN};
use ttoken_types::supply_management::arguments::{Burn, Mint};
use ttoken_types::supply_management::SUPPLY_OVERFLOW;
use ttoken_types::*;

const TOKEN_BYTECODE: &[u8] = include_bytes!("../../build/ttoken_contract.wasm");
const HOLDER_BYTECODE: &[u8] = include_bytes!("../../build/ttoken_test_contract.wasm");

const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
const HOLDER_ID: ContractId = ContractId::from_bytes([2; 32]);

const INITIAL_BALANCE: u64 = 1000;
const INITIAL_HOLDER_BALANCE: u64 = 1000;
const INITIAL_OWNER_BALANCE: u64 = 1000;
const INITIAL_SUPPLY: u64 = INITIAL_BALANCE + INITIAL_HOLDER_BALANCE + INITIAL_OWNER_BALANCE;

const DEPLOYER: [u8; 64] = [0u8; 64];

type Result<T, Error = VMError> = std::result::Result<T, Error>;

struct ContractSession {
    deploy_pk: PublicKey,
    deploy_sk: SecretKey,
    owner_sk: SecretKey,
    owner: Account,
    session: Session,
}

impl ContractSession {
    fn new() -> Self {
        let vm = VM::ephemeral().expect("Creating VM should succeed");
        let mut session = VM::genesis_session(&vm, 1);

        let mut rng = StdRng::seed_from_u64(0xF0CACC1A);
        let deploy_sk = SecretKey::random(&mut rng);
        let deploy_pk = PublicKey::from(&deploy_sk);

        let deploy_account = Account::External(deploy_pk);
        let holder_account = Account::Contract(HOLDER_ID);

        let owner_sk = SecretKey::random(&mut rng);
        let owner_account = Account::External(PublicKey::from(&owner_sk));

        // never set owner to deploy
        assert_ne!(owner_sk, deploy_sk);

        session
            .deploy(
                TOKEN_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(
                        vec![
                            (deploy_account, INITIAL_BALANCE),
                            (holder_account, INITIAL_HOLDER_BALANCE),
                            (owner_account, INITIAL_OWNER_BALANCE),
                        ],
                        owner_account,
                    ))
                    .contract_id(TOKEN_ID),
                u64::MAX,
            )
            .expect("Deploying the token contract should succeed");

        session
            .deploy(
                HOLDER_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(TOKEN_ID, INITIAL_HOLDER_BALANCE))
                    .contract_id(HOLDER_ID),
                u64::MAX,
            )
            .expect("Deploying the holder contract should succeed");

        Self {
            deploy_sk,
            deploy_pk,
            owner_sk,
            owner: owner_account,
            session,
        }
    }

    fn deploy_pk(&self) -> PublicKey {
        self.deploy_pk
    }

    fn call_token<A, R>(&mut self, fn_name: &str, fn_arg: &A) -> Result<CallReceipt<R>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.call(TOKEN_ID, fn_name, fn_arg, u64::MAX)
    }

    /// Helper function to call a "view" function on the token contract that does not take any arguments.
    fn call_getter<R>(&mut self, fn_name: &str) -> Result<CallReceipt<R>>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        // TODO: find out if there is another way to do that instead of passing &() as fn_arg
        self.session.call::<(), R>(TOKEN_ID, fn_name, &(), u64::MAX)
    }

    fn call_holder<A, R>(&mut self, fn_name: &str, fn_arg: &A) -> Result<CallReceipt<R>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.call(HOLDER_ID, fn_name, fn_arg, u64::MAX)
    }

    fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
        self.call_token("account", &account.into())
            .expect("Querying an account should succeed")
            .data
    }

    fn owner(&mut self) -> Result<CallReceipt<Account>> {
        self.call_getter("owner")
    }

    fn total_supply(&mut self) -> u64 {
        self.call_getter("total_supply")
            .expect("Querying the supply should succeed")
            .data
    }

    fn allowance(&mut self, owner: impl Into<Account>, spender: impl Into<Account>) -> u64 {
        self.call_token(
            "allowance",
            &Allowance {
                owner: owner.into(),
                spender: spender.into(),
            },
        )
        .expect("Querying an allowance should succeed")
        .data
    }
}

#[test]
fn deploy() {
    ContractSession::new();
}

#[test]
fn empty_account() {
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    let account = session.account(pk);
    assert_eq!(
        account,
        AccountInfo::EMPTY,
        "An account never transferred to should be empty"
    );
}

#[test]
fn transfer() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(pk).balance,
        0,
        "The account to transfer to should have no balance"
    );

    let transfer = Transfer::new_external(
        &session.deploy_sk,
        session.deploy_pk,
        pk,
        TRANSFERRED_AMOUNT,
        1,
    );
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transferring should succeed");

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(pk).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
}

#[test]
fn transfer_to_contract() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let transfer = Transfer::new_external(
        &session.deploy_sk,
        session.deploy_pk,
        HOLDER_ID,
        TRANSFERRED_AMOUNT,
        1,
    );
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transferring should succeed");

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE + TRANSFERRED_AMOUNT,
        "The contract transferred to should have the transferred amount added"
    );
}

#[test]
fn transfer_from_contract() {
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE,
        "The contract to transfer to should have its initial balance"
    );

    let sender = Account::Contract(HOLDER_ID);

    let transfer = Transfer::new_contract(
        sender,
        Account::External(session.deploy_pk()),
        TRANSFERRED_AMOUNT,
        0,
    );

    session
        .call_holder::<_, ()>("token_send", &transfer)
        .expect("Transferring should succeed");

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE + TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount added"
    );
    assert_eq!(
        session.account(HOLDER_ID).balance,
        INITIAL_HOLDER_BALANCE - TRANSFERRED_AMOUNT,
        "The contract transferred to should have the transferred amount subtracted"
    );
}

#[test]
fn approve() {
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new_external(&session.deploy_sk, pk, APPROVED_AMOUNT, 1);
    session
        .call_token::<_, ()>("approve", &approve)
        .expect("Approving should succeed");

    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );
}

#[test]
fn transfer_from() {
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE - 1;
    const TRANSFERRED_AMOUNT: u64 = APPROVED_AMOUNT / 2;

    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(pk).balance,
        0,
        "The account to transfer to should have no balance"
    );
    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new_external(&session.deploy_sk, pk, APPROVED_AMOUNT, 1);
    session
        .call_token::<_, ()>("approve", &approve)
        .expect("Approving should succeed");

    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );

    let transfer_from =
        TransferFrom::new_external(&sk, session.deploy_pk(), pk, TRANSFERRED_AMOUNT, 1);
    session
        .call_token::<_, ()>("transfer_from", &transfer_from)
        .expect("Transferring from should succeed");

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(pk).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        APPROVED_AMOUNT - TRANSFERRED_AMOUNT,
        "The account should have the transferred amount subtracted from its allowance"
    );
}

#[test]
fn transfer_ownership() {
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let new_owner_sk = SecretKey::random(&mut rng);
    let new_owner_pk = PublicKey::from(&new_owner_sk);

    let new_owner = Account::External(new_owner_pk);

    let transfer_ownership = TransferOwnership::new(&session.owner_sk, new_owner, 1);
    session
        .call_token::<_, ()>("transfer_ownership", &transfer_ownership)
        .expect("Transferring ownership should succeed");

    assert_eq!(
        session.owner().expect("Querying owner should succeed").data,
        new_owner
    );
}

#[test]
fn ownership_wrong_owner() {
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0x1618);
    let wrong_owner_sk: SecretKey = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&wrong_owner_sk);

    let new_owner = Account::External(pk);

    let transfer_ownership = TransferOwnership::new(&wrong_owner_sk, new_owner, 1);
    let receipt = session.call_token::<_, ()>("transfer_ownership", &transfer_ownership);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_EXT_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    let renounce_ownership = RenounceOwnership::new(&wrong_owner_sk, 1);
    let receipt = session.call_token::<_, ()>("renounce_ownership", &renounce_ownership);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_EXT_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    assert_eq!(
        session.owner().expect("Querying owner should succeed").data,
        session.owner
    );
}

#[test]
fn renounce_ownership() {
    let mut session = ContractSession::new();

    let renounce_ownership = RenounceOwnership::new(&session.owner_sk, 1);
    session
        .call_token::<_, ()>("renounce_ownership", &renounce_ownership)
        .expect("Renouncing ownership should succeed");

    let owner = session.owner().expect("Querying owner should succeed").data;

    assert_eq!(
        owner,
        Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]))
    );
}

#[test]
fn test_mint() {
    let mut session = ContractSession::new();
    let mint_amount = 1000;

    let mint = Mint::new(&session.owner_sk, mint_amount, session.owner, 1);

    session
        .call_token::<_, ()>("mint", &mint)
        .expect("Minting should succeed");

    assert_eq!(session.total_supply(), INITIAL_SUPPLY + mint_amount);

    // mint overflow
    let mint_amount = u64::MAX;

    let mint = Mint::new(&session.owner_sk, mint_amount, session.owner, 2);

    let receipt = session.call_token::<_, ()>("mint", &mint);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, SUPPLY_OVERFLOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // unauthorized pk
    let mut rng = StdRng::seed_from_u64(0x1618);
    let sk = SecretKey::random(&mut rng);

    let mint = Mint::new(&sk, mint_amount, session.owner, 3);
    let receipt = session.call_token::<_, ()>("mint", &mint);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_EXT_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

#[test]
fn test_burn() {
    let mut session = ContractSession::new();
    let burn_amount = 1000;

    let burn = Burn::new(&session.owner_sk, burn_amount, 1);

    session
        .call_token::<_, ()>("burn", &burn)
        .expect("Burning should succeed");

    assert_eq!(session.total_supply(), INITIAL_SUPPLY - burn_amount);

    // burn more than the account has
    let burn_amount = u64::MAX;

    let burn = Burn::new(&session.owner_sk, burn_amount, 2);

    let receipt = session.call_token::<_, ()>("burn", &burn);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BALANCE_TOO_LOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // unauthorized pk
    let mut rng = StdRng::seed_from_u64(0x1618);
    let sk = SecretKey::random(&mut rng);

    let burn = Burn::new(&sk, burn_amount, 3);
    let receipt = session.call_token::<_, ()>("burn", &burn);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_EXT_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

#[test]
fn test_pause() {
    const VALUE: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let pause_toggle = PauseToggle::new(&session.owner_sk, 1);

    session
        .call_token::<_, ()>("toggle_pause", &pause_toggle)
        .expect("Pausing should succeed");

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("Querying the pause state should succeed")
            .data,
        true
    );

    // test transfer
    let mut rng = StdRng::seed_from_u64(0x1618);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(pk).balance,
        0,
        "The account to transfer to should have no balance"
    );

    let transfer = Transfer::new_external(&session.deploy_sk, session.deploy_pk, pk, VALUE, 1);
    let receipt = session.call_token::<_, ()>("transfer", &transfer);

    match receipt.err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, PAUSED_MESSAGE);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    let pause_toggle = PauseToggle::new(&session.owner_sk, 2);

    session
        .call_token::<_, ()>("toggle_pause", &pause_toggle)
        .expect("Unpausing should succeed");

    assert_eq!(
        session
            .call_getter::<bool>("is_paused")
            .expect("Querying the pause state should succeed")
            .data,
        false
    );

    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transferring should now succeed");
}

#[test]
fn test_force_transfer() {
    const VALUE: u64 = INITIAL_BALANCE - 1;
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0x1618);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    let transfer = Transfer::new_external(&session.deploy_sk, session.deploy_pk, pk, VALUE, 1);
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transferring should succeed");

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - VALUE,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(pk).balance,
        VALUE,
        "The account transferred to should have the transferred amount"
    );

    let force_transfer = Transfer::new_external(&session.owner_sk, pk, session.owner, VALUE, 1);

    session
        .call_token::<_, ()>("force_transfer", &force_transfer)
        .expect("Force transferring should succeed");

    let force_transfer = Transfer::new_external(&session.owner_sk, pk, session.owner, VALUE, 2);

    match session
        .call_token::<_, ()>("force_transfer", &force_transfer)
        .err()
    {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BALANCE_TOO_LOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }
}

#[test]
fn test_sanctions() {
    // TODO: unify transfer logic in the contract so that this implicitly checks the invariants of transferFrom and
    // any other potential function leading to a "transfer" that updates the balance

    const VALUE: u64 = INITIAL_BALANCE / 3;
    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0x1618);
    let test_sk = SecretKey::random(&mut rng);
    let test_pk = PublicKey::from(&test_sk); // not blocked, not frozen

    // Transfer VALUE to test account
    let transfer = Transfer::new_external(&session.deploy_sk, session.deploy_pk, test_pk, VALUE, 1);
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transferring should succeed");

    // Block test account
    let sanction = Sanction::block_account(&session.owner_sk, test_pk, 1);
    session
        .call_token::<_, ()>("block", &sanction)
        .expect("Blocking should succeed");

    assert_eq!(
        session
            .call_token::<_, bool>("blocked", &Account::External(test_pk))
            .expect("Querying the state should succeed")
            .data,
        true
    );

    // Unfreeze test account
    let unsanction = Sanction::unsanction_account(&session.owner_sk, test_pk, 2);
    match session.call_token::<_, ()>("unfreeze", &unsanction).err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, "The account is not frozen");
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Transfer VALUE to test account
    let transfer = Transfer::new_external(&session.deploy_sk, session.deploy_pk, test_pk, VALUE, 2);
    match session.call_token::<_, ()>("transfer", &transfer).err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BLOCKED);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Transfer VALUE from test account
    let transfer = Transfer::new_external(&test_sk, test_pk, session.deploy_pk(), VALUE, 1);
    match session.call_token::<_, ()>("transfer", &transfer).err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BLOCKED);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Freeze test account
    let sanction = Sanction::freeze_account(&session.owner_sk, test_pk, 2);
    session
        .call_token::<_, ()>("freeze", &sanction)
        .expect("Freezing should succeed");

    assert_eq!(
        session
            .call_token::<_, bool>("frozen", &Account::External(test_pk))
            .expect("Querying the state should succeed")
            .data,
        true
    );

    // Transfer VALUE to test account
    let transfer = Transfer::new_external(&session.deploy_sk, session.deploy_pk, test_pk, VALUE, 2);
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transfer to frozen account should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new_external(&test_sk, test_pk, session.deploy_pk(), VALUE, 1);
    match session.call_token::<_, ()>("transfer", &transfer).err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, FROZEN);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Unsanction test account
    let unsanction = Sanction::unsanction_account(&session.owner_sk, test_pk, 3);
    match session.call_token::<_, ()>("unblock", &unsanction).err() {
        Some(VMError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, "The account is not blocked");
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // Unfreeze test account
    let unsanction = Sanction::unsanction_account(&session.owner_sk, test_pk, 3);
    session
        .call_token::<_, ()>("unfreeze", &unsanction)
        .expect("Unfreezing should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new_external(&test_sk, test_pk, session.deploy_pk(), VALUE, 1);
    session
        .call_token::<_, ()>("transfer", &transfer)
        .expect("Transfer should succeed again");
}

fn main() {
    unreachable!("`main` should never run for this crate");
}
