// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used to inteact with the `ttoken-contract`.

#![no_std]
#![deny(missing_docs)]

/// Types used for administrative functions.
pub mod admin_management;
/// Types used for access control through ownership.
pub mod ownership;
/// Types used for supply management.
pub mod supply_management;

use core::cmp::Ordering;

use bytecheck::CheckBytes;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::{PublicKey, SecretKey, Signature};
use rkyv::{Archive, Deserialize, Serialize};

/// Error messages for when an account doesn't have enough tokens to perform the desired operation.
pub const BALANCE_TOO_LOW: &str = "The account doesn't have enough tokens";

/// The label for an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub enum Account {
    /// An externally owned account.
    External(PublicKey),
    /// A contract account.
    Contract(ContractId),
}

impl Account {
    fn to_bytes(&self) -> [u8; 194] {
        match self {
            Account::External(pk) => {
                let mut bytes = [0u8; 194];
                let pk_bytes = pk.to_raw_bytes();

                bytes[0] = 0;
                bytes[1..].copy_from_slice(&pk_bytes);

                bytes
            }
            Account::Contract(contract) => {
                let mut bytes = [0u8; 194];
                let contract_bytes = contract.to_bytes();

                bytes[0] = 1;
                bytes[1..1 + contract_bytes.len()].copy_from_slice(&contract_bytes);

                bytes
            }
        }
    }
}

impl From<PublicKey> for Account {
    fn from(pk: PublicKey) -> Self {
        Self::External(pk)
    }
}

impl From<ContractId> for Account {
    fn from(contract: ContractId) -> Self {
        Self::Contract(contract)
    }
}

// The implementations of `PartialOrd` and `Ord`, while technically meaningless,
// are extremely useful for using `Account` as keys of a `BTreeMap` in the
// contract.

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> Ordering {
        use Account::*;

        match (self, other) {
            (External(lhs), External(rhs)) => {
                let lhs = lhs.to_raw_bytes();
                let rhs = rhs.to_raw_bytes();
                lhs.cmp(&rhs)
            }
            (Contract(lhs), Contract(rhs)) => lhs.cmp(rhs),
            // An externally owned account is defined as always "smaller" than a
            // contract account. This ensures they are never mixed
            // when ordering.
            (External(_lhs), Contract(_rhs)) => Ordering::Greater,
            (Contract(_lhs), External(_rhs)) => Ordering::Less,
        }
    }
}

/// The data an account has in the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct AccountInfo {
    /// The balance of the account.
    pub balance: u64,
    /// The current nonce of the account. Use the current value +1 to perform
    /// an interaction with the account.
    pub nonce: u64,
}

impl AccountInfo {
    /// An empty account.
    pub const EMPTY: Self = Self {
        balance: 0,
        nonce: 0,
    };
}

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
    from: Account,
    to: Account,
    value: u64,
    nonce: u64,
    signature: Signature,
}

impl Transfer {
    const SIGNATURE_MSG_SIZE: usize = 194 + 194 + 8 + 8;

    /// Create a new transfer.
    pub fn new(
        sender_sk: &SecretKey,
        from: impl Into<Account>,
        to: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        let mut transfer = Self {
            from: from.into(),
            to: to.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = transfer.signature_message();
        let sig = sender_sk.sign(&sig_msg);
        transfer.signature = sig;

        transfer
    }

    /// The account to transfer from.
    pub fn from(&self) -> &Account {
        &self.from
    }

    /// The account to transfer to.
    pub fn to(&self) -> &Account {
        &self.to
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

        let bytes = self.from.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.to.to_bytes();
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

/// Data used to transfer tokens from an owner to a recipient, by an allowed
/// party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferFrom {
    spender: PublicKey,
    owner: Account,
    to: Account,
    value: u64,
    nonce: u64,
    signature: Signature,
}

impl TransferFrom {
    const SIGNATURE_MSG_SIZE: usize = 193 + 194 + 194 + 8 + 8;

    /// Create a new transfer, spending tokens from the `owner`.
    pub fn new(
        spender_sk: &SecretKey,
        owner: impl Into<Account>,
        to: impl Into<Account>,
        value: u64,
        nonce: u64,
    ) -> Self {
        let spender = PublicKey::from(spender_sk);

        let mut transfer_from = Self {
            spender,
            owner: owner.into(),
            to: to.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = transfer_from.signature_message();
        let sig = spender_sk.sign(&sig_msg);
        transfer_from.signature = sig;

        transfer_from
    }

    /// The account spending the tokens.
    pub fn spender(&self) -> &PublicKey {
        &self.spender
    }

    /// The account that owns the tokens being transferred.
    pub fn owner(&self) -> &Account {
        &self.owner
    }

    /// The account to transfer to.
    pub fn to(&self) -> &Account {
        &self.to
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

        let bytes = self.spender.to_raw_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.owner.to_bytes();
        msg[offset..][..bytes.len()].copy_from_slice(&bytes);
        offset += bytes.len();

        let bytes = self.to.to_bytes();
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

/// Data used to approve spending tokens from a contract's account.
///
/// Note that there is no need for a signature, since contracts are essentially
/// asserting via their code that they wish the transaction to be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TransferFromContract {
    /// The account to transfer to.
    pub to: Account,
    /// The owner of the funds to transfer from. If `None` it will be assumed
    /// to be the contract itself.
    pub from: Option<Account>,
    /// The value to transfer.
    pub value: u64,
}

/// Data used to approve spending tokens from a user's account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Approve {
    owner: PublicKey,
    spender: Account,
    value: u64,
    nonce: u64,
    signature: Signature,
}

impl Approve {
    const SIGNATURE_MSG_SIZE: usize = 193 + 194 + 8 + 8;

    /// Create a new approval.
    pub fn new(owner_sk: &SecretKey, spender: impl Into<Account>, value: u64, nonce: u64) -> Self {
        let owner = PublicKey::from(owner_sk);

        let mut approve = Self {
            owner,
            spender: spender.into(),
            value,
            nonce,
            signature: Signature::default(),
        };

        let sig_msg = approve.signature_message();
        let sig = owner_sk.sign(&sig_msg);
        approve.signature = sig;

        approve
    }

    /// The account to allow the transfer of tokens.
    pub fn owner(&self) -> &PublicKey {
        &self.owner
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

        let bytes = self.owner.to_raw_bytes();
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
    pub owner: Account,
    /// The account spending the tokens, set if `transfer_from` is used.
    pub spender: Option<Account>,
    /// The account receiving the tokens.
    pub to: Account,
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
    pub owner: Account,
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
    pub from: Account,
    /// The number of tokens transferred.
    pub value: u64,
}
