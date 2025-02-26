// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error messages for overflow when minting tokens.
pub const SUPPLY_OVERFLOW: &str = "Supply overflow";

/// Arguments for supply management (mint, burn) transactions.
pub mod arguments {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// A mint transaction.
    ///
    /// This transaction is used to mint new tokens. It is signed by the minter.
    ///
    /// The `Mint` struct assumes the minter is known to the contract e.g.,
    /// through the owner.
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct Mint {
        amount: u64,
        receiver: Account,
        nonce: u64,
        signature: Signature,
    }

    impl Mint {
        const SIGNATURE_MSG_SIZE: usize = 8 + 194 + 8;

        /// Create a new `Mint` transaction. This transaction is used to mint
        /// new tokens.
        pub fn new(
            minter_sk: &SecretKey,
            amount: u64,
            receiver: Account,
            nonce: u64,
        ) -> Self {
            let mut mint = Self {
                amount,
                receiver,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = mint.signature_message();
            let sig = minter_sk.sign(&sig_msg);
            mint.signature = sig;

            mint
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

            let mut offset = 0;
            let bytes = self.amount.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.receiver.to_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.nonce.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);

            msg
        }

        /// Get the amount being minted for the mint transaction.
        pub fn amount(&self) -> u64 {
            self.amount
        }

        /// Get the receiver of the minted tokens.
        pub fn receiver(&self) -> &Account {
            &self.receiver
        }

        /// Get the nonce of the mint transaction.
        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        /// The signature used for minting.
        pub fn signature(&self) -> &Signature {
            &self.signature
        }
    }

    /// A burn transaction. This transaction is used to burn tokens.
    ///
    /// The `Burn` struct assumes the burner is known to the contract e.g.,
    /// through the owner
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct Burn {
        amount: u64,
        nonce: u64,
        signature: Signature,
    }

    impl Burn {
        const SIGNATURE_MSG_SIZE: usize = 8 + 8;

        /// Create a new `Burn` transaction. This transaction is used to burn
        /// tokens.
        pub fn new(burner_sk: &SecretKey, amount: u64, nonce: u64) -> Self {
            let mut burn = Self {
                amount,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = burn.signature_message();
            let sig = burner_sk.sign(&sig_msg);
            burn.signature = sig;

            burn
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

            let mut offset = 0;

            let bytes = self.amount.to_le_bytes();
            msg[..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.nonce.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            msg
        }

        /// Get the amount being burned for the burn transaction.
        pub fn amount(&self) -> u64 {
            self.amount
        }

        /// Get the nonce of the burn transaction.
        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        /// The signature used for burning.
        pub fn signature(&self) -> &Signature {
            &self.signature
        }
    }
}

/// Events emitted by supply management transactions.
pub mod events {
    use super::*;

    /// Event emitted when new tokens are minted.
    // note: mint events often re-use a transfer event from a 0 address to the
    // receiver to avoid integrating more event types than necessary
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct MintEvent {
        /// The amount of tokens minted.
        pub amount_minted: u64,
        /// the receiver of the minted tokens.
        pub receiver: Account,
    }

    impl MintEvent {
        /// The topic of the event.
        pub const TOPIC: &'static str = "mint";
    }

    /// Event emitted when tokens are burned.
    // note: burns usually often re-use a transfer event to the 0 address to
    // avoid integrating more event types than necessary
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
    )]
    #[archive_attr(derive(CheckBytes))]
    pub struct BurnEvent {
        /// The amount of burned tokens.
        pub amount_burned: u64,
        /// The account that burned the tokens.
        pub burned_by: Account,
    }

    impl BurnEvent {
        /// The topic of the event.
        pub const TOPIC: &'static str = "burn";
    }
}
