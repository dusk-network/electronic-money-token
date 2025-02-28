// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when the admin account is not found in the contract.
pub const OWNER_NOT_FOUND: &str = "The owner does not exist";

/// Error message for when the owner is not authorized i.e., wrong public_sender
/// value.
pub const UNAUTHORIZED_ACCOUNT: &str = "Unauthorized account";

/// Arguments for ownership transactions.
pub mod arguments {
    use super::*;

    /// Data used to transfer ownership of a contract.
    ///
    /// The arguments do not need to specify the current owner, as the signature
    /// will be verified by the contract that has access to the current owner
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct TransferOwnership {
        new_owner: Account,
    }

    impl TransferOwnership {
        /// Create a new `TransferOwnership` transaction. This transaction is
        /// used to change the owner of an account.
        ///
        /// # Arguments
        ///
        /// * `new_owner` - The new owner of the account.
        pub fn new(new_owner: impl Into<Account>) -> Self {
            Self {
                new_owner: new_owner.into(),
            }
        }

        /// Get the new owner specified for this transaction.
        pub fn new_owner(&self) -> &Account {
            &self.new_owner
        }
    }
}

/// Events emitted by ownership transactions.
pub mod events {
    use super::*;

    /// Event emitted when the ownership of a contract is transferred.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct OwnershipTransferredEvent {
        /// The previous owner of the contract.
        pub previous_owner: Account,
        /// The new owner of the contract.
        pub new_owner: Account,
    }

    impl OwnershipTransferredEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_transferred";
    }

    /// Event emitted when the ownership of a contract is accepted in a two step
    /// transfer process.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct OwnershipAcceptedEvent {
        /// The previous owner of the contract.
        pub previous_owner: Account,
        /// The new owner of the contract.
        pub new_owner: Account,
    }

    impl OwnershipAcceptedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_accepted";
    }

    /// Event emitted when the ownership of a contract is renounced i.e., no
    /// owner exists anymore.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct OwnerShipRenouncedEvent {
        /// The previous owner of the contract.
        pub previous_owner: Account,
    }

    impl OwnerShipRenouncedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_renounced";
    }
}
