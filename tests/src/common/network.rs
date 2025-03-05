// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_core::abi::{ContractError, StandardBufSerializer};
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::{Transaction, TRANSFER_CONTRACT};
use dusk_core::{dusk, LUX};
use dusk_vm::{execute, ExecutionConfig};
use dusk_vm::{CallReceipt, ContractData, Error as VMError, Session, VM};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use crate::common::utils::{account, chain_id};

const ZERO_ADDRESS: ContractId = ContractId::from_bytes([0; CONTRACT_ID_BYTES]);
const GAS_LIMIT: u64 = 0x10000000;
const CHAIN_ID: u8 = 0x1;
const MOONLIGHT_GENESIS_VALUE: u64 = dusk(1_000.0);
const MOONLIGHT_GENESIS_NONCE: u64 = 0;
const NO_CONFIG: ExecutionConfig = ExecutionConfig::DEFAULT;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

/// VM Sessions that behaves like a mainnet VM.
///
/// Calls go as tx through the genesis transfer contract,
/// before reaching the non-genesis contract.
pub struct NetworkSession {
    session: Session,
    config: ExecutionConfig,
}

impl NetworkSession {
    /// Passes the call to deploy bytecode of a contract to the
    /// underlying session with maximum gas limit.
    pub fn deploy<'a, A, D>(
        &mut self,
        bytecode: &[u8],
        deploy_data: D,
    ) -> Result<ContractId>
    where
        A: 'a + for<'b> Serialize<StandardBufSerializer<'b>>,
        D: Into<ContractData<'a, A>>,
    {
        self.session.deploy(bytecode, deploy_data, u64::MAX)
    }

    /// Directly calls the contract, circumventing the transfer contract and
    /// (among other things) also any gas-payment.
    pub fn direct_call<A, R>(
        &mut self,
        contract: ContractId,
        fn_name: &str,
        fn_arg: &A,
    ) -> Result<CallReceipt<R>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.call(contract, fn_name, fn_arg, u64::MAX)
    }

    /// Calls the contract trough the transfer-contract which is the standard
    /// way any contract is called on the network. The gas is paid using funds
    /// owned by the `moonlight_sk`.
    pub fn icc_transaction(
        &mut self,
        moonlight_sk: AccountSecretKey,
        contract: ContractId,
        fn_name: &str,
        fn_arg: Vec<u8>,
    ) -> CallReceipt<Result<Vec<u8>, ContractError>> {
        let contract_call = ContractCall {
            contract,
            fn_name: String::from(fn_name),
            fn_args: fn_arg,
        };

        let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

        let AccountData { nonce, .. } =
            account(&mut self.session, &moonlight_pk)
                .expect("Getting the account should succeed");

        let transaction = Transaction::moonlight(
            &moonlight_sk,
            None,
            0,
            0,
            GAS_LIMIT,
            LUX,
            nonce + 1,
            CHAIN_ID,
            Some(contract_call),
        )
        .expect("Creating moonlight transaction should succeed");

        // TODO: this function could return the generic R type of the .call
        // function used in it.
        execute(&mut self.session, &transaction, &self.config).unwrap_or_else(
            |e| panic!("Executing the transaction should succeed: {:?}", e),
        )
        // TODO: Return CallReceipt<R>?
    }
}

impl NetworkSession {
    /// Instantiate the virtual machine with both the transfer and stake
    /// contract deployed. The given public accounts own the specified
    /// amount of DUSK token in order to pay for transactions like deploying
    /// or executing contracts.
    pub fn instantiate(pks_to_fund: Vec<(&AccountPublicKey, u64)>) -> Self {
        let vm = VM::ephemeral().expect("Creating VM should succeed");

        let mut session = VM::genesis_session(&vm, 1);

        // deploy transfer contract
        let transfer_contract =
            include_bytes!("../../genesis-contracts/transfer_contract.wasm");

        session
            .deploy(
                transfer_contract,
                ContractData::builder()
                    .owner(ZERO_ADDRESS.to_bytes())
                    .contract_id(TRANSFER_CONTRACT),
                GAS_LIMIT,
            )
            .expect("Deploying the transfer contract should succeed");

        // deploy stake contract
        let stake_contract =
            include_bytes!("../../genesis-contracts/stake_contract.wasm");

        session
            .deploy(
                stake_contract,
                ContractData::builder()
                    .owner(ZERO_ADDRESS.to_bytes())
                    .contract_id(STAKE_CONTRACT),
                GAS_LIMIT,
            )
            .expect("Deploying the transfer contract should succeed");

        // fund public keys with DUSK
        for (&pk_to_fund, val) in &pks_to_fund {
            session
                .call::<_, ()>(
                    TRANSFER_CONTRACT,
                    "add_account_balance",
                    &(pk_to_fund, *val),
                    GAS_LIMIT,
                )
                .expect("Add account balance should succeed");
        }

        // commit the first block, this sets the block height for all subsequent
        // operations to 1
        let base = session.commit().expect("Committing should succeed");

        // start a new session from that base-commit
        let mut session = vm
            .session(base, CHAIN_ID, 1)
            .expect("Instantiating new session should succeed");

        // assert that the accounts are instantiated with their expected value
        for (pk, value) in pks_to_fund {
            let account = account(&mut session, pk)
                .expect("Getting the account should succeed");
            assert_eq!(
                account.balance, value,
                "The account should own the specified value"
            );
            // the account's nonce is 0
            assert_eq!(account.nonce, 0);
        }

        // chain-id is as expected
        let chain_id = chain_id(&mut session)
            .expect("Getting the chain ID should succeed");
        assert_eq!(chain_id, CHAIN_ID, "the chain id should be as expected");

        // set config
        let mut config = NO_CONFIG;
        config.with_public_sender = true;

        Self { session, config }
    }
}
