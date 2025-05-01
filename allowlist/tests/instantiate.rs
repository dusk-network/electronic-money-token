// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::LazyLock;

use dusk_core::abi::ContractError;
use dusk_core::abi::{
    ContractId,
    // StandardBufSerializer
};
use dusk_core::dusk;
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_vm::{CallReceipt, ContractData, Error as VMError};

// use bytecheck::CheckBytes;
//
// use rkyv::validation::validators::DefaultValidator;
// use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use emt_core::allowlist::{Address, Role};
use emt_core::Account;

use emt_tests::network::NetworkSession;

const ALLOWLIST_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/emt_allowlist.wasm"
);
// const TOKEN_BYTECODE: &[u8] = include_bytes!(
//     "../../target/wasm64-unknown-unknown/release/emt_token.wasm"
// );
const HOLDER_BYTECODE: &[u8] = include_bytes!(
    "../../target/wasm64-unknown-unknown/release/emt_holder_contract.wasm"
);

const DEPLOYER: [u8; 64] = [0u8; 64];

pub const ALLOWLIST_ID: ContractId = ContractId::from_bytes([1; 32]);
pub const TOKEN_ID: ContractId = ContractId::from_bytes([2; 32]);
pub const HOLDER_ID: ContractId = ContractId::from_bytes([3; 32]);

pub const INITIAL_DUSK_BALANCE: u64 = dusk(1_000.0);
pub const INITIAL_EMT_BALANCE: u64 = 1000;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

pub struct TestSession {
    session: NetworkSession,
}

impl TestSession {
    // first test key-pair, funded with some DUSK to be able to pay for gas
    pub const SK_0: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0x5EAF00D);
        AccountSecretKey::random(&mut rng)
    });
    pub const PK_0: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::SK_0));
    // TODO: change once From<bls-pk> is implemented
    pub const ADDRESS_0: LazyLock<Address> =
        LazyLock::new(|| Address::from(&[1u8; 32]));
    // TODO: change once From<&str> is implemented
    pub const ROLE_0: LazyLock<Role> = LazyLock::new(|| Role::from(&[2u8; 32]));

    // second test key-pair, funded with some DUSK to be able to pay for gas
    pub const SK_1: LazyLock<AccountSecretKey> = LazyLock::new(|| {
        let mut rng = StdRng::seed_from_u64(0xF0CACC1A);
        AccountSecretKey::random(&mut rng)
    });
    pub const PK_1: LazyLock<AccountPublicKey> =
        LazyLock::new(|| AccountPublicKey::from(&*Self::SK_1));
    // TODO: change once From<bls-pk> is implemented
    pub const ADDRESS_1: LazyLock<Address> =
        LazyLock::new(|| Address::from(&[3u8; 32]));
    // TODO: change once From<&str> is implemented
    pub const ROLE_1: LazyLock<Role> = LazyLock::new(|| Role::from(&[4u8; 32]));
}

impl TestSession {
    pub fn new() -> Self {
        // deploy a session with transfer & stake contract deployed
        // pass a list of accounts to fund
        let mut network_session = NetworkSession::instantiate(vec![
            (&*Self::PK_0, INITIAL_DUSK_BALANCE),
            (&*Self::PK_1, INITIAL_DUSK_BALANCE),
        ]);

        // deploy the allowlist contract
        network_session
            .deploy(
                ALLOWLIST_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(
                        vec![
                            (*Self::ADDRESS_0, *Self::ROLE_0),
                            (*Self::ADDRESS_1, *Self::ROLE_1),
                        ],
                        // set pk_0 as contract ownership
                        Account::from(*Self::PK_0),
                    ))
                    .contract_id(ALLOWLIST_ID),
            )
            .expect("Deploying the allowlist-contract should succeed");

        // deploy the holder contract
        network_session
            .deploy(
                HOLDER_BYTECODE,
                ContractData::builder()
                    .owner(DEPLOYER)
                    .init_arg(&(TOKEN_ID, 0u64))
                    .contract_id(HOLDER_ID),
            )
            .expect("Deploying the holder contract should succeed");

        Self {
            session: network_session,
        }
    }

    //
    // allowlist-contract functionality
    //

    /// call `is_allowed(user)` on the allowlist-contract.
    pub fn allowlist_is_allowed(
        &mut self,
        user: &Address,
    ) -> Result<CallReceipt<bool>, ContractError> {
        self.session.direct_call(ALLOWLIST_ID, "is_allowed", user)
    }

    /// call `has_role(user)` on the allowlist-contract.
    pub fn allowlist_has_role(
        &mut self,
        user: &Address,
    ) -> Result<CallReceipt<Option<Role>>, ContractError> {
        self.session.direct_call(ALLOWLIST_ID, "has_role", user)
    }

    /// call `register(user, role)` on the allowlist-contract
    pub fn allowlist_register(
        &mut self,
        tx_sk: &AccountSecretKey,
        user: Address,
        role: Role,
    ) -> Result<CallReceipt<()>, ContractError> {
        self.session.icc_transaction(
            tx_sk,
            ALLOWLIST_ID,
            "register",
            &(user, role),
        )
    }

    /// call `update(user, role)` on the allowlist-contract
    pub fn allowlist_update(
        &mut self,
        tx_sk: &AccountSecretKey,
        user: Address,
        role: Role,
    ) -> Result<CallReceipt<()>, ContractError> {
        self.session.icc_transaction(
            tx_sk,
            ALLOWLIST_ID,
            "update",
            &(user, role),
        )
    }

    /// call `remove(user)` on the allowlist-contract
    pub fn allowlist_remove(
        &mut self,
        tx_sk: &AccountSecretKey,
        user: &Address,
    ) -> Result<CallReceipt<()>, ContractError> {
        self.session
            .icc_transaction(tx_sk, ALLOWLIST_ID, "remove", user)
    }

    /// call `ownership()` on the allowlist-contract.
    pub fn allowlist_ownership(
        &mut self,
    ) -> Result<CallReceipt<Account>, ContractError> {
        self.session.direct_call(ALLOWLIST_ID, "ownership", &())
    }

    /// call `transfer_ownership(new_ownership)` on the allowlist-contract
    pub fn allowlist_transfer_ownership(
        &mut self,
        tx_sk: &AccountSecretKey,
        new_ownership: &Account,
    ) -> Result<CallReceipt<()>, ContractError> {
        self.session.icc_transaction(
            tx_sk,
            ALLOWLIST_ID,
            "transfer_ownership",
            new_ownership,
        )
    }

    /// call `renounce_ownership()` on the allowlist-contract
    pub fn allowlist_renounce_ownership(
        &mut self,
        tx_sk: &AccountSecretKey,
    ) -> Result<CallReceipt<()>, ContractError> {
        self.session.icc_transaction(
            tx_sk,
            ALLOWLIST_ID,
            "renounce_ownership",
            &(),
        )
    }
}
