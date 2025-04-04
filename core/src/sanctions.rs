// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when an account is blocked.
pub const BLOCKED: &str = "Account is blocked";
/// Error message for when an account is frozen.
pub const FROZEN: &str = "Account is frozen";

/// Events for sanction transactions.
pub mod events {
    use super::{Account, Archive, CheckBytes, Deserialize, Serialize};

    /// Event emitted when an account status changes.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct AccountStatusEvent {
        /// The affected account.
        pub account: Account,
        /// The new status of the account.
        pub status: u64,
    }

    impl AccountStatusEvent {
        /// The topic of the blocked event.
        pub const BLOCKED_TOPIC: &'static str = "blocked";
        /// The topic of the unblocked event.
        pub const UNBLOCKED_TOPIC: &'static str = "unblocked";
        /// The topic of the frozen event.
        pub const FROZEN_TOPIC: &'static str = "frozen";
        /// The topic of the unfrozen event.
        pub const UNFROZEN_TOPIC: &'static str = "unfrozen";

        /// Create a new `AccountStatusEvent` for a blocked account.
        pub fn blocked(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 2,
            }
        }

        /// Create a new `AccountStatusEvent` for an unblocked account.
        pub fn unblocked(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 0,
            }
        }

        /// Create a new `AccountStatusEvent` for a frozen account.
        pub fn frozen(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 1,
            }
        }

        /// Create a new `AccountStatusEvent` for an unfrozen account.
        pub fn unfrozen(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 0,
            }
        }
    }

    /// Event emitted when tokens are frozen or unfrozen.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct TokenSanctionEvent {
        // note: rename to TokenStatusEvent?
        /// The affected account.
        pub account: Account,
        /// The amount of tokens frozen.
        pub amount: u64,
    }

    impl TokenSanctionEvent {
        /// The topic of the partial freeze event.
        pub const TOKENS_FROZEN: &'static str = "tokens_frozen";
        /// The topic of the partial unfreeze event.
        pub const TOKENS_UNFROZEN: &'static str = "tokens_unfrozen"; // TODO: do we wanna use snake case here?
    }
}
