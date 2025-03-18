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

use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::{
    BufferScratch, BufferSerializer, CompositeSerializer,
};
use rkyv::ser::Serializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use ttoken_types::*;

use ttoken_tests::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/ttoken_contract.wasm"
);
const HOLDER_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/ttoken_holder_contract.wasm"
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

pub struct ContractSession {
    session: NetworkSession,
}

impl ContractSession {
    pub const TEST_SK_0: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0x5EAF00D);
        AccountSecretKey::random(&mut rng)
    });

    pub const TEST_PK_0: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::TEST_SK_0));

    pub const TEST_SK_1: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0xF0CACC1A);
        AccountSecretKey::random(&mut rng)
    });

    pub const TEST_PK_1: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::TEST_SK_1));

    pub const TEST_SK_2: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0x5A1AD);
        AccountSecretKey::random(&mut rng)
    });

    pub const TEST_PK_2: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::TEST_SK_2));

    pub const TEST_SK_3: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0xBEEF);
        AccountSecretKey::random(&mut rng)
    });

    pub const TEST_PK_3: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::TEST_SK_3));
}

impl ContractSession {
    pub fn new() -> Self {
        // deploy a session with transfer & stake contract deployed
        // pass a list of accounts to fund
        let mut network_session = NetworkSession::instantiate(vec![
            (&*Self::TEST_PK_0, MOONLIGHT_BALANCE),
            (&*Self::TEST_PK_1, MOONLIGHT_BALANCE),
            (&*Self::TEST_PK_2, MOONLIGHT_BALANCE),
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
                                Account::from(*Self::TEST_PK_0),
                                INITIAL_GOVERNANCE_BALANCE,
                            ),
                            (Account::from(*Self::TEST_PK_1), INITIAL_BALANCE),
                            (Account::from(HOLDER_ID), INITIAL_HOLDER_BALANCE),
                        ],
                        Account::from(*Self::TEST_PK_0),
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
            session: network_session,
        };

        assert_eq!(
            session.account(*Self::TEST_PK_0).balance,
            INITIAL_GOVERNANCE_BALANCE
        );
        assert_eq!(session.account(*Self::TEST_PK_1).balance, INITIAL_BALANCE);
        assert_eq!(session.account(*Self::TEST_PK_2).balance, 0);
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
    pub fn call_token<A>(
        &mut self,
        tx_sk: &AccountSecretKey,
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
    pub fn call_getter<R>(&mut self, fn_name: &str) -> Result<CallReceipt<R>>
    where
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        // TODO: find out if there is another way to do that instead of passing
        // &() as fn_arg
        self.session.direct_call::<(), R>(TOKEN_ID, fn_name, &())
    }

    pub fn call_holder<A>(
        &mut self,
        tx_sk: &AccountSecretKey,
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

    pub fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
        self.session
            .direct_call(TOKEN_ID, "account", &account.into())
            .expect("Querying an account should succeed")
            .data
    }

    pub fn governance(&mut self) -> Account {
        self.call_getter("governance")
            .expect("Querying governance should succeed")
            .data
    }

    pub fn total_supply(&mut self) -> u64 {
        self.call_getter("total_supply")
            .expect("Querying the supply should succeed")
            .data
    }

    pub fn allowance(
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
