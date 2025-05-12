// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::allowlist::{Address, Role};

pub use crate::token::events::OwnershipTransferred;

/// Event emitted when the allowlist is updated.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct UpdateAllowList {
    /// The updated entry.
    pub user: Address,
    /// The role of the updated user.
    pub role: Role,
}

impl UpdateAllowList {
    /// Event topic used a new user has been registered.
    pub const REGISTER: &'static str = "new_address_registered";
    /// Event topic used a user has been removed.
    pub const REMOVE: &'static str = "address_removed";
    /// Event topic used the role for a user has been updated.
    pub const UPDATE: &'static str = "role_updated";
}
