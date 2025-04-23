// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use core::mem::size_of;

use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::PublicKey;

use crate::Account;

// the max account size is the public key raw size `G2Affine::RAW_SIZE`
const ACCOUNT_MAX_SIZE: usize = 194;

/// The signature message for changing the token-contract is the current
/// admin-nonce in be-bytes appended by the new token-contract `ContractId`.
#[must_use]
pub fn set_token_contract(
    admin_nonce: u64,
    new_token_contract: &ContractId,
) -> Vec<u8> {
    let mut sig_msg = Vec::with_capacity(size_of::<u64>() + CONTRACT_ID_BYTES);
    sig_msg.extend(&admin_nonce.to_be_bytes());
    sig_msg.extend(&new_token_contract.to_bytes());

    sig_msg
}

/// The signature message for changing the admins, is the current
/// admin-nonce in be-bytes appended by the serialized public-keys of
/// the new admins.
#[must_use]
pub fn set_admins(
    admin_nonce: u64,
    new_admins: impl AsRef<[PublicKey]>,
) -> Vec<u8> {
    set_admin_or_operator(admin_nonce, new_admins)
}

/// The signature message for changing the operators, is the current
/// admin-nonce in be-bytes appended by the serialized public-keys of
/// the new operators.
#[must_use]
pub fn set_operators(
    admin_nonce: u64,
    new_operators: impl AsRef<[PublicKey]>,
) -> Vec<u8> {
    set_admin_or_operator(admin_nonce, new_operators)
}

#[must_use]
fn set_admin_or_operator(
    admin_nonce: u64,
    new_keys: impl AsRef<[PublicKey]>,
) -> Vec<u8> {
    let new_keys = new_keys.as_ref();
    let mut sig_msg = Vec::with_capacity(
        size_of::<u64>() + new_keys.len() * ACCOUNT_MAX_SIZE,
    );
    sig_msg.extend(&admin_nonce.to_be_bytes());
    new_keys
        .iter()
        .for_each(|pk| sig_msg.extend(&pk.to_raw_bytes()));

    sig_msg
}

/// The signature message for transferring the governance of the
/// token-contract is the current admin-nonce in big endian appended by the
/// new governance.
#[must_use]
pub fn transfer_governance(
    admin_nonce: u64,
    new_governance: &Account,
) -> Vec<u8> {
    let mut sig_msg = Vec::with_capacity(size_of::<u64>() + ACCOUNT_MAX_SIZE);
    sig_msg.extend(&admin_nonce.to_be_bytes());
    sig_msg.extend(&account_to_bytes(new_governance));

    sig_msg
}

#[must_use]
fn account_to_bytes(account: &Account) -> Vec<u8> {
    match account {
        Account::External(pk) => pk.to_raw_bytes().to_vec(),
        Account::Contract(id) => id.to_bytes().to_vec(),
    }
}

/// The signature message for renouncing the governance of the
/// token-contract is the current admin-nonce in big endian.
#[must_use]
pub fn renounce_governance(admin_nonce: u64) -> Vec<u8> {
    admin_nonce.to_be_bytes().into()
}

/// The signature message for executing an operator approved token-contract
/// call is the current operator-nonce in big endian, appended by the
/// call-name and -arguments.
#[must_use]
pub fn operator_token_call(
    operator_nonce: u64,
    call_name: &str,
    call_arguments: impl AsRef<[u8]>,
) -> Vec<u8> {
    let call_arguments = call_arguments.as_ref();
    let call_name_bytes = call_name.as_bytes();
    let mut sig_msg = Vec::with_capacity(
        size_of::<u64>() + call_name_bytes.len() + call_arguments.len(),
    );
    sig_msg.extend(&operator_nonce.to_be_bytes());
    sig_msg.extend(call_name_bytes);
    sig_msg.extend(call_arguments);

    sig_msg
}

/// The signature message for adding an operator token-contract call is the
/// current operator-nonce in big endian, appended by the call-name as
/// bytes and the signature threshold for that call.
#[must_use]
pub fn set_operator_token_call(
    operator_nonce: u64,
    call_name: &str,
    operator_signature_threshold: u8,
) -> Vec<u8> {
    let call_name_bytes = call_name.as_bytes();
    let mut sig_msg = Vec::with_capacity(
        size_of::<u64>() + call_name_bytes.len() + size_of::<u8>(),
    );
    sig_msg.extend(&operator_nonce.to_be_bytes());
    sig_msg.extend(call_name_bytes);
    sig_msg.extend(&[operator_signature_threshold]);

    sig_msg
}
