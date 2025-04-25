// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::abi::{self, ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::{MultisigSignature, PublicKey};
use emt_core::access_control::{error, events, signature_messages};
use emt_core::Account;

use crate::{contains_duplicates, supermajority};

const EMPTY: ContractId = ContractId::from_bytes([0u8; CONTRACT_ID_BYTES]);

/// The state of the token access-control-contract.
pub struct AccessControl {
    // The contract-id of the token-contract.
    token_contract: ContractId,
    // Only the admins of a token-contract are authorized to change the admins
    // or operators of the token-contract.
    admins: Vec<PublicKey>,
    // The nonce for the admins, initialized at 0 and strictly increasing.
    admin_nonce: u64,
    // The operators of the token-contract are authorized to execute all
    // inter-contract calls to the token-contract except changing ownership.
    operators: Vec<PublicKey>,
    // The nonce for the operators, initialized at 0 and strictly increasing.
    operator_nonce: u64,
    // A map for all the inter-contract calls to the token-contract, that are
    // executable by the operators. Each call has a threshold of signers
    // that are required for its execution. If the threshold for a call is set
    // to 0, a super-majority of signers is needed.
    operator_token_calls: BTreeMap<String, u8>,
}

/// The state of the access-control-contract at deployment.
pub static mut STATE: AccessControl = AccessControl::new();

/// Basic contract implementation.
impl AccessControl {
    /// Create a new empty instance of the access-control-contract.
    #[must_use]
    const fn new() -> Self {
        Self {
            token_contract: EMPTY,
            admins: Vec::new(),
            admin_nonce: 0,
            operators: Vec::new(),
            operator_nonce: 0,
            operator_token_calls: BTreeMap::new(),
        }
    }

    /// Initialize the access-control-contract state with sets of admins,
    /// operators and inter-contract calls.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The contract is already initialized.
    /// - The given set of admin keys is empty.
    /// - The given set of admin keys is larger than `u8::MAX`.
    /// - The given set of operator keys is larger than `u8::MAX`.
    /// - There are duplicate admin keys.
    /// - There are duplicate operator keys.
    /// - One of the new operator-calls is reserved for admin-calls.
    pub fn init(
        &mut self,
        token_contract: ContractId,
        admins: Vec<PublicKey>,
        operators: Vec<PublicKey>,
        operator_token_call_data: Vec<(String, u8)>,
    ) {
        // panic if the contract has already been initialized
        assert!(self.admins.is_empty(), "{}", error::ALLREADY_INITIALIZED);
        // panic if no admins are given
        assert!(!admins.is_empty(), "{}", error::EMPTY_ADMINS);
        // panic if there are more than `u8::MAX` admins
        assert!(
            !admins.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_ADMINS
        );
        // panic if there are more than `u8::MAX` operators
        assert!(
            !admins.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_OPERATORS
        );
        // panic if there are duplicate admins
        assert!(!contains_duplicates(&admins), "{}", error::DUPLICATE_ADMINS);

        // initialize token-contract and admins
        self.token_contract = token_contract;
        self.admins = admins;

        // initialize operators (if any)
        if !operators.is_empty() {
            // panic if there are duplicate operators
            assert!(
                !contains_duplicates(&operators),
                "{}",
                error::DUPLICATE_OPERATOR
            );
            self.operators = operators;
        }

        // initialize inter-contract calls (if any)
        let mut operator_token_calls = BTreeMap::new();
        for (call_name, signature_threshold) in operator_token_call_data {
            // panic if inter-contract calls that need admin approval are
            // added
            assert!(
                !Self::ADMIN_TOKEN_CALLS.contains(&call_name.as_str()),
                "{}",
                error::UNAUTHORIZED_TOKEN_CALL,
            );
            operator_token_calls.insert(call_name, signature_threshold);
        }
        self.operator_token_calls = operator_token_calls;
    }

    /// Return the linked token-contract.
    #[must_use]
    pub fn token_contract(&self) -> ContractId {
        self.token_contract
    }

    /// Return the current admins stored in the access-control-contract.
    #[must_use]
    pub fn admins(&self) -> Vec<PublicKey> {
        self.admins.clone()
    }

