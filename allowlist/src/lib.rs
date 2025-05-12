// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(target_family = "wasm")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]
#![no_std]

// #[cfg(target_family = "wasm")]
extern crate alloc;

// #[cfg(target_family = "wasm")]
pub(crate) mod state;

// #[cfg(target_family = "wasm")]
// mod wasm {

use dusk_core::abi;
use emt_core::allowlist::Address;

use crate::state::STATE;

#[no_mangle]
unsafe extern "C" fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(allowed, ownership)| {
        STATE.init(allowed, ownership);
    })
}

/*
 * Basic functionality of the allowlist-contract
 */

#[no_mangle]
unsafe extern "C" fn is_allowed(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |user: Address| STATE.is_allowed(&user))
}

#[no_mangle]
unsafe extern "C" fn has_role(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |user: Address| STATE.has_role(&user))
}

/*
 * Functions only allowed to be executed by the registered ownership
 * contract.
 */

#[no_mangle]
unsafe extern "C" fn register(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(user, role)| STATE.register(user, role))
}

#[no_mangle]
unsafe extern "C" fn update(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(user, role)| STATE.update(user, role))
}

#[no_mangle]
unsafe extern "C" fn remove(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |user| STATE.remove(user))
}

/*
 * Access control management functions
 */

#[no_mangle]
unsafe extern "C" fn transfer_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |new_ownership| {
        STATE.transfer_ownership(new_ownership);
    })
}

#[no_mangle]
unsafe extern "C" fn renounce_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.renounce_ownership())
}

#[no_mangle]
unsafe extern "C" fn ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.ownership())
}
// }
