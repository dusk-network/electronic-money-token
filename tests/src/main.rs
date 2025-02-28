// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod common;

use dusk_core::abi::ContractError;
use dusk_core::abi::{ContractId, StandardBufSerializer, CONTRACT_ID_BYTES};
use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::MoonlightTransactionEvent;
use dusk_vm::{CallReceipt, ContractData, Error as VMError};

use bytecheck::CheckBytes;

use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::{
    BufferScratch, BufferSerializer, CompositeSerializer,
};
use rkyv::ser::Serializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use ttoken_types::admin_management::PAUSED_MESSAGE;
use ttoken_types::ownership::arguments::TransferOwnership;
use ttoken_types::ownership::UNAUTHORIZED_ACCOUNT;
use ttoken_types::sanctions::arguments::Sanction;
use ttoken_types::sanctions::{BLOCKED, FROZEN};
use ttoken_types::supply_management::SUPPLY_OVERFLOW;
use ttoken_types::*;

use crate::common::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] =
    include_bytes!("../../build/ttoken_contract.wasm");
const HOLDER_BYTECODE: &[u8] =
    include_bytes!("../../build/ttoken_holder_contract.wasm");

const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
const HOLDER_ID: ContractId = ContractId::from_bytes([2; 32]);

const MOONLIGHT_BALANCE: u64 = dusk(1_000.0);
const INITIAL_BALANCE: u64 = 1000;
const INITIAL_HOLDER_BALANCE: u64 = 1000;
const INITIAL_OWNER_BALANCE: u64 = 1000;
const INITIAL_SUPPLY: u64 =
    INITIAL_BALANCE + INITIAL_HOLDER_BALANCE + INITIAL_OWNER_BALANCE;

const DEPLOYER: [u8; 64] = [0u8; 64];

type Result<T, Error = VMError> = core::result::Result<T, Error>;

struct ContractSession {
    deploy_pk: AccountPublicKey,
    deploy_sk: AccountSecretKey,
    owner_sk: AccountSecretKey,
    owner: AccountPublicKey,
    test_sk: AccountSecretKey,
    test_pk: AccountPublicKey,
    session: NetworkSession,
}

impl ContractSession {
    fn deploy_sk(&self) -> AccountSecretKey {
        self.deploy_sk.clone()
    }

    fn owner_sk(&self) -> AccountSecretKey {
        self.owner_sk.clone()
    }

    fn test_sk(&self) -> AccountSecretKey {
        self.test_sk.clone()
    }

    /// Deployer of the contract
    fn deploy_account(&self) -> Account {
        Account::External(self.deploy_pk)
    }

    /// Owner of the contract for admin functionality
    fn owner_account(&self) -> Account {
        Account::External(self.owner)
    }

    /// Random test account
    fn test_account(&self) -> Account {
        Account::External(self.test_pk)
    }

    fn deploy_pk(&self) -> AccountPublicKey {
        self.deploy_pk
    }
}

impl ContractSession {
    fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(0xF0CACC1A);
        let deploy_sk = AccountSecretKey::random(&mut rng);
        let deploy_pk = AccountPublicKey::from(&deploy_sk);

        let owner_sk = AccountSecretKey::random(&mut rng);
        let owner_pk = AccountPublicKey::from(&owner_sk);

        let mut rng = StdRng::seed_from_u64(0xBEEF);
        let test_sk = AccountSecretKey::random(&mut rng);
        let test_pk = AccountPublicKey::from(&test_sk);

        // deploy a session with transfer & stake contract deployed
        // pass a list of accounts to fund
        let mut network_session = NetworkSession::instantiate(vec![
            (&deploy_pk, MOONLIGHT_BALANCE),
            (&owner_pk, MOONLIGHT_BALANCE),
            (&test_pk, MOONLIGHT_BALANCE),
        ]);

        // never set owner to deploy
        assert_ne!(owner_sk, deploy_sk);

