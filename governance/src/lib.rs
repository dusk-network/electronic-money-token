// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

// use alloc::collections::BTreeMap;
// use alloc::string::String;
// use alloc::vec::Vec;

// use dusk_core::abi::{self, ContractId, CONTRACT_ID_BYTES};
// use dusk_core::signatures::bls::{PublicKey, Signature};
// use ttoken_types::admin_management::arguments::PauseToggle;
// use ttoken_types::admin_management::events::PauseToggled;
// use ttoken_types::admin_management::PAUSED_MESSAGE;
// use ttoken_types::ownership::arguments::{
//     RenounceOwnership, TransferOwnership,
// };
// use ttoken_types::ownership::events::{
//     OwnerShipRenouncedEvent, OwnershipTransferredEvent,
// };
// use ttoken_types::ownership::{
//     EXPECT_CONTRACT, OWNER_NOT_FOUND, UNAUTHORIZED_CONTRACT,
//     UNAUTHORIZED_EXT_ACCOUNT,
// };
// use ttoken_types::sanctions::arguments::Sanction;
// use ttoken_types::sanctions::events::AccountStatusEvent;
// use ttoken_types::sanctions::{BLOCKED, FROZEN};
// use ttoken_types::supply_management::arguments::{Burn, Mint};
// use ttoken_types::supply_management::events::{BurnEvent, MintEvent};
// use ttoken_types::supply_management::SUPPLY_OVERFLOW;
// use ttoken_types::*;

pub mod error;
pub(crate) mod state;
pub use state::{GovernanceState, STATE};

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
    abi::wrap_call(arg_len, |_: ()| STATE.owner)
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
