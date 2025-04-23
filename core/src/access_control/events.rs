// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::string::String;
use alloc::vec::Vec;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey;

/// Event emitted when the token-contract is replaced.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct UpdateToken {
    /// The new token-contract.
    pub contract: ContractId,
}

impl UpdateToken {
    /// Event topic used when the token-contract has been replaced.
    pub const TOPIC: &'static str = "new_token-contract";
}

/// Event emitted when the owners or operators are updated.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UpdatePublicKeys {
    /// The new public keys stored in the access-control-state.
    pub pks: Vec<PublicKey>,
}

impl UpdatePublicKeys {
    /// Event topic used when the owners have been updated.
    pub const NEW_OWNERS: &'static str = "new_owners";
    /// Event topic used when the operators have been updated.
    pub const NEW_OPERATORS: &'static str = "new_operators";
}

/// Event emitted when a token-contract call has been added or updated.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UpdateTokenCall {
    /// The new token-contract call.
    pub call_name: String,
    /// The threshold of operator signatures needed to trigger the call.
    pub operator_signature_threshold: u8,
}

impl UpdateTokenCall {
    /// Event topic used when a token-contract call has been added or updated.
    pub const TOPIC: &'static str = "update_token-contract_call";
}
