use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::{Account, AccountInfo};

/// Error message for when an account is blocked.
pub const BLOCKED: &str = "Account is blocked";
/// Error message for when an account is frozen.
pub const FROZEN: &str = "Account is frozen";

/// Arguments for sanction transactions.
pub mod arguments {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// Data used to sanction an account.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct Sanction {
        account: Account,
        sanction_type: u64,
        nonce: u64,
        signature: Signature,
    }

    impl Sanction {
        const SIGNATURE_MSG_SIZE: usize = 194 + 8 + 8;

        /// Create a new `Sanction` transaction for freezing an account.
        pub fn freeze_account(
            owner_sk: &SecretKey,
            account: impl Into<Account>,
            nonce: u64,
        ) -> Self {
            let mut sanction = Self {
                account: account.into(),
                sanction_type: AccountInfo::FROZEN,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = sanction.signature_message();
            let sig = owner_sk.sign(&sig_msg);
            sanction.signature = sig;

            sanction
        }

        /// Create a new `Sanction` transaction for blocking an account.
        pub fn block_account(
            owner_sk: &SecretKey,
            account: impl Into<Account>,
            nonce: u64,
        ) -> Self {
            let mut sanction = Self {
                account: account.into(),
                sanction_type: AccountInfo::BLOCKED,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = sanction.signature_message();
            let sig = owner_sk.sign(&sig_msg);
            sanction.signature = sig;

            sanction
        }

        /// Create a new `Unsanction` transaction for un-sanctioning an account.
        pub fn unsanction_account(
            owner_sk: &SecretKey,
            account: impl Into<Account>,
            nonce: u64,
        ) -> Self {
            let mut sanction = Self {
                account: account.into(),
                sanction_type: AccountInfo::NO_STATUS,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = sanction.signature_message();
            let sig = owner_sk.sign(&sig_msg);
            sanction.signature = sig;

            sanction
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

            let mut offset = 0;

            let bytes = self.account.to_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.sanction_type.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.nonce.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);

            msg
        }

        /// Get the account specified for this transaction.
        pub fn account(&self) -> &Account {
            &self.account
        }

        /// Get the sanction type specified for this transaction.
        pub fn sanction_type(&self) -> u64 {
            self.sanction_type
        }

        /// Get the nonce specified for this transaction.
        pub fn nonce(&self) -> u64 {
            self.nonce
        }

        /// Get the signature specified for this transaction.
        pub fn signature(&self) -> &Signature {
            &self.signature
        }
    }
}

/// Events for sanction transactions.
pub mod events {
    use super::*;

    /// Event emitted when an account status changes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct AccountStatusEvent {
        /// The affected account.
        pub account: Account,
        /// The new status of the account.
        pub status: u64,
    }

    impl AccountStatusEvent {
        /// The topic of the blocked event.
        pub const BLOCKED_TOPIC: &'static str = "blocked";
        /// The topic of the unblocked event.
        pub const UNBLOCKED_TOPIC: &'static str = "unblocked";
        /// The topic of the frozen event.
        pub const FROZEN_TOPIC: &'static str = "frozen";
        /// The topic of the unfrozen event.
        pub const UNFROZEN_TOPIC: &'static str = "unfrozen";

        /// Create a new `AccountStatusEvent` for a blocked account.
        pub fn blocked(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 2,
            }
        }

        /// Create a new `AccountStatusEvent` for an unblocked account.
        pub fn unblocked(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 0,
            }
        }

        /// Create a new `AccountStatusEvent` for a frozen account.
        pub fn frozen(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 1,
            }
        }

        /// Create a new `AccountStatusEvent` for an unfrozen account.
        pub fn unfrozen(account: impl Into<Account>) -> Self {
            Self {
                account: account.into(),
                status: 0,
            }
        }
    }
}
