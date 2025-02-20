// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_bytes::Serializable;
use dusk_core::abi::{self, ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{
    MultisigPublicKey, MultisigSignature, PublicKey,
};
use ttoken_types::ownership::arguments::{
    RenounceOwnership, TransferOwnership,
};

use crate::error;

/// The state of the token governance contract.
pub struct GovernanceState {
    // The owners, and only the owners, of a token-contract are authorized to
    // change the owners or operators of the  token-contract.
    owners: Vec<PublicKey>,
    // The nonce for the owners, initialized at 0 and strictly increasing.
    owner_nonce: u64,
    // The operators of the token-contract are authorized to to all
    // token-operations except changing ownership.
    operators: Vec<PublicKey>,
    // The nonce for the operators, initialized at 0 and strictly increasing.
    operator_nonce: u64,
    // A map for all the operations executable by the operators on the
    // token-contract. Each operation has a threshold of signers that are
    // required to execute the operation.
    // If the threshold for an operation is set to 0, a super-majority of
    // signers is needed.
    operations: BTreeMap<String, u8>,
}

pub static mut STATE: GovernanceState = GovernanceState::new();

/// Basic contract implementation.
impl GovernanceState {
    /// The contract-id of the token-contract.
    ///
    /// **Important:** The token-contract must be set to the correct id
    /// **before** the governance contract is deployed.
    pub const TOKEN_CONTRACT: ContractId =
        ContractId::from_bytes([0u8; CONTRACT_ID_BYTES]);

    /// Create a new empty instance of the governance-contract.
    pub const fn new() -> Self {
        Self {
            owners: Vec::new(),
            owner_nonce: 0,
            operators: Vec::new(),
            operator_nonce: 0,
            operations: BTreeMap::new(),
        }
    }

    /// Create a new instance of the governance contract state for a given
    /// token-contract, sets of owners, operators and operations.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The contract is already initialized.
    /// - The given set of owner keys is empty.
    /// - The given set of owner keys is larger than u8::MAX.
    /// - The given set of operator keys is larger than u8::MAX.
    /// - There are duplicate owner keys.
    /// - There are duplicate operator keys.
    // TODO: write check for duplicate keys.
    pub fn init(
        &mut self,
        owners: Vec<PublicKey>,
        operators: Vec<PublicKey>,
        operation_vec: Vec<(String, u8)>,
    ) {
        // panic if the contract has already been initialized
        if !self.owners.is_empty() {
            panic!("{}", error::ALLREADY_INITIALIZED);
        }
        // panic if no owners are given
        if owners.is_empty() {
            panic!("{}", error::EMTPY_OWNER);
        }
        // panic if more than u8::MAX owners are given
        if owners.len() > u8::MAX as usize {
            panic!("{}", error::TOO_MANY_OWNERS)
        }

        let mut operations = BTreeMap::new();
        operation_vec
            .into_iter()
            .for_each(|(operation, threshold)| {
                operations.insert(operation, threshold);
            });

        self.owners = owners;
        self.operators = operators;
        self.operations = operations;
    }

    pub fn name(&self) -> String {
        String::from("Token Governance Sample")
    }

    pub fn symbol(&self) -> String {
        String::from("TGS")
    }

    pub fn owners(&self) -> Vec<PublicKey> {
        self.owners.clone()
    }

    pub fn owner_nonce(&self) -> u64 {
        self.owner_nonce
    }

    pub fn operators(&self) -> Vec<PublicKey> {
        self.operators.clone()
    }

    pub fn operator_nonce(&self) -> u64 {
        self.operator_nonce
    }

    /// Return the minimum amount of valid operators signatures for a given
    /// operation on the token-contract.
    /// If the threshold for an operation is stored as 0, a super-majority of
    /// signers is required for the operation.
    pub fn operation_threshold(&self, operation: &String) -> Option<u8> {
        self.operations
            .get(operation)
            .copied()
            .map(|threshold| match threshold {
                0 => self.operators.len() as u8 / 2 + 1,
                _ => threshold,
            })
    }
}

// Owners operations
impl GovernanceState {
    /// Update the owner public-keys in the governance-contract.
    ///
    /// The signature message for this operations is the current owner-nonce in
    /// be-bytes appended by the serialized public-keys of the new owners.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The given set of owner keys is empty.
    /// - The given set of owner keys is larger than u8::MAX.
    /// - There are duplicates in the given owner keys.
    // TODO: write check for duplicate keys.
    pub fn set_owners(
        &mut self,
        sig: MultisigSignature,
        signers: Vec<u8>,
        new_owners: Vec<PublicKey>,
    ) {
        // panic if no owners are given
        if new_owners.is_empty() {
            panic!("{}", error::EMTPY_OWNER);
        }
        // panic if more than u8::MAX owners are given
        if new_owners.len() > u8::MAX as usize {
            panic!("{}", error::TOO_MANY_OWNERS)
        }

        // the threshold needs to be a super-majority
        let threshold = self.owners.len() as u8 / 2 + 1;

        // construct the signature message
        let mut sig_msg =
            Vec::with_capacity(u64::SIZE + new_owners.len() * PublicKey::SIZE);
        sig_msg.extend(&self.owner_nonce.to_be_bytes());
        new_owners
            .iter()
            .for_each(|pk| sig_msg.extend(&pk.to_bytes()));

        // this call will panic if the signature is not correct or the threshold
        // is not met
        self.authorize_owners(sig_msg, sig, signers, threshold);

        // update the owners to the new set
        self.owners = new_owners;

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Update the operator public-keys in the governance-contract.
    ///
    /// The signature message for this operations is the current owner-nonce in
    /// be-bytes appended by the serialized public-keys of the new owners.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The given set of operator keys is larger than u8::MAX.
    /// - There are duplicates in the given operator keys.
    // TODO: write check for duplicate keys.
    pub fn set_operators(
        &mut self,
        sig: MultisigSignature,
        signers: Vec<u8>,
        new_operators: Vec<PublicKey>,
    ) {
        // panic if more than u8::MAX owners are given
        if new_operators.len() > u8::MAX as usize {
            panic!("{}", error::TOO_MANY_OPERATORS)
        }

        // the threshold needs to be a super-majority
        let threshold = self.owners.len() as u8 / 2 + 1;

        // construct the signature message
        let mut sig_msg = Vec::with_capacity(
            u64::SIZE + new_operators.len() * PublicKey::SIZE,
        );
        sig_msg.extend(&self.owner_nonce.to_be_bytes());
        new_operators
            .iter()
            .for_each(|pk| sig_msg.extend(&pk.to_bytes()));

        // this call will panic if the signature is not correct or the threshold
        // is not met
        self.authorize_owners(sig_msg, sig, signers, threshold);

        // update the operators to the new set
        self.operators = new_operators;

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Transfer the governance stored in the state of the token-contract to a
    /// new account. After executing this call, this governance contract will no
    /// longer be authorized to do any operations on the token-contract, the
    /// new account needs to be used for authorization instead.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The nonce in `transfer_ownership` is not the `owner_nonce`
    pub fn transfer_governance(
        &mut self,
        transfer_ownership: TransferOwnership,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // check the nonce
        if transfer_ownership.nonce() != self.owner_nonce() {
            panic!("{}", error::INVALID_NONCE);
        }

        // the threshold needs to be a super-majority
        let threshold = self.owners.len() as u8 / 2 + 1;

        // check the signature
        self.authorize_owners(
            transfer_ownership.signature_message().to_vec(),
            sig,
            signers,
            threshold,
        );

        // transfer the ownership of the token-contract
        let _: () = abi::call(
            Self::TOKEN_CONTRACT,
            "transfer_ownership",
            &transfer_ownership,
        )
        .expect("transferring the governance should succeed");

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Renounce the governance of the token-contract.
    /// Note: After executing this call, neither this governance contract nor
    /// any other account will be authorized to do any operations on the
    /// token-contract.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The nonce in `renounce_ownership` is not the `owner_nonce`
    pub fn renounce_governance(
        &mut self,
        renounce_ownership: RenounceOwnership,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // check the nonce
        if renounce_ownership.nonce() != self.owner_nonce() {
            panic!("{}", error::INVALID_NONCE);
        }

        // the threshold needs to be a super-majority
        let threshold = self.owners.len() as u8 / 2 + 1;

        // check the signature
        self.authorize_owners(
            renounce_ownership.signature_message().to_vec(),
            sig,
            signers,
            threshold,
        );

        // transfer the ownership of the token-contract
        let _: () = abi::call(
            Self::TOKEN_CONTRACT,
            "renounce_ownership",
            &renounce_ownership,
        )
        .expect("renouncing the governance should succeed");

        // increment the owners nonce
        self.owner_nonce += 1;
    }
}

// Operators operations
impl GovernanceState {
    /// Execute a given operation on the token-contract.
    ///
    /// The signature message for executing an operation is the current
    /// operator-nonce in big endian, appended by the operation-arguments.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by the required threshold of
    ///   operators
    /// - The operation is not registered in the contract-state.
    pub fn execute_operation(
        &mut self,
        operation: String,
        operation_arguments: Vec<u8>,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // construct the signature message
        let mut sig_msg =
            Vec::with_capacity(u64::SIZE + operation_arguments.len());
        sig_msg.extend(&self.operator_nonce.to_be_bytes());
        sig_msg.extend(&operation_arguments);

        // verify the signature
        let threshold = self
            .operation_threshold(&operation)
            .unwrap_or_else(|| panic!("{}", error::OPERATION_NOT_FOUND));
        self.authorize_operators(sig_msg, sig, signers, threshold);

        // call the specified operation of the token-contract
        let _ = abi::call_raw(
            Self::TOKEN_CONTRACT,
            &operation,
            &operation_arguments,
        )
        .expect("calling the specified operation should succeed");

        // increment the operator nonce
        self.operator_nonce += 1;
    }

    /// Add a new operation to the stored set of operations or (if the operation
    /// already exists) update the operation threshold. A threshold of 0 means
    /// that the operation needs a super-majority of operator-signatures to
    /// be executed.
    ///
    /// The signature message for adding an operation is the current
    /// operator-nonce in big endian, appended by the operation as bytes and the
    /// threshold.
    ///
    /// Note: A super-majority of operator signatures is required to perform
    /// this action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of
    ///   operators
    pub fn set_operation(
        &mut self,
        operation: String,
        operation_threshold: u8,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // construct the signature message
        let operation_bytes = operation.as_bytes();
        let mut sig_msg =
            Vec::with_capacity(u64::SIZE + operation_bytes.len() + u8::SIZE);
        sig_msg.extend(&self.operator_nonce.to_be_bytes());
        sig_msg.extend(operation_bytes);
        sig_msg.extend(&[operation_threshold]);

        // this call will panic if the signature is not correct or not signed by
        // a super-majority of operators
        let threshold = self.operators.len() as u8 / 2 + 1;
        self.authorize_operators(sig_msg, sig, signers, threshold);

        // add the operation or update its threshold if it already exists
        self.operations.insert(operation, operation_threshold);

        // increment the operator nonce
        self.operator_nonce += 1;
    }
}

/// Access control implementation.
impl GovernanceState {
    /// Check if the aggregated signature of the given owners is valid.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect given the signature-message and public-keys
    /// - There are less signers than the specified threshold
    /// - One of the signers exceeds the owner-index
    /// - There are duplicate signers
    fn authorize_owners(
        &self,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: Vec<u8>,
        threshold: u8,
    ) {
        // panic if the signers contain duplicates
        if contains_duplicates(&signers) {
            panic!("{}", error::DUPLICATE_OWNER_KEY);
        }

        // panic if one of the signer indices is out of bounds of the
        // owner-keys
        if signers.iter().max().copied().unwrap_or_default()
            >= self.owners.len() as u8
        {
            panic!("{}", error::OWNER_NOT_FOUND);
        }

        self.authorize(sig_msg, sig, signers, threshold, true);
    }

    /// Check if the aggregated signature of the given operators is valid.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect given the signature-message and public-keys
    /// - There are less signers than the specified threshold
    /// - One of the signers exceeds the operator-index
    /// - There are duplicate signers
    fn authorize_operators(
        &self,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: Vec<u8>,
        threshold: u8,
    ) {
        // panic if the signers contain duplicates
        if contains_duplicates(&signers) {
            panic!("{}", error::DUPLICATE_OPERATOR_KEY);
        }

        // panic if one of the signer indices is out of bounds of the
        // operator-keys
        if signers.iter().max().copied().unwrap_or_default()
            >= self.operators.len() as u8
        {
            panic!("{}", error::OPERATOR_NOT_FOUND);
        }

        self.authorize(sig_msg, sig, signers, threshold, false);
    }

    /// Check if the given aggregated signature is correct given the public-keys
    /// and that the signer threshold is met.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect given the signature-message and public-keys
    /// - The public-keys are less than the specified threshold
    fn authorize(
        &self,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: Vec<u8>,
        threshold: u8,
        is_owner: bool,
    ) {
        // panic if the threshold of signers is not met
        if (signers.len() as u8) < threshold {
            panic!("{}", error::THRESHOLD_NOT_MET);
        }

        // get the signers public keys
        let public_keys = if is_owner {
            self.owners()
        } else {
            self.operators()
        };
        let signers: Vec<PublicKey> = signers
            .iter()
            .map(|index| public_keys[*index as usize])
            .collect();

        // aggregate the signers keys
        let multisig_pk = MultisigPublicKey::aggregate(&signers[..])
            .unwrap_or_else(|_| panic!("{}", error::INVALID_PUBLIC_KEY));

        // verify the signature
        if multisig_pk.verify(&sig, &sig_msg).is_err() {
            panic!("{}", error::INVALID_SIGNATURE);
        }
    }
}

/// Checks whether a given set contains duplicate elements.
fn contains_duplicates<T>(elements: impl AsRef<[T]>) -> bool
where
    T: PartialEq,
{
    let elements = elements.as_ref();
    let len = elements.len();
    for i in 0..len {
        for j in i..len {
            if elements[i] == elements[j] {
                return true;
            }
        }
    }
    false
}
