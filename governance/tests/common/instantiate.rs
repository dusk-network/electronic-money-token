// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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

use emt_core::*;

use emt_tests::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] = include_bytes!(
    "../../../target/wasm64-unknown-unknown/release/emt_token.wasm"
);
const GOVERNANCE_BYTECODE: &[u8] = include_bytes!(
    "../../../target/wasm64-unknown-unknown/release/emt_governance.wasm"
);
// const HOLDER_BYTECODE: &[u8] = include_bytes!(
//     "../../target/wasm64-unknown-unknown/release/emt_holder_contract.wasm"
// );

const DEPLOYER: [u8; 64] = [0u8; 64];

pub const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
pub const GOVERNANCE_ID: ContractId = ContractId::from_bytes([2; 32]);
// pub const HOLDER_ID: ContractId = ContractId::from_bytes([3; 32]);

pub const INITIAL_BALANCE: u64 = 1000;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

pub struct TestKeys<const O: usize, const P: usize, const H: usize> {
    pub owners_sk: [AccountSecretKey; O],
    pub owners_pk: [AccountPublicKey; O],
    pub operators_sk: [AccountSecretKey; P],
    pub operators_pk: [AccountPublicKey; P],
    pub holders_sk: [AccountSecretKey; H],
    pub holders_pk: [AccountPublicKey; H],
}

impl<const O: usize, const P: usize, const H: usize> TestKeys<O, P, H> {
    pub fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(0x5EAF00D);

        // generate owners keys
        let mut owners_sk = Vec::with_capacity(O);
        let mut owners_pk = Vec::with_capacity(O);
        for _ in 0..O {
            let sk = AccountSecretKey::random(&mut rng);
            let pk = AccountPublicKey::from(&sk);
            owners_sk.push(sk);
            owners_pk.push(pk);
        }

        // generate operators keys
        let mut operators_sk = Vec::with_capacity(P);
        let mut operators_pk = Vec::with_capacity(P);
        for _ in 0..P {
            let sk = AccountSecretKey::random(&mut rng);
            let pk = AccountPublicKey::from(&sk);
            operators_sk.push(sk);
            operators_pk.push(pk);
        }

        // generate holders keys
        let mut holders_sk = Vec::with_capacity(P);
        let mut holders_pk = Vec::with_capacity(P);
        for _ in 0..P {
            let sk = AccountSecretKey::random(&mut rng);
            let pk = AccountPublicKey::from(&sk);
            holders_sk.push(sk);
            holders_pk.push(pk);
        }

        Self {
            owners_sk: owners_sk.try_into().unwrap(),
            owners_pk: owners_pk.try_into().unwrap(),
            operators_sk: operators_sk.try_into().unwrap(),
            operators_pk: operators_pk.try_into().unwrap(),
            holders_sk: holders_sk.try_into().unwrap(),
            holders_pk: holders_pk.try_into().unwrap(),
        }
    }
}

pub struct TestSession {
    session: NetworkSession,
}

impl TestSession {
    pub fn new<const O: usize, const P: usize, const H: usize>() -> Self {
        let test_keys: TestKeys<O, P, H> = TestKeys::new();

        // deploy a session with transfer & stake contract deployed and a list
        // of public accounts that own DUSK for gas-costs
        const MOONLIGHT_BALANCE: u64 = dusk(1_000.0);
        let mut public_keys = Vec::with_capacity(O + P + H);
        public_keys.extend_from_slice(&test_keys.owners_pk);
        public_keys.extend_from_slice(&test_keys.operators_pk);
        public_keys.extend_from_slice(&test_keys.holders_pk);
        let public_balances = public_keys
            .iter()
            .map(|pk| (pk, MOONLIGHT_BALANCE))
            .collect();
        let mut network_session = NetworkSession::instantiate(public_balances);

        // deploy the token-contract
        let token_init_args = (
            // fund all keys with an initial balance
            public_keys
                .iter()
                .map(|pk| (Account::from(*pk), INITIAL_BALANCE))
                .collect::<Vec<_>>(),
            // set the governance-contract as token-contract governance
            Account::from(GOVERNANCE_ID),
        );
        network_session
            .deploy(
                TOKEN_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&token_init_args)
                    .contract_id(TOKEN_ID),
            )
            .expect("Deploying the token-contract should succeed");

