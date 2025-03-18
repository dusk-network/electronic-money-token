// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::{Account, AccountInfo};

/// Error message for when an account is blocked.
pub const BLOCKED: &str = "Account is blocked";
/// Error message for when an account is frozen.
pub const FROZEN: &str = "Account is frozen";

/// Arguments for sanction transactions.
pub mod arguments {
    use super::*;

    /// Data used to sanction an account.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct Sanction {
        account: Account,
        sanction_type: u64,
    }

    impl Sanction {
        /// Create a new `Sanction` transaction for freezing an account.
        pub fn freeze_account(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                sanction_type: AccountInfo::FROZEN,
            }
        }

        /// Create a new `Sanction` transaction for blocking an account.
        pub fn block_account(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                sanction_type: AccountInfo::BLOCKED,
            }
        }

        /// Create a new `Unsanction` transaction for un-sanctioning an account.
        pub fn unsanction_account(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                sanction_type: AccountInfo::NO_STATUS,
            }
        }

        /// Get the account specified for this transaction.
        pub fn account(&self) -> &Account {
            &self.account
        }

        /// Get the sanction type specified for this transaction.
        pub fn sanction_type(&self) -> u64 {
            self.sanction_type
        }
    }
}

/// Events for sanction transactions.
pub mod events {
    use super::*;

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
}
