// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Error messages for when an account doesn't have enough tokens to perform the
/// desired operation.
pub const BALANCE_TOO_LOW: &str = "The account doesn't have enough tokens";

/// Error message for when the account is not found in the contract.
pub const ACCOUNT_NOT_FOUND: &str = "The account does not exist";

/// Shielded transactions are not supported.
pub const SHIELDED_NOT_SUPPORTED: &str =
    "Shielded transactions are not supported";

/// Error message for when the admin account is not found in the contract.
pub const GOVERNANCE_NOT_FOUND: &str = "The governance does not exist";

/// Error message for when the governance is not authorized i.e., wrong
/// `public_sender` value.
pub const UNAUTHORIZED_ACCOUNT: &str = "Unauthorized account";

/// Error messages for overflow when minting tokens.
pub const SUPPLY_OVERFLOW: &str = "Supply overflow";

/// Error message for when an account is blocked.
pub const BLOCKED: &str = "Account is blocked";

/// Error message for when an account is frozen.
pub const FROZEN: &str = "Account is frozen";

/// Error message for when the contract is paused.
pub const PAUSED_MESSAGE: &str = "Contract is paused";
