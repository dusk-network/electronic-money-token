use bytecheck::CheckBytes;
use dusk_core::signatures::bls::PublicKey;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when the owner is not set.
pub const OWNER_NOT_SET: &str = "Owner not set";

/// Error message for when the admin account is not found in the contract.
pub const OWNER_NOT_FOUND: &str = "The owner does not exist";

/// Error message for when the owner is not authorized i.e., signature verification failed.
pub const UNAUTHORIZED_EXT_ACCOUNT: &str = "Unauthorized external account";

/// Error message for when the contract is not authorized i.e., wrong contract id.
pub const UNAUTHORIZED_CONTRACT: &str = "Unauthorized contract";

/// Error message for when an external account calls a contract function that expects a contract id as the caller & owner.
pub const EXPECT_CONTRACT: &str = "Must be called by a contract";

/// Arguments for ownership transactions.
pub mod arguments {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// Data used to transfer ownership of a contract.
    ///
    /// The arguments do not need to specify the current owner, as the signature
    /// will be verified by the contract that has access to the current owner
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct TransferOwnership {
        new_owner: Account,
        nonce: u64,
        signature: Signature,
    }

    impl TransferOwnership {
        const SIGNATURE_MSG_SIZE: usize = 194 + 8;

        /// Create a new `TransferOwnership` transaction. This transaction is used
        /// to change the owner of an account.
        ///
        /// # Arguments
        ///
        /// * `owner_sk` - The secret key of the current owner.
        /// * `new_owner` - The new owner of the account.
        /// * `nonce` - The nonce of the owner_pk account.
        pub fn new(owner_sk: &SecretKey, new_owner: impl Into<Account>, nonce: u64) -> Self {
            let mut change_owner = Self {
                new_owner: new_owner.into(),
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = change_owner.signature_message();
            let sig = owner_sk.sign(&sig_msg);
            change_owner.signature = sig;

            change_owner
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

            let mut offset = 0;

            let bytes = self.new_owner.to_bytes();
            msg[..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.nonce.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&self.nonce.to_le_bytes());

            msg
        }

        /// Get the new owner specified for this transaction.
        pub fn new_owner(&self) -> &Account {
            &self.new_owner
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

    /// Data used to renounce ownership of an account.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct RenounceOwnership {
        current_owner: Account,
        nonce: u64,
        signature: Signature,
    }

    impl RenounceOwnership {
        const SIGNATURE_MSG_SIZE: usize = 194 + 8;

        /// Create a new `RenounceOwnership` transaction. This transaction is used
        /// to renounce ownership of an account.
        ///
        /// # Arguments
        ///
        /// * `owner_sk` - The secret key of the current owner.
        /// * `nonce` - The nonce of the account.
        pub fn new(owner_sk: &SecretKey, nonce: u64) -> Self {
            let current_owner = Account::from(PublicKey::from(owner_sk));

            let mut renounce_ownership = Self {
                current_owner,
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = renounce_ownership.signature_message();
            let sig = owner_sk.sign(&sig_msg);
            renounce_ownership.signature = sig;

            renounce_ownership
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            let mut msg = [0u8; Self::SIGNATURE_MSG_SIZE];

            let mut offset = 0;

            let bytes = self.current_owner.to_bytes();
            msg[..offset + bytes.len()].copy_from_slice(&bytes);
            offset += bytes.len();

            let bytes = self.nonce.to_le_bytes();
            msg[offset..offset + bytes.len()].copy_from_slice(&bytes);
            msg
        }

        /// Get the current owner specified for this transaction.
        pub fn current_owner(&self) -> &Account {
            &self.current_owner
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

/// Events emitted by ownership transactions.
pub mod events {
    use super::*;

    /// Event emitted when the ownership of a contract is transferred.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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

    /// Event emitted when the ownership of a contract is accepted in a two step transfer process.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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

    /// Event emitted when the ownership of a contract is renounced i.e., no owner exists anymore.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
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
