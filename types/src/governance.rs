// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when the admin account is not found in the contract.
pub const GOVERNANCE_NOT_FOUND: &str = "The governance does not exist";

/// Error message for when the governance is not authorized i.e., wrong
/// public_sender value.
pub const UNAUTHORIZED_ACCOUNT: &str = "Unauthorized account";

/// Arguments for governance transactions.
pub mod arguments {
    use super::*;

    /// Data used to transfer governance of a contract.
    ///
    /// The arguments do not need to specify the current governance, as the
    /// signature will be verified by the contract that has access to the
    /// current governance
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct TransferGovernance {
        new_governance: Account,
    }

    impl TransferGovernance {
        /// Create a new `TransferGovernance` transaction. This transaction is
        /// used to change the governance of an account.
        ///
        /// # Arguments
        ///
        /// * `new_governance` - The new governance of the account.
        pub fn new(new_governance: impl Into<Account>) -> Self {
            Self {
                new_governance: new_governance.into(),
            }
        }

        /// Get the new governance specified for this transaction.
        pub fn new_governance(&self) -> &Account {
            &self.new_governance
        }
    }
}

/// Events emitted by governance transactions.
pub mod events {
    use super::*;

    /// Event emitted when the governance of a contract is transferred.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct GovernanceTransferredEvent {
        /// The previous governance of the contract.
        pub previous_governance: Account,
        /// The new governance of the contract.
        pub new_governance: Account,
    }

    impl GovernanceTransferredEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "governance_transferred";
    }

    /// Event emitted when the governance of a contract is accepted in a two
    /// step transfer process.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct GovernanceAcceptedEvent {
        /// The previous governance of the contract.
        pub previous_governance: Account,
        /// The new governance of the contract.
        pub new_governance: Account,
    }

    impl GovernanceAcceptedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "governance_accepted";
    }

    /// Event emitted when the governance of a contract is renounced i.e., no
    /// governance exists anymore.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct GovernanceRenouncedEvent {
        /// The previous governance of the contract.
        pub previous_governance: Account,
    }

    impl GovernanceRenouncedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "governance_renounced";
    }
}