    /// Return the current nonce for executing anything that requires a
    /// signature of the admins.
    #[must_use]
    pub fn admin_nonce(&self) -> u64 {
        self.admin_nonce
    }

    /// Return the current operators stored in the access-control-contract.
    #[must_use]
    pub fn operators(&self) -> Vec<PublicKey> {
        self.operators.clone()
    }

    /// Return the current nonce for executing anything that requires a
    /// signature of the operators.
    #[must_use]
    pub fn operator_nonce(&self) -> u64 {
        self.operator_nonce
    }

    /// Return the minimum amount of operators that must sign a given call to
    /// the token-contract in order for it to be executed.
    /// If the stored signature threshold for a call is 0, the super-majority is
    /// calculated and returned.
    /// Returns `None` if the `call_name` is not a registered operators
    /// token-contract call.
    #[must_use]
    pub fn operator_signature_threshold(&self, call_name: &str) -> Option<u8> {
        self.operator_token_calls
            .get(call_name)
            .copied()
            .map(|threshold| match threshold {
                0 => supermajority(self.operators.len()),
                _ => threshold,
            })
    }
}

// Methods that need the admins' approval.
impl AccessControl {
    /// Since the token-contract will execute every inter-contract call that
    /// comes from the access-control-contract, every token-contract call that
    /// need authorization by the admins **must** be excluded from the calls
    /// that the operators need to authorize.
    const ADMIN_TOKEN_CALLS: [&'static str; 2] = [
        // 'set_token_contract`, `set_admins` and `set_operators` also need
        // admins approval but because they don't contain a call to the
        // token-contract, they don't need to be added here.
        "transfer_ownership",
        "renounce_ownership",
    ];

    /// Update the token-contract in the access-control-contract and return the
    /// old token-contract ID.
    /// This allows for changing the token-contract while keeping the same
    /// access-control.
    ///
    /// The signature message for this inter-contract call is the current
    /// admin-nonce in be-bytes appended by the new token-contract `ContractId`.
    ///
    /// Note: A super-majority of admin signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of admins
    #[must_use]
    pub fn set_token_contract(
        &mut self,
        new_token_contract: ContractId,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) -> ContractId {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.admins.len());

        // check the signature
        let sig_msg = signature_messages::set_token_contract(
            self.admin_nonce,
            &new_token_contract,
        );
        self.authorize_admins(threshold, sig_msg, sig, signers);

        // increment the admins nonce
        self.admin_nonce += 1;

        // replace the token-contract
        let old_token_contract =
            core::mem::replace(&mut self.token_contract, new_token_contract);

        // alert network of the changes to the state
        abi::emit(
            events::UpdateToken::TOPIC,
            events::UpdateToken {
                contract: new_token_contract,
            },
        );

