// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// The error messages given by the allowlist-contract.

pub use crate::access_control::error::ALLREADY_INITIALIZED;
pub use crate::token::error::UNAUTHORIZED_ACCOUNT;

/// Error message given when someone tries to insert a user's address that is
/// already stored.
pub const DUPLICATE_USER: &str = "The user's address is already registered";

/// Error message given when the given address doesn't exist in the state.
pub const ADDRESS_NOT_FOUND: &str = "The given address doesn't exist";
