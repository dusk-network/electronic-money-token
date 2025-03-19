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
use emt_core::governance::arguments::TransferGovernance;
use emt_core::Account;

use crate::{contains_duplicates, error, supermajority};

const EMPTY: ContractId = ContractId::from_bytes([0u8; CONTRACT_ID_BYTES]);

/// The state of the token governance contract.
pub struct Governance {
    // The contract-id of the token-contract.
    token_contract: ContractId,
    // The owners, and only the owners, of a token-contract are authorized to
    // change the owners or operators of the  token-contract.
    owners: Vec<PublicKey>,
    // The nonce for the owners, initialized at 0 and strictly increasing.
    owner_nonce: u64,
    // The operators of the token-contract are authorized to execute all
    // inter-contract calls to the token-contract except changing governance.
    operators: Vec<PublicKey>,
    // The nonce for the operators, initialized at 0 and strictly increasing.
    operator_nonce: u64,
    // A map for all the inter-contract calls executable by the operators on
    // the token-contract. Each icc has a threshold of signers that
    // are required to execute the icc.
    // If the threshold for an icc is set to 0, a super-majority of
    // signers is needed.
    inter_contract_calls: BTreeMap<String, u8>,
}

/// The state of the governance contract at deployment.
pub static mut STATE: Governance = Governance::new();

/// Basic contract implementation.
impl Governance {
    /// Create a new empty instance of the governance-contract.
    #[must_use]
    const fn new() -> Self {
        Self {
            token_contract: EMPTY,
            owners: Vec::new(),
            owner_nonce: 0,
            operators: Vec::new(),
            operator_nonce: 0,
            inter_contract_calls: BTreeMap::new(),
        }
    }

    /// Initialize the governance contract state with sets of owners, operators
    /// and inter-contract calls.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The contract is already initialized.
    /// - The given set of owner keys is empty.
    /// - The given set of owner keys is larger than `u8::MAX`.
    /// - The given set of operator keys is larger than `u8::MAX`.
    /// - There are duplicate owner keys.
    /// - There are duplicate operator keys.
    /// - One of the new inter-contract calls is an icc that only owners can
    ///   authorize
    pub fn init(
        &mut self,
        token_contract: ContractId,
        owners: Vec<PublicKey>,
        operators: Vec<PublicKey>,
        icc_data: Vec<(String, u8)>,
    ) {
        // panic if the contract has already been initialized
        assert!(self.owners.is_empty(), "{}", error::ALLREADY_INITIALIZED);
        // panic if no owners are given
        assert!(!owners.is_empty(), "{}", error::EMTPY_OWNER);
        // panic if there are more than `u8::MAX` owners
        assert!(
            !owners.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_OWNERS
        );
        // panic if there are more than `u8::MAX` operators
        assert!(
            !owners.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_OPERATORS
        );
        // panic if there are duplicate owners
        assert!(!contains_duplicates(&owners), "{}", error::DUPLICATE_OWNER);

        // initialize token-contract and owners
        self.token_contract = token_contract;
        self.owners = owners;

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
        let mut inter_contract_calls = BTreeMap::new();
        for (icc, threshold) in icc_data {
            // panic if inter-contract calls that need owner approval are
            // added
            assert!(
                !Self::OWNER_ICC.contains(&icc.as_str()),
                "{}",
                error::UNAUTHORIZED_ICC,
            );
            inter_contract_calls.insert(icc, threshold);
        }
        self.inter_contract_calls = inter_contract_calls;
    }

    /// Return the name of the contract.
    #[must_use]
    pub fn name(&self) -> String {
        String::from("Token Governance Sample")
    }

    /// Return the symbol of the contract.
    #[must_use]
    pub fn symbol(&self) -> String {
        String::from("TGS")
    }

    /// Return the linked token-contract.
    #[must_use]
    pub fn token_contract(&self) -> ContractId {
        self.token_contract
    }

    /// Return the current owners stored in the governance contract.
    #[must_use]
    pub fn owners(&self) -> Vec<PublicKey> {
        self.owners.clone()
    }

    /// Return the current nonce for executing anything that requires a
    /// signature of the owners.
    #[must_use]
    pub fn owner_nonce(&self) -> u64 {
        self.owner_nonce
    }

    /// Return the current operators stored in the governance contract.
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

    /// Return the minimum amount of operators that must sign a given icc
    /// in order for it to be executed on the token-contract.
    /// If the stored threshold for an icc is 0, a super-majority of
    /// signers is required for executing that icc.
    #[must_use]
    pub fn icc_threshold(&self, icc: &str) -> Option<u8> {
        self.inter_contract_calls
            .get(icc)
            .copied()
            .map(|threshold| match threshold {
                0 => supermajority(self.owners.len()),
                _ => threshold,
            })
    }
}

