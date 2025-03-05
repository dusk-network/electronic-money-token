// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Module for the account implementation.
pub(crate) mod account;
use account::Account;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Arguments to query for how much of an allowance a spender has of the `owner`
/// account.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Allowance {
    /// The account that owns the tokens.
    pub owner: Account,
    /// The account allowed to spend the `owner`s tokens.
    pub spender: Account,
}

/// Data used to transfer tokens from one account to another.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Transfer {
    receiver: Account,
    value: u64,
}

impl Transfer {
    /// Create a new transfer with an external account.
    pub fn new(receiver: impl Into<Account>, value: u64) -> Self {
        Self {
            receiver: receiver.into(),
            value,
        }
    }

    /// The account to transfer to.
    pub fn receiver(&self) -> &Account {
        &self.receiver
    }

    /// The value to transfer.
    pub fn value(&self) -> u64 {
        self.value
    }
}

/// Data used to transfer tokens from an owner (sender) to a receiver, by an
/// allowed party (spender).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferFrom {
    sender: Account,
    receiver: Account,
    value: u64,
}

impl TransferFrom {
    /// Create a new transfer for external accounts, spending tokens from the
    /// `sender`.
    pub fn new(
        sender: impl Into<Account>,
        receiver: impl Into<Account>,
        value: u64,
    ) -> Self {
        Self {
            sender: sender.into(),
            receiver: receiver.into(),
            value,
        }
    }

    /// The account from which the tokens are spent.
    pub fn sender(&self) -> &Account {
        &self.sender
    }

    /// The account to transfer to.
    pub fn receiver(&self) -> &Account {
        &self.receiver
    }

    /// The value to transfer.
    pub fn value(&self) -> u64 {
        self.value
    }
}

/// Data used to approve spending tokens from a user's account.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct Approve {
    spender: Account,
    value: u64,
}

impl Approve {
    /// Create a new approval for an external account.
    pub fn new(spender: impl Into<Account>, value: u64) -> Self {
        Self {
            spender: spender.into(),
            value,
        }
    }

    /// The account to allow spending tokens from.
    pub fn spender(&self) -> &Account {
        &self.spender
    }

    /// The value to approve the transfer of.
    pub fn value(&self) -> u64 {
        self.value
    }
}

/// Event emitted when tokens are transferred from one account to another.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferEvent {
    /// The account tokens are transferred from.
    pub sender: Account,
    /// The account spending the tokens, set if `transfer_from` is used.
    pub spender: Option<Account>,
    /// The account receiving the tokens.
    pub receiver: Account,
    /// The value transferred.
    pub value: u64,
}

impl TransferEvent {
    /// Event topic used when a normal transfer is made.
    pub const TRANSFER_TOPIC: &'static str = "transfer";
    /// Event topic used when a forced transfer is made.
    pub const FORCE_TRANSFER_TOPIC: &'static str = "force_transfer";
}

/// Event emitted when a spender is approved on an account.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct ApproveEvent {
    /// The account allowing the transfer.
    pub sender: Account,
    /// The allowed spender.
    pub spender: Account,
    /// The value `spender` is allowed to spend.
    pub value: u64,
}

/// Used to inform a contract of the source of funds they're receiving.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferInfo {
    /// The originating account of the funds transferred to the contract.
    pub sender: Account,
    /// The number of tokens transferred.
    pub value: u64,
}
