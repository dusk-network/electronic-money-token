// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::abi::{ContractError, ContractId, StandardBufSerializer};
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

use emt_core::Account;

use emt_tests::network::NetworkSession;

const TOKEN_BYTECODE: &[u8] = include_bytes!(
    "../../../target/wasm64-unknown-unknown/release/emt_token.wasm"
);
const ACCESS_CONTROL_BYTECODE: &[u8] = include_bytes!(
    "../../../target/wasm64-unknown-unknown/release/emt_access_control.wasm"
);

const DEPLOYER: [u8; 64] = [0u8; 64];

pub const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
pub const ACCESS_CONTROL_ID: ContractId = ContractId::from_bytes([2; 32]);

pub const INITIAL_BALANCE: u64 = 1000;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

pub struct TestKeys<const O: usize, const P: usize, const H: usize> {
    pub owners_sk: [AccountSecretKey; O],
    pub owners_pk: [AccountPublicKey; O],
    pub operators_sk: [AccountSecretKey; P],
    pub operators_pk: [AccountPublicKey; P],
    pub test_sk: [AccountSecretKey; H],
    pub test_pk: [AccountPublicKey; H],
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

        // generate test keys
        let mut test_sk = Vec::with_capacity(H);
        let mut test_pk = Vec::with_capacity(H);
        for _ in 0..H {
            let sk = AccountSecretKey::random(&mut rng);
            let pk = AccountPublicKey::from(&sk);
            test_sk.push(sk);
            test_pk.push(pk);
        }

        Self {
            owners_sk: owners_sk.try_into().unwrap(),
            owners_pk: owners_pk.try_into().unwrap(),
            operators_sk: operators_sk.try_into().unwrap(),
            operators_pk: operators_pk.try_into().unwrap(),
            test_sk: test_sk.try_into().unwrap(),
            test_pk: test_pk.try_into().unwrap(),
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
        public_keys.extend_from_slice(&test_keys.test_pk);
        let public_balances = public_keys
            .iter()
            .map(|pk| (pk, MOONLIGHT_BALANCE))
            .collect();
        let mut network_session = NetworkSession::instantiate(public_balances);

        // deploy the token-contract
        // fund all keys and the access-control-contract itself with an initial
        // balance
        let mut initial_balances = public_keys
            .iter()
            .map(|pk| (Account::from(*pk), INITIAL_BALANCE))
            .collect::<Vec<_>>();
        initial_balances
            .push((Account::Contract(ACCESS_CONTROL_ID), INITIAL_BALANCE));
        let token_init_args = (
            initial_balances,
            // set the access-control-contract as token-contract access-control
            Account::from(ACCESS_CONTROL_ID),
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

        // deploy the access-control-contract
        let access_control_init_args = (
            // set the token-contract in the access-control state
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
                ("force_transfer".to_string(), 0),
            ],
        );
        network_session
            .deploy(
                ACCESS_CONTROL_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&access_control_init_args)
                    .contract_id(ACCESS_CONTROL_ID),
            )
            .expect("Deploying the access-control-contract should succeed");

        let session = Self {
            session: network_session,
        };

        session
    }

    /// Execute a state-transition of the access-control-contract, paying gas
    /// with `tx_sk`.
    pub fn execute_access_control<A, R>(
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
            .icc_transaction(tx_sk, ACCESS_CONTROL_ID, fn_name, fn_arg)
    }

    /// Query the access-control-contract directly without paying gas.
    pub fn query_access_control<A, R>(
        &mut self,
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
            .direct_call::<A, R>(ACCESS_CONTROL_ID, fn_name, fn_arg)
    }

    /// Query the token-contract directly without paying gas.
    pub fn query_token<A, R>(
        &mut self,
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
        self.session.direct_call::<A, R>(TOKEN_ID, fn_name, fn_arg)
    }
}
