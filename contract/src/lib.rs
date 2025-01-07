// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::abi;
use dusk_core::signatures::bls::Signature;
use ttoken_types::ownership::events::{OwnerShipRenouncedEvent, OwnershipTransferredEvent};
use ttoken_types::ownership::payloads::{RenounceOwnership, TransferOwnership};
use ttoken_types::ownership::{
    EXPECT_CONTRACT, OWNER_NOT_SET, UNAUTHORIZED_CONTRACT, UNAUTHORIZED_EXT_ACCOUNT,
};
use ttoken_types::*;

/// The state of the token contract.
struct TokenState {
    accounts: BTreeMap<Account, AccountInfo>,
    allowances: BTreeMap<Account, BTreeMap<Account, u64>>,
    supply: u64,

    // TODO: remove Option and find a way to set an owner through a const fn
    owner: Option<Account>,
}

impl TokenState {
    fn init(&mut self, accounts: Vec<(Account, u64)>, owner: Account) {
        for (account, balance) in accounts {
            let account = self.accounts.entry(account).or_insert(AccountInfo::EMPTY);
            account.balance += balance;
            self.supply += balance;
        }

        // Set the owner
        self.owner = Some(owner);

        // Always insert owner
        self.accounts
            .entry(self.owner())
            .or_insert(AccountInfo::EMPTY);
    }
}

static mut STATE: TokenState = TokenState {
    accounts: BTreeMap::new(),
    allowances: BTreeMap::new(),
    supply: 0,
    owner: None,
};

/// Access control implementation.
impl TokenState {
    fn owner(&self) -> Account {
        self.owner.clone().expect(OWNER_NOT_SET)
    }

    fn authorize_owner(&self, sig_msg: Vec<u8>, sig: Signature) {
        let owner = self.owner().clone();

        match owner {
            Account::External(pk) => {
                assert!(
                    abi::verify_bls(sig_msg, pk, sig),
                    "{}",
                    UNAUTHORIZED_EXT_ACCOUNT
                )
            }
            Account::Contract(contract_id) => assert!(
                abi::caller().expect(EXPECT_CONTRACT) == contract_id,
                "{}",
                UNAUTHORIZED_CONTRACT
            ),
        }
    }

    fn transfer_ownership(&mut self, transfer_owner: TransferOwnership) {
        let sig = *transfer_owner.signature();
        let sig_msg = transfer_owner.signature_message().to_vec();
        let prev_owner = self.owner().clone();

        let prev_owner_account = self
            .accounts
            .get_mut(&prev_owner)
            .expect("The account does not exist");

        if transfer_owner.nonce() != prev_owner_account.nonce + 1 {
            panic!("Nonces must be sequential");
        }

        match prev_owner {
            Account::External(pk) => {
                assert!(
                    abi::verify_bls(sig_msg, pk, sig),
                    "{}",
                    UNAUTHORIZED_EXT_ACCOUNT
                )
            }
            Account::Contract(contract_id) => assert!(
                abi::caller().expect(EXPECT_CONTRACT) == contract_id,
                "{}",
                UNAUTHORIZED_CONTRACT
            ),
        }

        prev_owner_account.nonce += 1;

        self.owner = Some(*transfer_owner.new_owner());
        // Always insert owner
        self.accounts
            .entry(*transfer_owner.new_owner())
            .or_insert(AccountInfo::EMPTY);

        abi::emit(
            OwnershipTransferredEvent::TOPIC,
            OwnershipTransferredEvent::new(prev_owner, self.owner()),
        );
    }

    fn renounce_ownership(&mut self, payload: RenounceOwnership) {
        let sig = *payload.signature();
        let sig_msg = payload.signature_message().to_vec();
        let owner = self.owner();

        let owner_account = self
            .accounts
            .get_mut(&owner)
            .expect("The account does not exist");

        if payload.nonce() != owner_account.nonce + 1 {
            panic!("Nonces must be sequential");
        }

        owner_account.nonce += 1;

        match owner {
            Account::External(pk) => {
                assert!(
                    abi::verify_bls(sig_msg, pk, sig),
                    "{}",
                    UNAUTHORIZED_EXT_ACCOUNT
                )
            }
            Account::Contract(contract_id) => assert_eq!(
                abi::caller().expect(EXPECT_CONTRACT),
                contract_id,
                "{}",
                UNAUTHORIZED_CONTRACT
            ),
        }

        self.owner = None;

        abi::emit(
            OwnerShipRenouncedEvent::TOPIC,
            OwnerShipRenouncedEvent::new(owner),
        );
    }
}

/// Basic token contract implementation.
impl TokenState {
    fn name(&self) -> String {
        String::from("Transparent Fungible Token Sample")
    }

    fn symbol(&self) -> String {
        String::from("TFTS")
    }

    fn decimals(&self) -> u8 {
        18
    }

    fn total_supply(&self) -> u64 {
        self.supply
    }

    fn account(&self, account: Account) -> AccountInfo {
        self.accounts
            .get(&account)
            .copied()
            .unwrap_or(AccountInfo::EMPTY)
    }

    fn allowance(&self, allowance: Allowance) -> u64 {
        match self.allowances.get(&allowance.owner) {
            Some(allowances) => allowances.get(&allowance.spender).copied().unwrap_or(0),
            None => 0,
        }
    }

