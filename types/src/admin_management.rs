// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Error message for when the contract is paused.
pub const PAUSED_MESSAGE: &str = "Contract is paused";

/// Arguments for admin management transactions.
pub mod arguments {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// Data used to toggle the pause state of the contract.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct PauseToggle {
        nonce: u64,
        signature: Signature,
    }

    impl PauseToggle {
        const SIGNATURE_MSG_SIZE: usize = 8;

        /// Create a new pause toggle.
        pub fn new(sender_sk: &SecretKey, nonce: u64) -> Self {
            let mut toggle = Self {
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = toggle.signature_message();
            let sig = sender_sk.sign(&sig_msg);
            toggle.signature = sig;

            toggle
        }

        /// The nonce used for the toggle.
        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        /// The signature used for the toggle.
        pub fn signature(&self) -> &Signature {
            &self.signature
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            self.nonce.to_le_bytes()
        }
    }
}

/// Events emitted by admin management transactions.
pub mod events {

    use super::*;

    /// Event emitted when a contract is paused or unpaused.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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
