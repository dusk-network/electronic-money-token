// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when the contract is paused.
pub const PAUSED_MESSAGE: &str = "Contract is paused";

/// Default owner i.e., no owner. This is the Zero address.
pub const DEFAULT_OWNER: Account =
    Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]));

/// Events emitted by admin management transactions.
pub mod events {

    use super::*;

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
}
