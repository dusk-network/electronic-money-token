// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Event emitted when tokens are transferred from one account to another.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Transfer {
    /// The account tokens are transferred from.
    pub sender: Account,
    /// The account spending the tokens, set if `transfer_from` is used.
    pub spender: Option<Account>,
    /// The account receiving the tokens.
    pub receiver: Account,
    /// The value transferred.
    pub value: u64,
}

impl Transfer {
    /// Event topic used when a normal transfer is made.
    pub const TRANSFER_TOPIC: &'static str = "transfer";
    /// Event topic used when a forced transfer is made.
    pub const FORCE_TRANSFER_TOPIC: &'static str = "force_transfer";
    /// Event topic used when new tokens are minted.
    pub const MINT_TOPIC: &'static str = "mint";
    /// Event topic used when tokens are burned.
    pub const BURN_TOPIC: &'static str = "burn";
}

/// Event emitted when a spender is approved on an account.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Approve {
    /// The account allowing the transfer.
    pub sender: Account,
    /// The allowed spender.
    pub spender: Account,
    /// The value `spender` is allowed to spend.
    pub value: u64,
}

impl Approve {
    /// Event topic used when a spender is approved.
    pub const APPROVE_TOPIC: &'static str = "approve";
}

/// Event emitted when a contract is paused or unpaused.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct PauseToggled {
    /// State of the pause in the contract after the function call.
    pub paused: bool,
}

impl PauseToggled {
    /// The topic of the event.
    pub const TOPIC: &'static str = "pause_toggled";
}

/// Event emitted when the governance of a contract is transferred.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct GovernanceTransferred {
    /// The previous governance of the contract.
    pub previous_governance: Account,
    /// The new governance of the contract.
    pub new_governance: Account,
}

impl GovernanceTransferred {
    /// Event Topic for transferring the governance.
    pub const GOVERNANCE_TRANSFERRED: &'static str = "governance_transferred";
    /// Event Topic for renouncing the governance.
    pub const GOVERNANCE_RENOUNCED: &'static str = "governance_renounced";
}

/// Event emitted when the governance of a contract is accepted in a two
/// step transfer process.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct GovernanceAccepted {
    /// The previous governance of the contract.
    pub previous_governance: Account,
    /// The new governance of the contract.
    pub new_governance: Account,
}

impl GovernanceAccepted {
    /// Event Topic
    pub const TOPIC: &'static str = "governance_accepted";
}

/// Event emitted when an account status changes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct AccountStatus {
    /// The affected account.
    pub account: Account,
    /// The new status of the account.
    pub status: u64,
}

impl AccountStatus {
    /// The topic of the blocked event.
    pub const BLOCKED_TOPIC: &'static str = "blocked";
    /// The topic of the unblocked event.
    pub const UNBLOCKED_TOPIC: &'static str = "unblocked";
    /// The topic of the frozen event.
    pub const FROZEN_TOPIC: &'static str = "frozen";
    /// The topic of the unfrozen event.
    pub const UNFROZEN_TOPIC: &'static str = "unfrozen";

    /// Create a new `AccountStatus` event for a blocked account.
    pub fn blocked(account: impl Into<Account>) -> Self {
        Self {
            account: account.into(),
            status: 2,
        }
    }

    /// Create a new `AccountStatus` event for an unblocked account.
    pub fn unblocked(account: impl Into<Account>) -> Self {
        Self {
            account: account.into(),
            status: 0,
        }
    }

    /// Create a new `AccountStatus` event for a frozen account.
    pub fn frozen(account: impl Into<Account>) -> Self {
        Self {
            account: account.into(),
            status: 1,
        }
    }

    /// Create a new `AccountStatus` event for an unfrozen account.
    pub fn unfrozen(account: impl Into<Account>) -> Self {
        Self {
            account: account.into(),
            status: 0,
        }
    }
}