        // return the old token-contract id
        old_token_contract
    }

    /// Update the admin public-keys in the access-control-contract.
    ///
    /// The signature message for this inter-contract call is the current
    /// admin-nonce in be-bytes appended by the serialized public-keys of
    /// the new admins.
    ///
    /// Note: A super-majority of admin signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of admins
    /// - The new set of admin keys is empty.
    /// - The new set of admin keys is larger than `u8::MAX`.
    /// - The new set of admin keys contains duplicates.
    pub fn set_admins(
        &mut self,
        new_admins: Vec<PublicKey>,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // panic if no admins are given
        assert!(!new_admins.is_empty(), "{}", error::EMPTY_ADMINS);
        // panic if more than `u8::MAX` admins are given
        assert!(
            !new_admins.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_ADMINS
        );
        // panic if there are duplicate admins
        assert!(
            !contains_duplicates(&new_admins),
            "{}",
            error::DUPLICATE_ADMINS
        );

        // the threshold needs to be a super-majority
        let threshold = supermajority(self.admins.len());

        // check the signature
        let sig_msg =
            signature_messages::set_admins(self.admin_nonce, &new_admins);
        self.authorize_admins(threshold, sig_msg, sig, signers);

        // update the admins to the new set
        self.admins = new_admins.clone();

        // increment the admins nonce
        self.admin_nonce += 1;

        // alert network of the changes to the state
        abi::emit(
            events::UpdatePublicKeys::NEW_ADMINS,
            events::UpdatePublicKeys { pks: new_admins },
        );
    }

    /// Update the operator public-keys in the access-control-contract.
    ///
    /// The signature message for this inter-contract call is the current
    /// admin-nonce in be-bytes appended by the serialized public-keys of
    /// the new operators.
    ///
    /// Note: A super-majority of admin signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of admins
    /// - The new set of operator keys is larger than `u8::MAX`.
    /// - The new set of operator keys contains duplicates.
    pub fn set_operators(
        &mut self,
        new_operators: Vec<PublicKey>,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // panic if more than `u8::MAX` operators are given
        assert!(
            !new_operators.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_OPERATORS
        );
        // panic if there are duplicate operators
        assert!(
            !contains_duplicates(&new_operators),
            "{}",
            error::DUPLICATE_OPERATOR
        );

        // the threshold needs to be a super-majority
        let threshold = supermajority(self.admins.len());

        // check the signature
        let sig_msg =
            signature_messages::set_operators(self.admin_nonce, &new_operators);
        self.authorize_admins(threshold, sig_msg, sig, signers);

        // update the operators to the new set
        self.operators = new_operators.clone();

        // increment the admins nonce
        self.admin_nonce += 1;

        // alert network of the changes to the state
        abi::emit(
            events::UpdatePublicKeys::NEW_OPERATORS,
            events::UpdatePublicKeys { pks: new_operators },
        );
    }

    /// Authorize the transfer of the ownership stored in the state of the
    /// token-contract to a new account. After executing this call, this
    /// ownership contract will **no longer be authorized** to do any
    /// inter-contract calls on the token-contract and the new account needs to
    /// be used for authorization instead.
    ///
    /// The signature message for transferring the ownership of the
    /// token-contract is the current admin-nonce in big endian appended by the
    /// new ownership.
    ///
    /// Note: A super-majority of admin signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of admins
    pub fn transfer_ownership(
        &mut self,
        new_ownership: Account,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.admins.len());

        // check the signature
        let sig_msg = signature_messages::transfer_ownership(
            self.admin_nonce,
            &new_ownership,
        );
        self.authorize_admins(threshold, sig_msg, sig, signers);

        // transfer the ownership of the token-contract
        let _: () = abi::call(
            self.token_contract(),
            "transfer_ownership",
            &new_ownership,
        )
        .expect("transferring the ownership should succeed");

        // increment the admins nonce
        self.admin_nonce += 1;
    }

    /// Renounce the ownership of the token-contract.
    /// Note: After executing this call, neither this ownership-contract nor
    /// any other account will be authorized to call any functions on the
    /// token-contract that require authorization from the ownership account.
    ///
    /// The signature message for renouncing the ownership of the
    /// token-contract is the current admin-nonce in big endian.
    ///
    /// Note: A super-majority of admin signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of admins
    pub fn renounce_ownership(
        &mut self,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.admins.len());

        // check the signature
        let sig_msg = signature_messages::renounce_ownership(self.admin_nonce);
        self.authorize_admins(threshold, sig_msg, sig, signers);

        // removing the ownership on the token-contract
        let _: () = abi::call(self.token_contract(), "renounce_ownership", &())
            .expect("renouncing the ownership should succeed");

        // increment the admins nonce
        self.admin_nonce += 1;
    }
}

// Methods that need the operators' approval
impl AccessControl {
    /// Execute a call to the token-contract, that doesn't require admin's
    /// approval.
    ///
    /// The signature message for executing an operator approved token-contract
    /// call is the current operator-nonce in big endian, appended by the
    /// call-name and -arguments.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by the required threshold of
    ///   operators
    /// - The `call_name` is not registered in the contract-state.
    pub fn operator_token_call(
        &mut self,
        call_name: &str,
        call_arguments: &[u8],
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // get the stored threshold for this operation
        let threshold = self
            .operator_signature_threshold(call_name)
            .unwrap_or_else(|| panic!("{}", error::TOKEN_CALL_NOT_FOUND));

        // check the signature
        let sig_msg = signature_messages::operator_token_call(
            self.operator_nonce,
            call_name,
            call_arguments,
        );
        self.authorize_operators(threshold, sig_msg, sig, signers);

        // call the specified method of the token-contract
        let _ = abi::call_raw(self.token_contract(), call_name, call_arguments)
            .expect(error::OPERATOR_TOKEN_CALL_PANIC);

        // increment the operator nonce
        self.operator_nonce += 1;
    }

