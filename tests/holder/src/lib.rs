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
    fn token_send(&mut self, transfer: Transfer) {
        if let Err(err) =
            abi::call::<_, ()>(self.token_contract, "transfer", &transfer)
        {
            panic!("Failed sending tokens: {err}");
        }

        self.balance -= transfer.value();
    }

    /// Handles incoming token transfers from the token-contract.
    ///
    /// This function is called automatically by the token-contract's transfer
    /// function when this contract is the receiver of a transfer.
    fn token_received(&mut self, transfer: TransferInfo) {
        // Only accept transfers from the specific token-contract we're tracking
        if abi::caller().expect("Expected a contract as caller") == TOKEN_ID {
            self.balance += transfer.value;
        } else {
            panic!("Only the {TOKEN_ID} contract can call this function");
        }
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
    abi::wrap_call(arg_len, |arg| STATE.token_send(arg))
}

#[no_mangle]
unsafe fn token_received(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.token_received(arg))
}