    fn transfer(&mut self, transfer: Transfer) {
        let from_key = *transfer.from();
        let from = Account::External(from_key);

        let from_account = self
            .accounts
            .get_mut(&from)
            .expect("The account has no tokens to transfer");

        let value = transfer.value();
        if from_account.balance < value {
            panic!("The account doesn't have enough tokens");
        }

        if transfer.nonce() != from_account.nonce + 1 {
            panic!("Nonces must be sequential");
        }

        from_account.balance -= value;
        from_account.nonce += 1;

        let sig = *transfer.signature();
        let sig_msg = transfer.signature_message().to_vec();
        if !abi::verify_bls(sig_msg, from_key, sig) {
            panic!("Invalid signature");
        }

        let to = *transfer.to();
        let to_account = self.accounts.entry(to).or_insert(AccountInfo::EMPTY);

        to_account.balance += value;

        abi::emit(
            "transfer",
            TransferEvent {
                owner: from,
                spender: None,
                to,
                value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(contract) = to {
            if let Err(err) =
                abi::call::<_, ()>(contract, "token_received", &TransferInfo { from, value })
            {
                panic!("Failed calling `token_received` on the receiving contract: {err}");
            }
        }
    }

    fn transfer_from(&mut self, transfer: TransferFrom) {
        let spender_key = *transfer.spender();
        let spender = Account::External(spender_key);

        let spender_account = self.accounts.entry(spender).or_insert(AccountInfo::EMPTY);
        if transfer.nonce() != spender_account.nonce + 1 {
            panic!("Nonces must be sequential");
        }

        spender_account.nonce += 1;

        let sig = *transfer.signature();
        let sig_msg = transfer.signature_message().to_vec();
        if !abi::verify_bls(sig_msg, spender_key, sig) {
            panic!("Invalid signature");
        }

        let owner = *transfer.owner();

        let allowance = self
            .allowances
            .get_mut(&owner)
            .expect("The account has no allowances")
            .get_mut(&spender)
            .expect("The spender is not allowed to use the account");

        let value = transfer.value();
        if value > *allowance {
            panic!("The spender can't spent the defined amount");
        }

        let owner_account = self
            .accounts
            .get_mut(&owner)
            .expect("The account has no tokens to transfer");

        if owner_account.balance < value {
            panic!("The account doesn't have enough tokens");
        }

        *allowance -= value;
        owner_account.balance -= value;

        let to = *transfer.to();
        let to_account = self.accounts.entry(to).or_insert(AccountInfo::EMPTY);

        to_account.balance += value;

        abi::emit(
            "transfer",
            TransferEvent {
                owner,
                spender: Some(spender),
                to,
                value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(contract) = to {
            if let Err(err) = abi::call::<_, ()>(
                contract,
                "token_received",
                &TransferInfo { from: owner, value },
            ) {
                panic!("Failed calling `token_received` on the receiving contract: {err}");
            }
        }
    }

    fn transfer_from_contract(&mut self, transfer: TransferFromContract) {
        let contract = abi::caller().expect("Must be called by a contract");
        let contract = Account::Contract(contract);

        let contract_account = self
            .accounts
            .get_mut(&contract)
            .expect("Contract has no tokens to transfer");

        if contract_account.balance < transfer.value {
            panic!("The contract doesn't have enough tokens");
        }

        contract_account.balance -= transfer.value;

        let to_account = self
            .accounts
            .entry(transfer.to)
            .or_insert(AccountInfo::EMPTY);

        to_account.balance += transfer.value;

        abi::emit(
            "transfer",
            TransferEvent {
                owner: contract,
                spender: None,
                to: transfer.to,
                value: transfer.value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(to_contract) = transfer.to {
            if let Err(err) = abi::call::<_, ()>(
                to_contract,
                "token_received",
                &TransferInfo {
                    from: contract,
                    value: transfer.value,
                },
            ) {
                panic!("Failed calling `token_received` on the receiving contract: {err}");
            }
        }
    }

    fn approve(&mut self, approve: Approve) {
        let owner_key = *approve.owner();
        let owner = Account::External(owner_key);

        let owner_account = self.accounts.entry(owner).or_insert(AccountInfo::EMPTY);
        if approve.nonce() != owner_account.nonce + 1 {
            panic!("Nonces must be sequential");
        }

        owner_account.nonce += 1;

        let sig = *approve.signature();
        let sig_msg = approve.signature_message().to_vec();
        if !abi::verify_bls(sig_msg, owner_key, sig) {
            panic!("Invalid signature");
        }

        let spender = *approve.spender();

        let allowances = self.allowances.entry(owner).or_insert(BTreeMap::new());

        let value = approve.value();
        allowances.insert(spender, value);

        abi::emit(
            "approve",
            ApproveEvent {
                owner,
                spender,
                value,
            },
        );
    }
}

#[no_mangle]
unsafe fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(initial_accounts, owner)| {
        STATE.init(initial_accounts, owner)
    })
}

#[no_mangle]
unsafe fn name(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.name())
}

#[no_mangle]
unsafe fn symbol(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.symbol())
}

#[no_mangle]
unsafe fn decimals(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.decimals())
}

#[no_mangle]
unsafe fn total_supply(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.total_supply())
}

#[no_mangle]
unsafe fn account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.account(arg))
}

#[no_mangle]
unsafe fn allowance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.allowance(arg))
}

#[no_mangle]
unsafe fn transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer(arg))
}

#[no_mangle]
unsafe fn transfer_from(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from(arg))
}

#[no_mangle]
unsafe fn transfer_from_contract(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from_contract(arg))
}

#[no_mangle]
unsafe fn approve(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.approve(arg))
}

/*
 * Access control functions
 */

#[no_mangle]
unsafe fn transfer_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_ownership(arg))
}

#[no_mangle]
unsafe fn renounce_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.renounce_ownership(arg))
}

#[no_mangle]
unsafe fn owner(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.owner())
}
