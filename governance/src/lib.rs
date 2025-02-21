// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The governance contract for electronic money tokens.

#![no_std]
// #![cfg_attr(target_family = "wasm", no_std)]
// #![cfg(target_family = "wasm")]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]

extern crate alloc;

// use dusk_core::abi;

pub mod error;
pub(crate) mod state;
pub use state::{Governance as GovernanceState, STATE};

/*
#[no_mangle]
unsafe fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(token_contract, owner, operator)| {
        STATE.init(token_contract, owner, operator)
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

/*
 * Access control functions
 */

#[no_mangle]
unsafe fn transfer_governance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_governance(arg))
}

#[no_mangle]
unsafe fn renounce_governance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.renounce_governance(arg))
}

#[no_mangle]
unsafe fn owner(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.owner())
}

#[no_mangle]
unsafe fn operator(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.operator())
}
*/

/*
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
 * Supply management functions
 */

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn burn(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.burn(arg))
}
*/