    /// Add a new call to the stored set of operator calls or update the call
    /// threshold if it already exists. A threshold of 0 means that the call
    /// needs a super-majority of operator-signatures to be executed.
    ///
    /// The signature message for adding an operator token-contract call is the
    /// current operator-nonce in big endian, appended by the call-name as
    /// bytes and the signature threshold for that call.
    ///
    /// Note: A super-majority of operator signatures is required to perform
    /// this action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of
    ///   operators
    /// - The new operator-calls is reserved for admin-calls.
    pub fn set_operator_token_call(
        &mut self,
        call_name: String,
        operator_signature_threshold: u8,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // panic if inter-contract calls that need admin approval are added
        assert!(
            !Self::ADMIN_TOKEN_CALLS.contains(&call_name.as_str()),
            "{}",
            error::UNAUTHORIZED_TOKEN_CALL,
        );

        // the threshold needs to be a super-majority
        let threshold = supermajority(self.operators.len());

        // check the signature
        let sig_msg = signature_messages::set_operator_token_call(
            self.operator_nonce,
            call_name.as_str(),
            operator_signature_threshold,
        );
        self.authorize_operators(threshold, sig_msg, sig, signers);

        // add the call or update its threshold if it already exists
        self.operator_token_calls
            .insert(call_name.clone(), operator_signature_threshold);

        // increment the operator nonce
        self.operator_nonce += 1;

        // alert network of the changes to the state
        abi::emit(
            events::UpdateTokenCall::TOPIC,
            events::UpdateTokenCall {
                call_name,
                operator_signature_threshold,
            },
        );
    }
}

/// Access control implementation.
impl AccessControl {
    /// Check if the aggregated signature of the given admins is valid.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect given the signature-message and public-keys
    /// - There are less signers than the specified threshold
    /// - One of the signers exceeds the admin-index
    /// - There are duplicate signers
    pub fn authorize_admins(
        &self,
        threshold: u8,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: impl AsRef<[u8]>,
    ) {
        self.authorize(threshold, sig_msg, sig, signers, true);
    }

    /// Check if the aggregated signature of the given operators is valid.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect given the signature-message and public-keys
    /// - There are less signers than the specified threshold
    /// - One of the signers exceeds the operator-index
    /// - There are duplicate signers
    pub fn authorize_operators(
        &self,
        threshold: u8,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: impl AsRef<[u8]>,
    ) {
        self.authorize(threshold, sig_msg, sig, signers, false);
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
        threshold: u8,
        sig_msg: Vec<u8>,
        sig: MultisigSignature,
        signers: impl AsRef<[u8]>,
        is_admin: bool,
    ) {
        let signer_idx = signers.as_ref();

        // at this point the threshold should never be 0
        assert!(threshold > 0, "{}", error::THRESHOLD_ZERO);

        // panic if the signers contain duplicates
        assert!(
            !contains_duplicates(signer_idx),
            "{}",
            error::DUPLICATE_SIGNER
        );

        // panic if one of the signer's indices doesn't exist
        assert!(
            (signer_idx.iter().max().copied().unwrap_or_default() as usize)
                < self.admins.len(),
            "{}",
            error::SIGNER_NOT_FOUND
        );

        // panic if the threshold of signers is not met
        assert!(
            signer_idx.len() >= threshold as usize,
            "{}",
            error::THRESHOLD_NOT_MET
        );

        // get the signers public keys
        let public_keys = if is_admin {
            self.admins()
        } else {
            self.operators()
        };
        let signers: Vec<PublicKey> = signer_idx
            .iter()
            .map(|index| public_keys[*index as usize])
            .collect();

        // verify the signature
        assert!(
            abi::verify_bls_multisig(sig_msg, signers, sig),
            "{}",
            error::INVALID_SIGNATURE
        );
    }
}
