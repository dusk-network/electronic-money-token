// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Module for the account implementation.
pub(crate) mod account;

use account::{Account, AccountInfo};
use bytecheck::CheckBytes;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use rkyv::{Archive, Deserialize, Serialize};

use crate::BALANCE_TOO_LOW;

/// Zero address.
/// TODO: Consider having this in core & make it a reserved address so that no
/// one can ever use it.
pub const ZERO_ADDRESS: Account =
    Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]));

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

/// Trait defining transfer conditions that can be applied to a transfer.
/// This is used to enforce rules on the transfer of tokens.
pub trait Condition {
    /// Precondition that is executed before the transfer.
    #[allow(unused_variables)]
    fn sender_precondition(&self, sender: &AccountInfo, value: u64) {
        assert!(sender.balance >= value, "{}", BALANCE_TOO_LOW);
    }

    /// Precondition that is executed before the transfer.
    #[allow(unused_variables)]
    fn receiver_precondition(&self, receiver: &AccountInfo, value: u64) {}

    /// Optional postcondition that may be executed after the transfer.
    fn postcondition(
        &self,
        sender: &AccountInfo,
        receiver: &AccountInfo,
        value: u64,
    );
}
