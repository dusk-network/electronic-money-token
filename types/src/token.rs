// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Module for the account implementation.
pub mod account;
pub use account::*;

use bytecheck::CheckBytes;
use dusk_core::signatures::bls::{PublicKey, SecretKey, Signature};
use rkyv::{Archive, Deserialize, Serialize};

/// Arguments to query for how much of an allowance a spender has of the `owner`
/// account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Allowance {
    /// The account that owns the tokens.
    pub owner: Account,
    /// The account allowed to spend the `owner`s tokens.
    pub spender: Account,
}

/// Data used to transfer tokens from one account to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transfer {
    sender: Account,
    receiver: Account,
    value: u64,
    nonce: u64,
    /// The signature is optional as a contract account doesn't need to sign the transfer.
    signature: Signature,
}

impl Transfer {
    const SIGNATURE_MSG_SIZE: usize = 194 + 194 + 8 + 8;

    /// Create a new transfer with an external account.
    pub fn new_external(
        sender_sk: &SecretKey,
        sender: impl Into<Account>,
        receiver: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        let mut transfer = Self {
            sender: sender.into(),
            receiver: receiver.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = transfer.signature_message();
        let sig = sender_sk.sign(&sig_msg);
        transfer.signature = sig;
        transfer
    }

    /// Create a new transfer with a contract account.
    pub fn new_contract(
        sender: impl Into<Account>,
        receiver: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        Self {
            sender: sender.into(),
            receiver: receiver.into(),
            value,
            nonce,
            signature: Signature::default(),
        }
    }

    /// The account to transfer from.
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

    /// The nonce used to sign the transfer.
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// The signature used for the transfer.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The message to be signed over.
    pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
        let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

        let mut offset = 0;

        let bytes = self.sender.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.receiver.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.value.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.nonce.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        // offset += bytes.len();

        msg
    }
}

/// Data used to transfer tokens from an owner (sender) to a receiver, by an allowed
/// party (spender).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferFrom {
    spender: Account,
    sender: Account,
    receiver: Account,
    value: u64,
    nonce: u64,
    signature: Signature,
}

impl TransferFrom {
    const SIGNATURE_MSG_SIZE: usize = 194 + 194 + 194 + 8 + 8;

    /// Create a new transfer for external accounts, spending tokens from the `sender`.
    pub fn new_external(
        spender_sk: &SecretKey,
        sender: impl Into<Account>,
        receiver: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        let spender = PublicKey::from(spender_sk);

        let mut transfer_from = Self {
            spender: Account::External(spender),
            sender: sender.into(),
            receiver: receiver.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = transfer_from.signature_message();
        let sig = spender_sk.sign(&sig_msg);
        transfer_from.signature = sig;

        transfer_from
    }

    /// Create a new transfer for contracts, spending tokens from the `sender`.
    pub fn new_contract(
        spender: impl Into<Account>,
        sender: impl Into<Account>,
        receiver: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        Self {
            spender: spender.into(),
            sender: sender.into(),
            receiver: receiver.into(),
            value,
            nonce,
            signature: Signature::default(),
        }
    }

    /// The account spending the tokens.
    pub fn spender(&self) -> &Account {
        &self.spender
    }

    /// The account that owns the tokens being transferred.
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

    /// The nonce used to sign the transfer.
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// The signature used for the transfer.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The message to be signed over.
    pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
        let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

        let mut offset = 0;

        let bytes = self.spender.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.sender.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.receiver.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.value.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.nonce.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        // offset += bytes.len();

        msg
    }
}

/// Data used to approve spending tokens from a user's account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Approve {
    sender: Account,
    spender: Account,
    value: u64,
    nonce: u64,
    signature: Signature,
}

impl Approve {
    const SIGNATURE_MSG_SIZE: usize = 194 + 194 + 8 + 8;

    /// Create a new approval for an external account.
    pub fn new_external(
        sender_sk: &SecretKey,
        spender: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        let owner = PublicKey::from(sender_sk);

        let mut approve = Self {
            sender: Account::External(owner),
            spender: spender.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = approve.signature_message();
        let sig = sender_sk.sign(&sig_msg);
        approve.signature = sig;

        approve
    }

    /// Create a new approval for a contract account.
    pub fn new_contract(
        sender: impl Into<Account>,
        spender: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        Self {
            sender: sender.into(),
            spender: spender.into(),
            value,
            nonce,
            signature: Signature::default(),
        }
    }

    /// The account to allow the transfer of tokens.
    pub fn sender(&self) -> &Account {
        &self.sender
    }

    /// The account to allow spending tokens from.
    pub fn spender(&self) -> &Account {
        &self.spender
    }

    /// The value to approve the transfer of.
    pub fn value(&self) -> u64 {
        self.value
    }

    /// The nonce used to sign the allowance.
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// The signature used for the allowance.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The message to be signed over.
    pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
        let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

        let mut offset = 0;

        let bytes = self.sender.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.spender.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.value.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.nonce.to_le_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        // offset += bytes.len();

        msg
    }
}

/// Event emitted when tokens are transferred from one account to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferInfo {
    /// The originating account of the funds transferred to the contract.
    pub sender: Account,
    /// The number of tokens transferred.
    pub value: u64,
}