        // deploy the Token contract
        network_session
            .deploy(
                TOKEN_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(
                        vec![
                            (Account::External(deploy_pk), INITIAL_BALANCE),
                            (
                                Account::Contract(HOLDER_ID),
                                INITIAL_HOLDER_BALANCE,
                            ),
                            (
                                Account::External(owner_pk),
                                INITIAL_OWNER_BALANCE,
                            ),
                        ],
                        Account::External(owner_pk),
                    ))
                    .contract_id(TOKEN_ID),
            )
            .expect("Deploying the token contract should succeed");

        // deploy the holder contract
        network_session
            .deploy(
                HOLDER_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(TOKEN_ID, INITIAL_HOLDER_BALANCE))
                    .contract_id(HOLDER_ID),
            )
            .expect("Deploying the holder contract should succeed");

        let mut session = Self {
            deploy_sk,
            deploy_pk,
            owner_sk,
            owner: owner_pk,
            test_sk,
            test_pk,
            session: network_session,
        };

        assert_eq!(session.account(deploy_pk).balance, INITIAL_BALANCE);
        assert_eq!(session.account(owner_pk).balance, INITIAL_OWNER_BALANCE);
        assert_eq!(session.account(test_pk).balance, 0);
        assert_eq!(session.account(HOLDER_ID).balance, INITIAL_HOLDER_BALANCE);

        session
    }

    fn serialize<A>(fn_arg: &A) -> Vec<u8>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let mut sbuf = [0u8; 1024];
        let scratch = BufferScratch::new(&mut sbuf);
        let mut buffer = [0u8; 1024];
        let ser = BufferSerializer::new(&mut buffer[..]);
        let mut ser = CompositeSerializer::new(ser, scratch, Infallible);

        ser.serialize_value(fn_arg)
            .expect("Failed to rkyv serialize fn_arg");
        let pos = ser.pos();

        buffer[..pos].to_vec()
    }

    // TODO: Find a way to return CallReceipt<R>
    fn call_token<A>(
        &mut self,
        tx_sk: AccountSecretKey,
        fn_name: &str,
        fn_arg: &A,
    ) -> CallReceipt<Result<Vec<u8>, ContractError>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>
            + PartialEq
            + std::fmt::Debug,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        <A as Archive>::Archived: Deserialize<A, SharedDeserializeMap>,
        //R: Archive,
        //R::Archived: Deserialize<R, Infallible> + for<'b>
        // CheckBytes<DefaultValidator<'b>>,
    {
        let vec_fn_arg;
        {
            vec_fn_arg = Self::serialize(fn_arg);

            // deserialize the vec_fn_arg for sanity check
            let back = rkyv::from_bytes::<A>(&vec_fn_arg)
                .expect("failed to deserialize previously serialized fn_arg");

            assert_eq!(&back, fn_arg);
        }

        self.session
            .icc_transaction(tx_sk, TOKEN_ID, fn_name, vec_fn_arg)
    }

    /// Helper function to call a "view" function on the token contract that
    /// does not take any arguments.
    fn call_getter<R>(&mut self, fn_name: &str) -> Result<CallReceipt<R>>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        // TODO: find out if there is another way to do that instead of passing
        // &() as fn_arg
        self.session.direct_call::<(), R>(TOKEN_ID, fn_name, &())
    }

    fn call_holder<A>(
        &mut self,
        tx_sk: AccountSecretKey,
        fn_name: &str,
        fn_arg: &A,
    ) -> CallReceipt<Result<Vec<u8>, ContractError>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let fn_arg = Self::serialize(fn_arg);

        self.session
            .icc_transaction(tx_sk, HOLDER_ID, fn_name, fn_arg)
    }

    fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
        self.session
            .direct_call(TOKEN_ID, "account", &account.into())
            .expect("Querying an account should succeed")
            .data
    }

    fn owner(&mut self) -> Account {
        self.call_getter("owner")
            .expect("Querying owner should succeed")
            .data
    }

    fn total_supply(&mut self) -> u64 {
        self.call_getter("total_supply")
            .expect("Querying the supply should succeed")
            .data
    }

    fn allowance(
        &mut self,
        owner: impl Into<Account>,
        spender: impl Into<Account>,
    ) -> u64 {
        self.session
            .direct_call(
                TOKEN_ID,
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

    let receiver_pk = session.test_pk;

    assert_eq!(
        session.account(session.deploy_pk()).balance,
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
        session.call_token(session.deploy_sk(), "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(session.deploy_pk()).balance,
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
        session.account(session.deploy_pk()).balance,
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
        session.call_token(session.deploy_sk(), "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

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

/// Test a token transfer from the HOLDER_ID contract account to the deploy
/// account.
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

    let transfer = Transfer::new(session.deploy_pk(), TRANSFERRED_AMOUNT);

    let receipt =
        session.call_holder::<_>(session.deploy_sk(), "token_send", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    receipt.events.iter().for_each(|event| {
        if event.topic == "moonlight" {
            let transfer_info =
                rkyv::from_bytes::<MoonlightTransactionEvent>(&event.data)
                    .unwrap();

            assert!(
                transfer_info.sender == session.deploy_pk(),
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
                transfer_event.receiver == session.deploy_account(),
                "The receiver should be the deploy account"
            );
            assert_eq!(
                transfer_event.value, TRANSFERRED_AMOUNT,
                "The transferred amount should be the same"
            );
        }
    });

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

/// Test approval of deploy account to test account.
#[test]
fn approve() {
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE - 1;

    let mut session = ContractSession::new();

    let test_account = session.test_account();

    assert_eq!(
        session.allowance(session.deploy_pk(), test_account),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new(test_account, APPROVED_AMOUNT);
    let receipt = session.call_token(session.deploy_sk(), "approve", &approve);

    if let Err(e) = receipt.data {
        panic!("Approve should succeed, err: {e}");
    }

    assert_eq!(
        session.allowance(session.deploy_pk(), test_account),
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
    let spender_account = session.test_account();

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );
    assert_eq!(
        session.account(spender_account).balance,
        0,
        "The account to transfer to should have no balance"
    );
    assert_eq!(
        session.allowance(session.deploy_pk(), spender_account),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new(spender_account, APPROVED_AMOUNT);

    let receipt = session.call_token(session.deploy_sk(), "approve", &approve);
    receipt.data.expect("Approve should succeed");

    assert_eq!(
        session.allowance(session.deploy_pk(), spender_account),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );

    let transfer_from = TransferFrom::new(
        session.deploy_pk(),
        spender_account,
        TRANSFERRED_AMOUNT,
    );
    let receipt =
        session.call_token(session.test_sk(), "transfer_from", &transfer_from);

    if let Err(e) = receipt.data {
        panic!("Transfer from should succeed, err: {e}");
    }

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - TRANSFERRED_AMOUNT,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(spender_account).balance,
        TRANSFERRED_AMOUNT,
        "The account transferred to should have the transferred amount"
    );
    assert_eq!(
        session.allowance(session.deploy_pk(), spender_account),
        APPROVED_AMOUNT - TRANSFERRED_AMOUNT,
        "The account should have the transferred amount subtracted from its allowance"
    );
}

/// Test transfer of ownership from owner account to test account.
#[test]
fn transfer_ownership() {
    let mut session = ContractSession::new();
    let new_owner = session.test_account();

    let transfer_ownership = TransferOwnership::new(new_owner);
    let receipt = session.call_token(
        session.owner_sk(),
        "transfer_ownership",
        &transfer_ownership,
    );

    if let Err(e) = receipt.data {
        panic!("Transfer ownership should succeed, err: {e}");
    }

    assert_eq!(session.owner(), new_owner);
}

/// Test TransferOwnership, RenounceOwnership with wrong owner
/// and check for correct error message.
///
/// TODO: Squash wrong sk case with transfer ownership & renounce ownership
/// tests functions as the other tests (mint, burn etc) do it.
#[test]
fn ownership_wrong_owner() {
    let mut session = ContractSession::new();

    let wrong_owner_sk = session.test_sk();
    let new_owner = session.test_account();

    let transfer_ownership = TransferOwnership::new(new_owner);
    let receipt = session.call_token(
        wrong_owner_sk.clone(),
        "transfer_ownership",
        &transfer_ownership,
    );

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    let receipt = session.call_token(wrong_owner_sk, "renounce_ownership", &());

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, UNAUTHORIZED_ACCOUNT);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    assert_eq!(session.owner(), session.owner_account());
}

/// Test renounce ownership.
#[test]
fn renounce_ownership() {
    let mut session = ContractSession::new();

    let receipt =
        session.call_token(session.owner_sk(), "renounce_ownership", &());

    if let Err(e) = receipt.data {
        panic!("Renounce ownership should succeed, err: {e}");
    }

    let owner = session.owner();

    assert_eq!(
        owner,
        // TODO: consider defining this as ZERO_ADDRESS in core?
        Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]))
    );
}

/// Test mint with owner sk
/// Test mint with wrong sk
/// Test mint with overflow
#[test]
fn test_mint() {
    let mut session = ContractSession::new();
    let mint_amount = 1000;

    // Note: Direct usage of PublicKey here fails during rkyv deserialization.
    // TODO: Consider changing call_token to support types implementing
    // Into<Account> by somehow detecting the types the fn expects.
    let mint_receiver = session.owner_account();

    assert_eq!(session.total_supply(), INITIAL_SUPPLY);

    // mint with owner sk
    let receipt = session.call_token(
        session.owner_sk(),
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
        session.owner_sk(),
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
        session.test_sk(),
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

/// Test burn with owner sk
/// Test burn with wrong sk
/// Test burn with balance too low / underflow
#[test]
fn test_burn() {
    let mut session = ContractSession::new();
    let burn_amount = 1000;

    let receipt = session.call_token(session.owner_sk(), "burn", &burn_amount);

    if let Err(e) = receipt.data {
        panic!("Burn should succeed, err: {e}");
    }

    assert_eq!(session.total_supply(), INITIAL_SUPPLY - burn_amount);

    // burn more than the owner account has
    let burn_amount = u64::MAX;

    let receipt = session.call_token(session.owner_sk(), "burn", &burn_amount);

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, BALANCE_TOO_LOW);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    // unauthorized account
    let receipt = session.call_token(session.test_sk(), "burn", &burn_amount);

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

    let receipt = session.call_token(session.owner_sk(), "toggle_pause", &());

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

    let transfer = Transfer::new(session.test_account(), VALUE);
    let receipt =
        session.call_token(session.deploy_sk(), "transfer", &transfer);

    match receipt.data.err() {
        Some(ContractError::Panic(panic_msg)) => {
            assert_eq!(panic_msg, PAUSED_MESSAGE);
        }
        _ => {
            panic!("Expected a panic error");
        }
    }

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE,
        "The deployed account should have the initial balance"
    );

    assert_eq!(
        session.account(session.test_account()).balance,
        0,
        "The account to transfer to should have no balance"
    );

    let receipt = session.call_token(session.owner_sk(), "toggle_pause", &());

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
        session.call_token(session.deploy_sk(), "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed again, err: {e}");
    }

    // unauthorized account
    let receipt = session.call_token(session.test_sk(), "toggle_pause", &());

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
    let transfer = Transfer::new(session.test_account(), VALUE);
    let receipt =
        session.call_token(session.deploy_sk(), "transfer", &transfer);

    if let Err(e) = receipt.data {
        panic!("Transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(session.deploy_pk()).balance,
        INITIAL_BALANCE - VALUE,
        "The deployed account should have the transferred amount subtracted"
    );
    assert_eq!(
        session.account(session.test_account()).balance,
        VALUE,
        "The test account should have the transferred amount"
    );

    // Force transfer from test account to owner account
    let force_transfer = Transfer::new(session.owner, VALUE);
    let obliged_sender = session.test_account();
    let receipt = session.call_token(
        session.owner_sk(),
        "force_transfer",
        &(force_transfer, obliged_sender),
    );

    if let Err(e) = receipt.data {
        panic!("Force transfer should succeed, err: {e}");
    }

    assert_eq!(
        session.account(session.test_account()).balance,
        0,
        "The test account should have the transferred amount subtracted"
    );

    assert_eq!(
        session.account(session.owner_account()).balance,
        INITIAL_BALANCE + VALUE,
        "The owner account should have the transferred amount added"
    );

    // Force transfer from test account to owner account again (balance will be
    // too low)
    let force_transfer = Transfer::new(session.owner, VALUE);

    match session
        .call_token(
            session.owner_sk(),
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
    let force_transfer = Transfer::new(session.test_account(), VALUE);
    let obliged_sender = session.owner_account();
    let receipt = session.call_token(
        session.test_sk(),
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
    let blocked_account = session.test_account();

    // Transfer VALUE to test account
    let transfer = Transfer::new(blocked_account, VALUE);
    session
        .call_token(session.deploy_sk(), "transfer", &transfer)
        .data
        .expect("Transfer should succeed");

    // Block test account
    let sanction = Sanction::block_account(blocked_account);
    let receipt = session.call_token(session.owner_sk(), "block", &sanction);

    if let Err(e) = receipt.data {
        panic!("Block should succeed, err: {e}");
    }

    assert_eq!(
        rkyv::from_bytes::<bool>(
            &session
                .call_token(session.test_sk(), "blocked", &blocked_account)
                .data
                .expect("Querying the state should succeed")
        )
        .expect("Deserializing the state should succeed"),
        true
    );

    // Unfreeze test account
    let unsanction = Sanction::unsanction_account(blocked_account);
    match session
        .call_token(session.owner_sk(), "unfreeze", &unsanction)
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
        .call_token(session.deploy_sk(), "transfer", &transfer)
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
    let transfer = Transfer::new(session.deploy_pk(), VALUE);
    match session
        .call_token(session.test_sk(), "transfer", &transfer)
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
    let receipt = session.call_token(session.owner_sk(), "freeze", &sanction);

    if let Err(e) = receipt.data {
        panic!("Freeze should succeed, err: {e}");
    }

    assert_eq!(
        rkyv::from_bytes::<bool>(
            &session
                .call_token(session.test_sk(), "frozen", &frozen_account)
                .data
                .expect("Querying the state should succeed")
        )
        .expect("Deserializing the state should succeed"),
        true
    );

    // Transfer VALUE to test account
    let transfer = Transfer::new(frozen_account, VALUE);
    session
        .call_token(session.deploy_sk(), "transfer", &transfer)
        .data
        .expect("Transfer to frozen account should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new(session.deploy_pk(), VALUE);
    match session
        .call_token(session.test_sk(), "transfer", &transfer)
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
        .call_token(session.owner_sk(), "unblock", &unsanction)
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
        .call_token(session.test_sk(), "unblock", &unsanction)
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
        .call_token(session.test_sk(), "unfreeze", &unsanction)
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
        .call_token(session.owner_sk(), "unfreeze", &unsanction)
        .data
        .expect("Unfreezing should succeed");

    // Transfer VALUE from test account
    let transfer = Transfer::new(session.deploy_pk(), VALUE);
    session
        .call_token(session.test_sk(), "transfer", &transfer)
        .data
        .expect("Transfer should succeed again");
}

fn main() {
    unreachable!("`main` should never run for this crate");
}