// Methods that need the owners' approval.
impl Governance {
    /// Since the token-contract will execute every inter-contract call that
    /// comes from the governance contract, every icc that need
    /// authorization by the owners **must** be excluded from the
    /// inter-contract calls that the operators need to authorize.
    const OWNER_ICC: [&'static str; 2] = [
        // "set_owners" and "set_operators" also need owners approval but
        // because they don't contain a call to the token-contract, they
        // don't need to be added here.
        "transfer_governance",
        "renounce_governance",
    ];

    /// Update the token-contract in the governance-contract.
    /// This might be useful when the token-contract changes but the governance
    /// remains the same.
    ///
    /// The signature message for this inter-contract calls is the current
    /// owner-nonce in be-bytes appended by the new token-contract `ContractId`.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    pub fn set_token_contract(
        &mut self,
        new_token_contract: ContractId,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.owners.len());

        // construct the signature message
        let mut sig_msg = Vec::with_capacity(u64::SIZE + CONTRACT_ID_BYTES);
        sig_msg.extend(&self.owner_nonce.to_be_bytes());
        sig_msg.extend(&new_token_contract.to_bytes());

        // this call will panic if the signature is not correct or the threshold
        // is not met
        self.authorize_owners(threshold, &sig_msg, sig, signers);
    }

    /// Update the owner public-keys in the governance-contract.
    ///
    /// The signature message for this inter-contract calls is the current
    /// owner-nonce in be-bytes appended by the serialized public-keys of
    /// the new owners.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The new set of owner keys is empty.
    /// - The new set of owner keys is larger than `u8::MAX`.
    /// - The new set of owner keys contains duplicates.
    // NOTE: It might be that having `add_owner` and `remove_owner` functions to
    // add or remove a single owner make more sense from a user perspective. I
    // however opted for a general `set_owners` method which can be used for
    // both removing and adding owner, in order to reduce the byte-code of the
    // contract. The functionality to remove and add a single owner can be
    // implemented by the wallet application.
    // The same applies for `set_operators`.
    pub fn set_owners(
        &mut self,
        new_owners: Vec<PublicKey>,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // panic if no owners are given
        assert!(!new_owners.is_empty(), "{}", error::EMTPY_OWNER);
        // panic if more than `u8::MAX` owners are given
        assert!(
            !new_owners.len() > u8::MAX as usize,
            "{}",
            error::TOO_MANY_OWNERS
        );
        // panic if there are duplicate owners
        assert!(
            !contains_duplicates(&new_owners),
            "{}",
            error::DUPLICATE_OWNER
        );

        // the threshold needs to be a super-majority
        let threshold = supermajority(self.owners.len());

        // construct the signature message
        let mut sig_msg =
            Vec::with_capacity(u64::SIZE + new_owners.len() * PublicKey::SIZE);
        sig_msg.extend(&self.owner_nonce.to_be_bytes());
        new_owners
            .iter()
            .for_each(|pk| sig_msg.extend(&pk.to_bytes()));

        // this call will panic if the signature is not correct or the threshold
        // is not met
        self.authorize_owners(threshold, &sig_msg, sig, signers);

        // update the owners to the new set
        self.owners = new_owners;

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Update the operator public-keys in the governance-contract.
    ///
    /// The signature message for this inter-contract calls is the current
    /// owner-nonce in be-bytes appended by the serialized public-keys of
    /// the new operators.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The new set of operator keys is larger than `u8::MAX`.
    /// - The new set of operator keys contains duplicates.
    // NOTE: See `set_owner`.
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
        let threshold = supermajority(self.owners.len());

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
        self.authorize_owners(threshold, &sig_msg, sig, signers);

        // update the operators to the new set
        self.operators = new_operators;

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Authorize the transfer of the governance stored in the state of the
    /// token-contract to a new account. After executing this call, this
    /// governance contract will **no longer be authorized** to do any
    /// inter-contract calls on the token-contract and the new account needs to
    /// be used for authorization instead.
    ///
    ///
    /// The signature message for transferring the governance of the
    /// token-contract is the current owner-nonce in big endian appended by the
    /// new governance.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    /// - The nonce given in `transfer_ownership` is not same as the
    ///   `owner_nonce` in the state of the governance contract.
    pub fn transfer_governance(
        &mut self,
        new_governance: Account,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.owners.len());

        // check the signature
        let mut sig_msg = Vec::with_capacity(u64::SIZE + Account::SIZE);
        sig_msg.extend(&self.owner_nonce.to_be_bytes());
        sig_msg.extend(&new_governance.to_bytes());
        self.authorize_owners(threshold, &sig_msg, sig, signers);

        // transfer the ownership of the token-contract
        let _: () = abi::call(
            self.token_contract(),
            "transfer_ownership",
            &TransferGovernance::new(new_governance),
        )
        .expect("transferring the governance should succeed");

        // increment the owners nonce
        self.owner_nonce += 1;
    }

    /// Renounce the governance of the token-contract.
    /// Note: After executing this call, neither this governance contract nor
    /// any other account will be authorized to do any inter-contract calls on
    /// the token-contract.
    ///
    /// The signature message for renouncing the governance of the
    /// token-contract is the current owner-nonce in big endian.
    ///
    /// Note: A super-majority of owner signatures is required to perform this
    /// action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of owners
    pub fn renounce_governance(
        &mut self,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // the threshold needs to be a super-majority
        let threshold = supermajority(self.owners.len());

        // check the signature
        let sig_msg = self.owner_nonce().to_be_bytes();
        self.authorize_owners(threshold, &sig_msg, sig, signers);

        // transfer the ownership of the token-contract
        let _: () = abi::call(self.token_contract(), "renounce_ownership", &())
            .expect("renouncing the governance should succeed");

        // increment the owners nonce
        self.owner_nonce += 1;
    }
}

