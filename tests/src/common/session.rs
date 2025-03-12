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

use ttoken_types::*;

use crate::common::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] =
    include_bytes!("../../../build/ttoken_contract.wasm");
const HOLDER_BYTECODE: &[u8] =
    include_bytes!("../../../build/ttoken_holder_contract.wasm");

pub const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
pub const HOLDER_ID: ContractId = ContractId::from_bytes([2; 32]);

pub const MOONLIGHT_BALANCE: u64 = dusk(1_000.0);
pub const INITIAL_BALANCE: u64 = 1000;
pub const INITIAL_HOLDER_BALANCE: u64 = 1000;
pub const INITIAL_OWNER_BALANCE: u64 = 1000;
pub const INITIAL_SUPPLY: u64 =
    INITIAL_BALANCE + INITIAL_HOLDER_BALANCE + INITIAL_OWNER_BALANCE;

const DEPLOYER: [u8; 64] = [0u8; 64];

type Result<T, Error = VMError> = core::result::Result<T, Error>;

pub struct ContractSession {
    deploy_sk: AccountSecretKey,
    deploy_pk: AccountPublicKey,
    owner_sk: AccountSecretKey,
    owner: AccountPublicKey,
    test_sk: AccountSecretKey,
    test_pk: AccountPublicKey,
    session: NetworkSession,
}

impl ContractSession {
    pub fn deploy_sk(&self) -> AccountSecretKey {
        self.deploy_sk.clone()
    }

    pub fn owner_sk(&self) -> AccountSecretKey {
        self.owner_sk.clone()
    }

    pub fn test_sk(&self) -> AccountSecretKey {
        self.test_sk.clone()
    }

    /// Deployer of the contract
    pub fn deploy_account(&self) -> Account {
        Account::External(self.deploy_pk)
    }

    /// Owner of the contract for admin functionality
    pub fn owner_account(&self) -> Account {
        Account::External(self.owner)
    }

    /// Random test account
    pub fn test_account(&self) -> Account {
        Account::External(self.test_pk)
    }

    pub fn deploy_pk(&self) -> AccountPublicKey {
        self.deploy_pk
    }

    pub fn test_pk(&self) -> AccountPublicKey {
        self.test_pk
    }
}

impl ContractSession {
    pub fn new() -> Self {
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
    pub fn call_token<A>(
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

    pub fn account(&mut self, account: impl Into<Account>) -> AccountInfo {
        self.session
            .direct_call(TOKEN_ID, "account", &account.into())
            .expect("Querying an account should succeed")
            .data
    }

    pub fn owner(&mut self) -> Account {
        self.call_getter("owner")
            .expect("Querying owner should succeed")
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
