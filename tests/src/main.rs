use dusk_core::abi::{ContractId, StandardBufSerializer};
use dusk_core::signatures::bls::{PublicKey, SecretKey};
use dusk_vm::{CallReceipt, ContractData, Error as VMError, Session, VM};

use bytecheck::CheckBytes;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

use rand::rngs::StdRng;
use rand::SeedableRng;

use ttoken_types::*;

const TOKEN_BYTECODE: &[u8] = include_bytes!("../../build/ttoken_contract.wasm");
const HOLDER_BYTECODE: &[u8] = include_bytes!("../../build/ttoken_test_contract.wasm");

const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);
const HOLDER_ID: ContractId = ContractId::from_bytes([2; 32]);

const INITIAL_BALANCE: u64 = 1000;
const INITIAL_HOLDER_BALANCE: u64 = 1000;

const OWNER: [u8; 64] = [0u8; 64];

type Result<T, Error = VMError> = std::result::Result<T, Error>;

struct ContractSession {
    deploy_pk: PublicKey,
    deploy_sk: SecretKey,
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

        session
            .deploy(
                TOKEN_BYTECODE,
                ContractData::builder()
                    .owner(OWNER)
                    .init_arg(&vec![
                        (deploy_account, INITIAL_BALANCE),
                        (holder_account, INITIAL_HOLDER_BALANCE),
                    ])
                    .contract_id(TOKEN_ID),
                u64::MAX,
            )
            .expect("Deploying the token contract should succeed");

        session
            .deploy(
                HOLDER_BYTECODE,
                ContractData::builder()
                    .owner(OWNER)
                    .init_arg(&(TOKEN_ID, INITIAL_HOLDER_BALANCE))
                    .contract_id(HOLDER_ID),
                u64::MAX,
            )
            .expect("Deploying the holder contract should succeed");

        Self {
            deploy_sk,
            deploy_pk,
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
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE / 2;

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

    let transfer = Transfer::new(&session.deploy_sk, pk, TRANSFERRED_AMOUNT, 1);
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
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE / 2;

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

    let transfer = Transfer::new(&session.deploy_sk, HOLDER_ID, TRANSFERRED_AMOUNT, 1);
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
    const TRANSFERRED_AMOUNT: u64 = INITIAL_BALANCE / 2;

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

    let transfer = TransferFromContract {
        to: Account::External(session.deploy_pk()),
        from: None,
        value: TRANSFERRED_AMOUNT,
    };
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
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE / 2;

    let mut session = ContractSession::new();

    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let sk = SecretKey::random(&mut rng);
    let pk = PublicKey::from(&sk);

    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        0,
        "The account should not be allowed to spend tokens from the deployed account"
    );

    let approve = Approve::new(&session.deploy_sk, pk, APPROVED_AMOUNT, 1);
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
    const APPROVED_AMOUNT: u64 = INITIAL_BALANCE / 2;
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

    let approve = Approve::new(&session.deploy_sk, pk, APPROVED_AMOUNT, 1);
    session
        .call_token::<_, ()>("approve", &approve)
        .expect("Approving should succeed");

    assert_eq!(
        session.allowance(session.deploy_pk(), pk),
        APPROVED_AMOUNT,
        "The account should be allowed to spend tokens from the deployed account"
    );

    let transfer_from = TransferFrom::new(&sk, session.deploy_pk(), pk, TRANSFERRED_AMOUNT, 1);
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

fn main() {
    unreachable!("`main` should never run for this crate");
}
