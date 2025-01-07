use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error messages for overflow when minting tokens.
pub const SUPPLY_OVERFLOW: &str = "Supply overflow";

/// Payloads for supply management (mint, burn) transactions.
pub mod payloads {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// A mint transaction.
    ///
    /// This transaction is used to mint new tokens. It is signed by the minter.
    ///
    /// The `Mint` struct assumes the minter is known to the contract e.g., through the owner.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct Mint {
        amount: u64,
        recipient: Account,
        nonce: u64,
        signature: Signature,
    }

    impl Mint {
        const SIGNATURE_MSG_SIZE: usize = 8 + 194 + 8;

        /// Create a new `Mint` transaction. This transaction is used to mint new tokens.
        pub fn new(minter_sk: &SecretKey, amount: u64, recipient: Account, nonce: u64) -> Self {
            let mut mint = Self {
                amount,
                recipient,
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

            msg[..8].copy_from_slice(&self.amount.to_le_bytes());
            msg[8..202].copy_from_slice(&self.recipient.to_bytes());
            msg[202..].copy_from_slice(&self.nonce.to_le_bytes());

            msg
        }

        /// Get the amount being minted for the mint transaction.
        pub fn amount(&self) -> u64 {
            self.amount
        }

        /// Get the recipient of the minted tokens.
        pub fn recipient(&self) -> &Account {
            &self.recipient
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
    /// The `Burn` struct assumes the burner is known to the contract e.g., through the owner
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct Burn {
        amount: u64,
        nonce: u64,
        signature: Signature,
    }

    impl Burn {
        const SIGNATURE_MSG_SIZE: usize = 8 + 8;

        /// Create a new `Burn` transaction. This transaction is used to burn tokens.
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

            msg[..8].copy_from_slice(&self.amount.to_le_bytes());
            msg[8..].copy_from_slice(&self.nonce.to_le_bytes());
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
    // note: mint events often re-use a transfer event from a 0 address to the recipient to avoid integrating more event types than necessary
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct MintEvent {
        amount_minted: u64,
        recipient: Account,
    }

    impl MintEvent {
        /// The topic of the event.
        pub const TOPIC: &'static str = "Mint";

        /// Create a new `MintEvent`.
        pub fn new(amount_minted: u64, recipient: Account) -> Self {
            Self {
                amount_minted,
                recipient,
            }
        }

        /// Get the amount of tokens minted.
        pub fn amount_minted(&self) -> u64 {
            self.amount_minted
        }

        /// Get the recipient of the minted tokens.
        pub fn recipient(&self) -> &Account {
            &self.recipient
        }
    }

    /// Event emitted when tokens are burned.
    // note: burns usually often re-use a transfer event to the 0 address to avoid integrating more event types than necessary
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct BurnEvent {
        amount_burned: u64,
        burned_by: Account,
    }

    impl BurnEvent {
        /// The topic of the event.
        pub const TOPIC: &'static str = "Burn";

        /// Create a new `BurnEvent`.
        pub fn new(amount_burned: u64, burned_by: Account) -> Self {
            Self {
                amount_burned,
                burned_by,
            }
        }

        /// Get the amount of tokens burned.
        pub fn amount_burned(&self) -> u64 {
            self.amount_burned
        }

        /// Get the account that burned the tokens.
        pub fn burned_by(&self) -> &Account {
            &self.burned_by
        }
    }
}
