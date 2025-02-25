// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use dusk_core::abi::{self, ContractId};

use ttoken_types::*;

struct TokenState {
    this_contract: ContractId,
    token_contract: ContractId,
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
    fn token_send(&mut self, transfer: Transfer) {
        if let Err(err) = abi::call::<_, ()>(self.token_contract, "transfer", &transfer) {
            panic!("Failed sending tokens: {err}");
        }

        if matches!(transfer.sender(), Account::Contract(x) if *x == self.this_contract) {
            self.balance -= transfer.value();
        }
    }

    fn token_received(&mut self, transfer: TransferInfo) {
        self.balance += transfer.value;
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
