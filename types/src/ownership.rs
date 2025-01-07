use bytecheck::CheckBytes;
use dusk_core::signatures::bls::PublicKey;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Error message for when the owner is not set.
pub const OWNER_NOT_SET: &str = "Owner not set";

/// Error message for when the owner is not authorized i.e., signature verification failed.
pub const UNAUTHORIZED_EXT_ACCOUNT: &str = "Unauthorized external account";

/// Error message for when the contract is not authorized i.e., wrong contract id.
pub const UNAUTHORIZED_CONTRACT: &str = "Unauthorized contract";

/// Error message for when an external account calls a contract function that expects a contract id as the caller & owner.
pub const EXPECT_CONTRACT: &str = "Must be called by a contract";

/// Payloads for ownership transactions.
pub mod payloads {
    use dusk_core::signatures::bls::{SecretKey, Signature};

    use super::*;

    /// Data used to transfer ownership of a contract.
    ///
    /// The payload does not need to specify the current owner, as the signature
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

            msg[..194].copy_from_slice(&self.new_owner.to_bytes());
            msg[194..].copy_from_slice(&self.nonce.to_le_bytes());
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

    /// Data used to accept ownership of a contract.
    ///
    /// The payload does not need to specify the new owner, as the signature
    /// will be verified by the contract that has access to the new owner
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct AcceptOwnership {
        nonce: u64,
        signature: Signature,
    }

    impl AcceptOwnership {
        const SIGNATURE_MSG_SIZE: usize = 8;

        /// Create a new `AcceptOwnership` transaction. This transaction is used to
        /// accept ownership of an account when the ownership is implemented as a
        /// two-step process.
        ///
        /// # Arguments
        ///
        /// * `new_owner_sk` - The secret key of the new owner.
        /// * `nonce` - The nonce of the account.
        pub fn new(new_owner_sk: &SecretKey, nonce: u64) -> Self {
            let mut accept_ownership = Self {
                nonce,
                signature: Signature::default(),
            };

            let sig_msg = accept_ownership.signature_message();
            let sig = new_owner_sk.sign(&sig_msg);
            accept_ownership.signature = sig;

            accept_ownership
        }

        /// The message to be signed over.
        pub fn signature_message(&self) -> [u8; Self::SIGNATURE_MSG_SIZE] {
            self.nonce.to_le_bytes()
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
    ///
    /// The payload does not need to specify the current owner, however the signature
    /// needs to be different from the `AcceptOwnership` transaction.
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

            msg[..194].copy_from_slice(&self.current_owner.to_bytes());
            msg[194..].copy_from_slice(&self.nonce.to_le_bytes());
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
        previous_owner: Account,
        /// The new owner of the contract.
        new_owner: Account,
    }

    impl OwnershipTransferredEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_transferred";

        /// Create a new `OwnerShipTransferredEvent` instance.
        pub fn new(previous_owner: Account, new_owner: Account) -> Self {
            Self {
                previous_owner,
                new_owner,
            }
        }

        /// Get the previous owner of the contract.
        pub fn previous_owner(&self) -> &Account {
            &self.previous_owner
        }

        /// Get the new owner of the contract.
        pub fn new_owner(&self) -> &Account {
            &self.new_owner
        }
    }

    /// Event emitted when the ownership of a contract is accepted in a two step transfer process.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct OwnershipAcceptedEvent {
        /// The previous owner of the contract.
        previous_owner: Account,
        /// The new owner of the contract.
        new_owner: Account,
    }

    impl OwnershipAcceptedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_accepted";

        /// Create a new `OwnerShipAcceptedEvent` instance.
        pub fn new(previous_owner: Account, new_owner: Account) -> Self {
            Self {
                previous_owner,
                new_owner,
            }
        }

        /// Get the previous owner of the contract.
        pub fn previous_owner(&self) -> &Account {
            &self.previous_owner
        }

        /// Get the new owner of the contract.
        pub fn new_owner(&self) -> &Account {
            &self.new_owner
        }
    }

    /// Event emitted when the ownership of a contract is renounced i.e., no owner exists anymore.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub struct OwnerShipRenouncedEvent {
        /// The previous owner of the contract.
        previous_owner: Account,
    }

    impl OwnerShipRenouncedEvent {
        /// Event Topic
        pub const TOPIC: &'static str = "ownership_renounced";

        /// Create a new `OwnerShipRenouncedEvent` instance.
        pub fn new(previous_owner: Account) -> Self {
            Self { previous_owner }
        }

        /// Get the previous owner of the contract.
        pub fn previous_owner(&self) -> &Account {
            &self.previous_owner
        }
    }
}
