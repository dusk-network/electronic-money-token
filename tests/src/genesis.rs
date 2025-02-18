use crate::common::utils::{account, chain_id};
use bytecheck::CheckBytes;
use dusk_core::abi::{ContractError, StandardBufSerializer};
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{PublicKey as AccountPublicKey, SecretKey as AccountSecretKey};
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::data::{ContractCall, TransactionData};
use dusk_core::transfer::moonlight::{AccountData, Transaction as MoonlightTransaction};
use dusk_core::transfer::{ContractToAccount, ContractToContract, Transaction, TRANSFER_CONTRACT};
use dusk_core::{dusk, LUX};
use dusk_vm::{execute, ExecutionConfig};
use dusk_vm::{CallReceipt, ContractData, Error as VMError, Session, VM};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

/*
TODOS:
- dusk_vm when used for integration tests, should have access to genesis contract bytecodes or another testing crate
-
*/

const ZERO_ADDRESS: ContractId = ContractId::from_bytes([0; CONTRACT_ID_BYTES]);
const GAS_LIMIT: u64 = 0x10000000;
const CHAIN_ID: u8 = 0x1;
const MOONLIGHT_GENESIS_VALUE: u64 = dusk(1_000.0);
const MOONLIGHT_GENESIS_NONCE: u64 = 0;
const NO_CONFIG: ExecutionConfig = ExecutionConfig::DEFAULT;

type Result<T, Error = VMError> = core::result::Result<T, Error>;

/// VM Sessions that behaves like a mainnet VM.
///
/// This means the underlying session cannot be called directly.
/// Any integration test against a contract needs to be crafted exactly like a transaction would be crafted for mainnet, which will go through the
/// genesis transfer contract, before reaching the non-genesis contract.
pub struct MainnetSession {
    session: Session,
}

impl MainnetSession {
    /// Passes the call to the underlying session with maximum gas limit.
    pub fn deploy<'a, A, D>(&mut self, bytecode: &[u8], deploy_data: D) -> Result<ContractId>
    where
        A: 'a + for<'b> Serialize<StandardBufSerializer<'b>>,
        D: Into<ContractData<'a, A>>,
    {
        self.session.deploy(bytecode, deploy_data, u64::MAX)
    }

    /// Directly calls the contract, circumventing the transfer contract.
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
        R::Archived: Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        self.session.call(contract, fn_name, fn_arg, u64::MAX)
    }

    pub fn icc_transaction<A, R>(
        &mut self,
        moonlight_sk: AccountSecretKey,
        contract: ContractId,
        fn_name: &str,
        fn_arg: Vec<u8>,
    ) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let contract_call = ContractCall {
            contract,
            fn_name: String::from(fn_name),
            fn_args: fn_arg.into(),
        };

        let moonlight_pk = AccountPublicKey::from(&moonlight_sk);

        let AccountData { nonce, .. } =
            account(&mut self.session, &moonlight_pk).expect("Getting the account should succeed");

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

        let receipt = execute(&mut self.session, &transaction, &NO_CONFIG);

        receipt
    }
}

impl MainnetSession {
    /// Instantiate the virtual machine with the transfer contract deployed and the stake contract deployed.
    /// One moonlight account owns some tokens.
    pub fn instantiate(moonlight_pk: &AccountPublicKey) -> Self {
        let vm = VM::ephemeral().expect("Creating VM should succeed");

        let mut session = VM::genesis_session(&vm, 1);

        // deploy transfer contract
        let transfer_contract = include_bytes!("../genesis-contracts/transfer_contract.wasm");

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
        let stake_contract = include_bytes!("../genesis-contracts/stake_contract.wasm");

        session
            .deploy(
                stake_contract,
                ContractData::builder()
                    .owner(ZERO_ADDRESS.to_bytes())
                    .contract_id(STAKE_CONTRACT),
                GAS_LIMIT,
            )
            .expect("Deploying the transfer contract should succeed");

        // insert genesis value to moonlight account
        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_account_balance",
                &(*moonlight_pk, MOONLIGHT_GENESIS_VALUE),
                GAS_LIMIT,
            )
            .expect("Inserting genesis account should succeed");

        // commit the first block, this sets the block height for all subsequent
        // operations to 1
        let base = session.commit().expect("Committing should succeed");
        // start a new session from that base-commit
        let mut session = vm
            .session(base, CHAIN_ID, 1)
            .expect("Instantiating new session should succeed");

        // the moonlight account is instantiated with the expected value
        let sender_account = account(&mut session, &moonlight_pk)
            .expect("Getting the sender account should succeed");
        assert_eq!(
            sender_account.balance, MOONLIGHT_GENESIS_VALUE,
            "The sender moonlight account should have its genesis value"
        );
        // the moonlight account's nonce is 0
        assert_eq!(sender_account.nonce, MOONLIGHT_GENESIS_NONCE);

        // chain-id is as expected
        let chain_id = chain_id(&mut session).expect("Getting the chain ID should succeed");
        assert_eq!(chain_id, CHAIN_ID, "the chain id should be as expected");

        Self { session }
    }
}
