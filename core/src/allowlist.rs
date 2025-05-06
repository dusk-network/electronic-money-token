// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error messages given by allowlist-contract.
pub mod error;
/// Events emitted by the allowlist-contract.
pub mod events;

/// The address of a user account.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    PartialOrd,
    Ord,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Address([u8; 32]);

impl From<&Account> for Address {
    fn from(account: &Account) -> Self {
        let _bytes = account.to_bytes();
        // TODO: create 32 byte hash from bytes
        unimplemented!()
    }
}

/// The role registered for a user.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Role([u8; 32]);

impl From<&str> for Role {
    fn from(str: &str) -> Self {
        let _bytes = str.bytes();
        // TODO: create 32 byte hash from bytes
        unimplemented!()
    }
}

//
// TODO: these are useful for tests but can be removed once From<bls-pk> is
// implemented
//

impl From<&[u8; 32]> for Address {
    fn from(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }
}

impl From<&[u8; 32]> for Role {
    fn from(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }
}
