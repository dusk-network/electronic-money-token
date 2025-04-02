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

use dusk_core::{
    abi::{self, ContractId},
    transfer::TRANSFER_CONTRACT,
};

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
        if let Err(err) = abi::call::<_, ()>(
            self.token_contract,
            "transfer",
            &(receiver, value),
        ) {
            panic!("Failed sending tokens: {err}");
        }

        self.balance -= value
    }

    /// Handles incoming token transfers from the token-contract.
    ///
    /// This function is called automatically by the token-contract's transfer
    /// function when this contract is the receiver of a transfer.
    fn token_received(&mut self, sender: Account, value: u64) {
        // Only accept transfers from the specific token-contract we're tracking
        if abi::caller().expect("Expected a contract as caller") == TOKEN_ID {
            let public_sender =
                abi::public_sender().expect("Expected a public sender");

            let call_stack = abi::callstack();
            // get the last caller in the callstack
            let emt_caller = *call_stack
                .iter()
                .rev()
                .next()
                .expect("Expected a caller in the callstack");

            match sender {
                Account::External(sender) => {
                    // the sender is the tx origin, because of that we also
                    // check the call stack and emt_caller to
                    // prevent any other contract from calling this function,
                    // trying to act on behalf of the tx origin
                    //
                    // Any other sender could not be verified with this function
                    // without additional arguments (signatures) provided.
                    assert_eq!(sender, public_sender);
                    assert_eq!(emt_caller, TRANSFER_CONTRACT);
                    assert_eq!(call_stack.len(), 2);
                }
                Account::Contract(sender) => {
                    // The sender is a contract. The sender is the contract that
                    // called the EMT contract
                    //
                    // Any other specified sender could not be verified without
                    // additional arguments provided.
                    if sender != emt_caller {
                        panic!("Unexpected sender: {sender}");
                    }
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
unsafe fn token_received(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(sender, value)| {
        STATE.token_received(sender, value)
    })
}

#[no_mangle]
unsafe fn tracked_balance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.tracked_balance())
}
