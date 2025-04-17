// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Test contract that can hold tokens and keeps track of them inside its own
//! state.
//!
//! Anyone can send tokens to this contract, and anyone can call the
//! `token_send` function to send tokens from this contract to any other
//! receiver.

#![no_std]

extern crate alloc;

use dusk_core::abi::{self, ContractId};
use dusk_core::transfer::data::ContractCall;
use dusk_core::transfer::TRANSFER_CONTRACT;

use emt_core::*;

// The contract ID of the token-contract
const TOKEN_ID: ContractId = ContractId::from_bytes([1; 32]);

struct TokenState {
    this_contract: ContractId,
    token_contract: ContractId,
    /// Tracks the holder contracts balance of TOKEN_ID tokens
    balance: u64,
}

impl TokenState {
    fn init(&mut self, token_contract: ContractId, balance: u64) {
        self.this_contract = abi::self_id();
        self.token_contract = token_contract;
        self.balance = balance;
    }
}

static mut STATE: TokenState = TokenState {
    this_contract: ContractId::from_bytes([0u8; 32]),
    token_contract: ContractId::from_bytes([0u8; 32]),
    balance: 0,
};

impl TokenState {
    /// Can be called by anyone to make this contract send tokens to another
    /// account
    fn token_send(&mut self, receiver: Account, value: u64) {
        self.balance -= value;

        if let Err(err) = abi::call::<_, ()>(
            self.token_contract,
            "transfer",
            &(receiver, value),
        ) {
            panic!("Failed sending tokens: {err}");
        }
    }

    /// Can be called by anyone to make this contract send tokens to a
    /// contract & call a specific function on that contract.
    fn token_send_and_call(&mut self, value: u64, contract_call: ContractCall) {
        // Note: Subtract the balance before calling the function.

        // Otherwise, we’d need to add checks for self-transfers, to not update
        // the balance. Without any of this, a self-transfer would
        // incorrectly increase the perceived balance, causing the
        // `token_received` function to (luckily) fail due to a balance
        // mismatch.

        self.balance -= value;

        if let Err(err) = abi::call::<_, ()>(
            self.token_contract,
            "transfer_and_call",
            &(value, contract_call),
        ) {
            panic!("Failed sending tokens: {err}");
        }
    }

    /// Handles incoming token transfers from the token-contract.
    ///
    /// This function is called automatically by the token-contract's transfer
    /// function when this contract is the receiver of a transfer.
    fn token_received(&mut self, sender: Account, value: u64) {
        // Only accept transfers from the specific token-contract we're tracking
        if abi::caller().expect("Expected a contract as caller") == TOKEN_ID {
            let resolved_sender = token_sender();
            // make sure the given sender is the same as the one we resolved
            assert_eq!(resolved_sender, sender, "Sender mismatch");

            // Additional explanatory assertions. The logic in token_sender()
            // is enough to know who the sender is.

            let call_stack = abi::callstack();
            // get the TOKEN_ID caller in the callstack
            let emt_caller = *call_stack
                .iter()
                .nth(1)
                .expect("Expected a caller in the callstack");

            match resolved_sender {
                Account::External(sender) => {
                    // the sender is an external account, so we assert:
                    // - the caller of the EMT has to be the TRANSFER_CONTRACT
                    // - the call stack length has to be 2
                    // - the sender of the transaction has to be the public
                    //   sender
                    assert_eq!(sender, abi::public_sender().unwrap());
                    assert_eq!(
                        emt_caller, TRANSFER_CONTRACT,
                        "Expected the caller to be the transfer contract"
                    );
                    assert_eq!(call_stack.len(), 2);
                }
                Account::Contract(sender) => {
                    // The sender is a contract, so the sender is the contract
                    // that called the EMT contract. We assert:
                    // - the sender has to tbe the second last caller in the
                    //   call stack
                    // - the call stack length is variable, but has to be > 2
                    assert_eq!(
                        sender, emt_caller,
                        "Expected the sender to be the caller"
                    );
                    assert!(call_stack.len() > 2);
                }
            }

            self.balance += value;

            // Check if self.balance now corresponds to the balance in the
            // token-contract
            match abi::call::<_, AccountInfo>(
                TOKEN_ID,
                "account",
                &Account::Contract(self.this_contract),
            ) {
                Ok(acc_info) => {
                    assert!(
                        self.balance == acc_info.balance,
                        "Balance mismatch: {0} != {1}",
                        self.balance,
                        acc_info.balance
                    );
                }
                Err(err) => panic!(
                    "Failed to get account info from token-contract: {err}"
                ),
            }
        }
    }

    fn tracked_balance(&self) -> u64 {
        self.balance
    }
}

#[no_mangle]
unsafe fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(token_contract, balance)| {
        STATE.init(token_contract, balance)
    })
}

#[no_mangle]
unsafe fn token_send(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(receiver, value)| {
        STATE.token_send(receiver, value)
    })
}

#[no_mangle]
unsafe fn token_send_and_call(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(value, contract_call)| {
        STATE.token_send_and_call(value, contract_call)
    })
}

#[no_mangle]
unsafe fn token_received(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(sender, value)| {
        STATE.token_received(sender, value)
    })
}

#[no_mangle]
unsafe fn tracked_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.tracked_balance())
}

/// Determines and returns the sender of the current token transfer.
///
/// If the sender is an external account, return the transaction origin.
/// If the sender is a contract, return the contract that called the token
/// contract.
///
/// # Returns
///
/// An `Account` representing the token sender.
///
/// # Panics
///
/// - If no public sender is available in the case of an external account.
/// - If no caller can be determined (impossible case)
fn token_sender() -> Account {
    // get the last actual caller in the callstack
    // abi::caller() would return the token contract itself, since
    // transfer_and_call calls the contract.
    let emt_caller = *abi::callstack()
        .iter()
        .nth(1)
        .expect("Expected a caller behind the EMT caller in the callstack");

    // Identifies the sender by checking the call stack and transaction origin:
    // - For direct external account transactions (call stack length = 2),
    //   returns the transaction origin
    // - For non-protocol contracts that call the token (call stack length > 2),
    //   returns the immediate calling contract "behind" the EMT contract
    if abi::callstack().len() == 2 {
        // This also implies, that the call directly originates via the protocol
        // transfer contract i.e., the caller of the EMT is the transfer
        // contract
        Account::External(abi::public_sender().expect(SHIELDED_NOT_SUPPORTED))
    } else {
        Account::Contract(emt_caller)
    }
}
