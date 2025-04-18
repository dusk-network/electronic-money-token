// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use dusk_core::abi::ContractError;
use dusk_core::abi::{ContractId, StandardBufSerializer};
use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_vm::{CallReceipt, ContractData, Error as VMError};

use bytecheck::CheckBytes;

use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use emt_core::*;

use emt_tests::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/emt_token.wasm"
);
const HOLDER_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/emt_holder_contract.wasm"
);

const DEPLOYER: [u8; 64] = [0u8; 64];

pub const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
pub const HOLDER_ID: ContractId = ContractId::from_bytes([2; 32]);

pub const MOONLIGHT_BALANCE: u64 = dusk(1_000.0);
pub const INITIAL_BALANCE: u64 = 1000;
pub const INITIAL_HOLDER_BALANCE: u64 = 1000;
pub const INITIAL_GOVERNANCE_BALANCE: u64 = 1000;
pub const INITIAL_SUPPLY: u64 =
    INITIAL_BALANCE + INITIAL_HOLDER_BALANCE + INITIAL_GOVERNANCE_BALANCE;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

pub struct TestSession {
    session: NetworkSession,
}

impl TestSession {
    pub const SK_0: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0x5EAF00D);
        AccountSecretKey::random(&mut rng)
    });

    pub const PK_0: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::SK_0));

    pub const SK_1: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0xF0CACC1A);
        AccountSecretKey::random(&mut rng)
    });

    pub const PK_1: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::SK_1));

    pub const SK_2: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0x5A1AD);
        AccountSecretKey::random(&mut rng)
    });

    /// Test session public key for the second account. Does not have any
    /// tokens.
    pub const PK_2: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::SK_2));
}

impl TestSession {
    pub fn new() -> Self {
        // deploy a session with transfer & stake contract deployed
        // pass a list of accounts to fund
        let mut network_session = NetworkSession::instantiate(vec![
            (&*Self::PK_0, MOONLIGHT_BALANCE),
            (&*Self::PK_1, MOONLIGHT_BALANCE),
            (&*Self::PK_2, MOONLIGHT_BALANCE),
        ]);

        // deploy the Token contract
        network_session
            .deploy(
                TOKEN_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(
                        vec![
                            (
                                Account::from(*Self::PK_0),
                                INITIAL_GOVERNANCE_BALANCE,
                            ),
                            (Account::from(*Self::PK_1), INITIAL_BALANCE),
                            (Account::from(HOLDER_ID), INITIAL_HOLDER_BALANCE),
                        ],
                        Account::from(*Self::PK_0),
                    ))
                    .contract_id(TOKEN_ID),
            )
            .expect("Deploying the token-contract should succeed");

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
            session: network_session,
        };

        assert_eq!(
            session.account(*Self::PK_0).balance,
            INITIAL_GOVERNANCE_BALANCE
        );
        assert_eq!(session.account(*Self::PK_1).balance, INITIAL_BALANCE);
        assert_eq!(session.account(*Self::PK_2).balance, 0);
        assert_eq!(session.account(HOLDER_ID).balance, INITIAL_HOLDER_BALANCE);

        session
    }

    pub fn call_token<A, R>(
        &mut self,
        tx_sk: &AccountSecretKey,
        fn_name: &str,
        fn_arg: &A,
    ) -> Result<CallReceipt<R>, ContractError>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session
            .icc_transaction(tx_sk, TOKEN_ID, fn_name, fn_arg)
    }

    fn call_token_getter<R>(&mut self, fn_name: &str) -> CallReceipt<R>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.direct_call::<(), R>(TOKEN_ID, fn_name, &()).expect(
            format!(
                "Calling the getter function {} on the token-contract should succeed",
                fn_name
            )
            .as_str(),
        )
    }

    fn call_holder_getter<R>(&mut self, fn_name: &str) -> CallReceipt<R>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.direct_call::<(), R>(HOLDER_ID, fn_name, &()).expect(format!(
            "Calling the getter function {} on the holder-contract should succeed",
            fn_name
        ).as_str())
    }

    pub fn call_holder<A, R>(
        &mut self,
        tx_sk: &AccountSecretKey,
        fn_name: &str,
        fn_arg: &A,
    ) -> Result<CallReceipt<R>, ContractError>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session
            .icc_transaction(tx_sk, HOLDER_ID, fn_name, fn_arg)
    }

    pub fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
        self.session
            .direct_call(TOKEN_ID, "account", &account.into())
            .expect("call to pass")
            .data
    }

    pub fn balance_of(&mut self, account: impl Into<Account>) -> u64 {
        self.session
            .direct_call(TOKEN_ID, "balance_of", &account.into())
            .expect("call to pass")
            .data
    }

    pub fn governance(&mut self) -> Account {
        self.call_token_getter("governance").data
    }

    pub fn total_supply(&mut self) -> u64 {
        self.call_token_getter("total_supply").data
    }

    /// Query the paused status of the EMT token contract.
    pub fn is_paused(&mut self) -> bool {
        self.call_token_getter("is_paused").data
    }

    /// Query the balance the holder contract is tracking and therefore aware
    /// of.
    pub fn holder_tracked_balance(&mut self) -> u64 {
        self.call_holder_getter::<u64>("tracked_balance").data
    }

    pub fn allowance(
        &mut self,
        owner: impl Into<Account>,
        spender: impl Into<Account>,
    ) -> u64 {
        self.session
            .direct_call(TOKEN_ID, "allowance", &(owner.into(), spender.into()))
            .expect("call to pass")
            .data
    }
}
