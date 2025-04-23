// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The access-control-contract for electronic money tokens.

#![no_std]
#![deny(unused_extern_crates)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]

#[cfg(target_family = "wasm")]
pub(crate) mod state;

#[cfg(target_family = "wasm")]
mod wasm {
    extern crate alloc;
    use alloc::string::String;
    use alloc::vec::Vec;

    use dusk_core::{abi, signatures::bls::MultisigSignature};

    use crate::state::STATE;

    /*
     * Basic contract implementation.
     */

    #[no_mangle]
    unsafe extern "C" fn init(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(token_contract, owner, operator, operator_token_call_data)| {
                STATE.init(
                    token_contract,
                    owner,
                    operator,
                    operator_token_call_data,
                );
            },
        )
    }

    #[no_mangle]
    unsafe extern "C" fn token_contract(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.token_contract())
    }

    #[no_mangle]
    unsafe extern "C" fn owners(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.owners())
    }

    #[no_mangle]
    unsafe extern "C" fn owner_nonce(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.owner_nonce())
    }

    #[no_mangle]
    unsafe extern "C" fn operators(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.operators())
    }

    #[no_mangle]
    unsafe extern "C" fn operator_nonce(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.operator_nonce())
    }

    #[no_mangle]
    unsafe extern "C" fn operator_signature_threshold(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |call_name: String| {
            STATE.operator_signature_threshold(call_name.as_str())
        })
    }

    #[no_mangle]
    unsafe extern "C" fn authorize_owners(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(threshold, sig_msg, sig, signers): (
                u8,
                Vec<u8>,
                MultisigSignature,
                Vec<u8>,
            )| {
                STATE.authorize_owners(threshold, sig_msg, sig, signers);
            },
        )
    }

    #[no_mangle]
    unsafe extern "C" fn authorize_operators(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(threshold, sig_msg, sig, signers): (
                u8,
                Vec<u8>,
                MultisigSignature,
                Vec<u8>,
            )| {
                STATE.authorize_operators(threshold, sig_msg, sig, signers);
            },
        )
    }

    /*
     * Functions that need the owners' approval.
     */

    #[no_mangle]
    unsafe extern "C" fn set_token_contract(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_token_contract, sig, signers)| {
            STATE.set_token_contract(new_token_contract, sig, signers)
        })
    }

    #[no_mangle]
    unsafe extern "C" fn set_owners(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_owners, sig, signers)| {
            STATE.set_owners(new_owners, sig, signers);
        })
    }

    #[no_mangle]
    unsafe extern "C" fn set_operators(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_operators, sig, signers)| {
            STATE.set_operators(new_operators, sig, signers);
        })
    }

    #[no_mangle]
    unsafe extern "C" fn transfer_governance(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_governance, sig, signers)| {
            STATE.transfer_governance(new_governance, sig, signers);
        })
    }

    #[no_mangle]
    unsafe extern "C" fn renounce_governance(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(sig, signers)| {
            STATE.renounce_governance(sig, signers);
        })
    }

    /*
     * Functions that need the operators' approval.
     */

    #[no_mangle]
    unsafe extern "C" fn operator_token_call(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(call_name, call_arguments, sig, signers): (
                String,
                Vec<u8>,
                MultisigSignature,
                Vec<u8>,
            )| {
                STATE.operator_token_call(
                    call_name.as_str(),
                    &call_arguments,
                    sig,
                    signers,
                );
            },
        )
    }

    #[no_mangle]
    unsafe extern "C" fn set_operator_token_call(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(call_name, operator_signature_threshold, sig, signers)| {
                STATE.set_operator_token_call(
                    call_name,
                    operator_signature_threshold,
                    sig,
                    signers,
                );
            },
        )
    }
}

/// Calculate the super-majority for the given amount eligible signers.
///
/// # Panics
/// This function panics if the amount is 0 or larger than `u8::MAX`
#[must_use]
fn supermajority(amt: usize) -> u8 {
    assert!(amt > 0, "Cannot calculate supermajority of 0");
    let amt = u8::try_from(amt).expect(
        "Neither owner nor operator key sets are larger than `u8::MAX`",
    );
    amt / 2 + 1
}

/// Checks whether a given set contains duplicate elements.
#[must_use]
fn contains_duplicates<T>(elements: impl AsRef<[T]>) -> bool
where
    T: PartialEq,
{
    let elements = elements.as_ref();
    let len = elements.len();
    if len > 0 {
        for i in 0..len - 1 {
            for j in i + 1..len {
                if elements[i] == elements[j] {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supermajority() {
        assert_eq!(supermajority(1), 1);
        assert_eq!(supermajority(2), 2);
        assert_eq!(supermajority(3), 2);
        assert_eq!(supermajority(4), 3);
        assert_eq!(supermajority(5), 3);
        assert_eq!(supermajority(42), 22);
        assert_eq!(supermajority(101), 51);
        assert_eq!(supermajority(u8::MAX as usize), 128);
    }

    #[test]
    #[should_panic(expected = "Cannot calculate supermajority of 0")]
    fn test_supermajority_lower_bound() {
        let _ = supermajority(0);
    }

    #[test]
    #[should_panic(
        expected = "Neither owner nor operator key sets are larger than `u8::MAX`"
    )]
    fn test_supermajority_upper_bound() {
        let _ = supermajority(u8::MAX as usize + 1);
    }

    #[test]
    fn test_contains_duplicates() {
        // test sets without duplicates
        let empty: [u32; 0] = [];
        assert!(!contains_duplicates(empty));
        assert!(!contains_duplicates([1]));
        assert!(!contains_duplicates([1, 2, 3]));
        assert!(!contains_duplicates([1, 2, 3, 4, 5]));

        // test sets without duplicates
        assert!(contains_duplicates([1, 1]));
        assert!(contains_duplicates([1, 2, 2]));
        assert!(contains_duplicates([1, 1, 2, 3]));
        assert!(contains_duplicates([1, 2, 3, 3]));
        assert!(contains_duplicates([1, 2, 2, 3]));
        assert!(contains_duplicates([1, 2, 3, 3, 4, 5]));
    }
}