// Methods that need the operators' approval
impl Governance {
    /// Execute a given inter-contract call on the token-contract.
    ///
    /// The signature message for executing an icc is the current
    /// operator-nonce in big endian, appended by the icc-name and -arguments.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by the required threshold of
    ///   operators
    /// - The icc is not registered in the contract-state.
    pub fn execute_icc(
        &mut self,
        icc_name: String,
        icc_arguments: Vec<u8>,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // construct the signature message
        let mut sig_msg = Vec::with_capacity(
            u64::SIZE + icc_name.len() + icc_arguments.len(),
        );
        sig_msg.extend(&self.operator_nonce.to_be_bytes());
        sig_msg.extend(&icc_arguments);

        // verify the signature
        let threshold = self
            .icc_threshold(&icc_name)
            .unwrap_or_else(|| panic!("{}", error::ICC_NOT_FOUND));
        self.authorize_operators(threshold, &sig_msg, sig, signers);

        // call the specified method of the token-contract
        let _ = abi::call_raw(self.token_contract(), &icc_name, &icc_arguments)
            .expect("calling the specified icc should succeed");

        // increment the operator nonce
        self.operator_nonce += 1;
    }

    /// Add a new icc to the stored set of inter-contract calls or (if the
    /// icc already exists) update the icc threshold. A
    /// threshold of 0 means that the icc needs a super-majority of
    /// operator-signatures to be executed.
    ///
    /// The signature message for adding an icc is the current
    /// operator-nonce in big endian, appended by the icc-name as bytes and the
    /// threshold.
    ///
    /// Note: A super-majority of operator signatures is required to perform
    /// this action.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The signature is incorrect or not signed by a super-majority of
    ///   operators
    /// - The new icc is an icc that only owners can authorize
    pub fn set_inter_contract_call(
        &mut self,
        icc_name: String,
        icc_threshold: u8,
        sig: MultisigSignature,
        signers: Vec<u8>,
    ) {
        // panic if inter-contract calls that need owner approval are added
        assert!(
            !Self::OWNER_ICC.contains(&icc_name.as_str()),
            "{}",
            error::UNAUTHORIZED_ICC,
        );
        // construct the signature message
        let icc_bytes = icc_name.as_bytes();
        let mut sig_msg =
            Vec::with_capacity(u64::SIZE + icc_bytes.len() + u8::SIZE);
        sig_msg.extend(&self.operator_nonce.to_be_bytes());
        sig_msg.extend(icc_bytes);
        sig_msg.extend(&[icc_threshold]);

        // this call will panic if the signature is not correct or not signed by
        // a super-majority of operators
        let threshold = supermajority(self.operators.len());
        self.authorize_operators(threshold, &sig_msg, sig, signers);

        // add the icc or update its threshold if it already exists
        self.inter_contract_calls.insert(icc_name, icc_threshold);

        // increment the operator nonce
        self.operator_nonce += 1;
    }
}

/// Access control implementation.
impl Governance {
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
        threshold: u8,
        sig_msg: impl AsRef<[u8]>,
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
    fn authorize_operators(
        &self,
        threshold: u8,
        sig_msg: impl AsRef<[u8]>,
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
        sig_msg: impl AsRef<[u8]>,
        sig: MultisigSignature,
        signers: impl AsRef<[u8]>,
        is_owner: bool,
    ) {
        let signer_idx = signers.as_ref();
        let sig_msg = sig_msg.as_ref();

        // panic if the signers contain duplicates
        assert!(
            !contains_duplicates(signer_idx),
            "{}",
            error::DUPLICATE_SIGNER
        );

        // panic if one of the signer's indices doesn't exist
        assert!(
            (signer_idx.iter().max().copied().unwrap_or_default() as usize)
                < self.owners.len(),
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
        let public_keys = if is_owner {
            self.owners()
        } else {
            self.operators()
        };
        let signers: Vec<PublicKey> = signer_idx
            .iter()
            .map(|index| public_keys[*index as usize])
            .collect();

        // aggregate the signers keys
        let multisig_pk = MultisigPublicKey::aggregate(&signers[..])
            .unwrap_or_else(|_| panic!("{}", error::INVALID_PUBLIC_KEY));

        // verify the signature
        assert!(
            multisig_pk.verify(&sig, sig_msg).is_ok(),
            "{}",
            error::INVALID_SIGNATURE
        );
    }
}