        // deploy the governance-contract
        let governance_init_args = (
            // set the token-contract in the governance state
            TOKEN_ID,
            // set the owner and operator keys
            test_keys.owners_pk.to_vec(),
            test_keys.operators_pk.to_vec(),
            // register all operator token-contract calls
            vec![
                // block and freeze need 1 sig
                ("block".to_string(), 1),
                ("freeze".to_string(), 1),
                ("unblock".to_string(), 1),
                ("unfreeze".to_string(), 1),
                // everything else needs a supermajority
                ("mint".to_string(), 0),
                ("burn".to_string(), 0),
                ("toggle_pause".to_string(), 0),
                ("forced_transfer".to_string(), 0),
            ],
        );
        network_session
            .deploy(
                GOVERNANCE_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&governance_init_args)
                    .contract_id(GOVERNANCE_ID),
            )
            .expect("Deploying the governance-contract should succeed");

        // deploy the test holder-contract
        // let holder_init_args = (TOKEN_ID, INITIAL_BALANCE_HOLDER_CONTRACT);
        // network_session
        //     .deploy(
        //         HOLDER_BYTECODE,
        //         ContractData::builder()
        //             .owner(DEPLOYER)
        //             .init_arg(&holder_init_args)
        //             .contract_id(HOLDER_ID),
        //     )
        //     .expect("Deploying the test holder-contract should succeed");
        //
        let session = Self {
            session: network_session,
        };

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

    /// Execute a state-transition of the governance-contract, paying gas with
    /// `tx_sk`.
    pub fn execute_governance<A>(
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
    {
        let vec_fn_arg = Self::serialize(fn_arg);

        // deserialize the vec_fn_arg for sanity check
        let back = rkyv::from_bytes::<A>(&vec_fn_arg)
            .expect("failed to deserialize previously serialized fn_arg");

        assert_eq!(&back, fn_arg);

        self.session
            .icc_transaction(tx_sk, GOVERNANCE_ID, fn_name, vec_fn_arg)
    }

    /// Query the governance-contract directly without paying gas.
    pub fn query_governance<A, R>(
        &mut self,
        fn_name: &str,
        fn_arg: &A,
    ) -> CallReceipt<R>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>
            + PartialEq
            + std::fmt::Debug,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        <A as Archive>::Archived: Deserialize<A, SharedDeserializeMap>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session
            .direct_call::<A, R>(GOVERNANCE_ID, fn_name, fn_arg)
    }

    /// Query the token-contract directly without paying gas.
    pub fn query_token<A, R>(
        &mut self,
        fn_name: &str,
        fn_arg: &A,
    ) -> CallReceipt<R>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>
            + PartialEq
            + std::fmt::Debug,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        <A as Archive>::Archived: Deserialize<A, SharedDeserializeMap>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.direct_call::<A, R>(TOKEN_ID, fn_name, fn_arg)
    }

    // pub fn call_holder<A>(
    //     &mut self,
    //     tx_sk: &AccountSecretKey,
    //     fn_name: &str,
    //     fn_arg: &A,
    // ) -> CallReceipt<Result<Vec<u8>, ContractError>>
    // where
    //     A: for<'b> Serialize<StandardBufSerializer<'b>>,
    //     A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
    // {
    //     let fn_arg = Self::serialize(fn_arg);
    //
    //     self.session
    //         .icc_transaction(tx_sk, HOLDER_ID, fn_name, fn_arg)
    // }
    //
    // pub fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
    //     self.session
    //         .direct_call(TOKEN_ID, "account", &account.into())
    //         .data
    // }
    //
    // pub fn governance(&mut self) -> Account {
    //     self.call_getter("governance").data
    // }
    //
    // pub fn total_supply(&mut self) -> u64 {
    //     self.call_getter("total_supply").data
    // }
    //
    // pub fn allowance(
    //     &mut self,
    //     owner: impl Into<Account>,
    //     spender: impl Into<Account>,
    // ) -> u64 {
    //     self.session
    //         .direct_call(
    //             TOKEN_ID,
    //             "allowance",
    //             &Allowance {
    //                 owner: owner.into(),
    //                 spender: spender.into(),
    //             },
    //         )
    //         .expect("Querying an allowance should succeed")
    //         .data
    // }
}
