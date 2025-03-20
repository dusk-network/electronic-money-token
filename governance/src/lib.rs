// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The governance contract for electronic money tokens.

#![no_std]
#![deny(unused_extern_crates)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]

pub mod error;

#[cfg(target_family = "wasm")]
pub(crate) mod state;

#[cfg(target_family = "wasm")]
pub use wasm::*;

#[cfg(target_family = "wasm")]
pub(crate) mod wasm {
    extern crate alloc;

    use alloc::string::String;

    use dusk_core::abi;

    use crate::state::STATE;

    /*
     * Basic contract implementation.
     */

    #[no_mangle]
    unsafe extern "C" fn init(arg_len: u32) -> u32 {
        abi::wrap_call(
            arg_len,
            |(token_contract, owner, operator, icc_data)| {
                STATE.init(token_contract, owner, operator, icc_data)
            },
        )
    }

    #[no_mangle]
    unsafe extern "C" fn name(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.name())
    }

    #[no_mangle]
    unsafe extern "C" fn symbol(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(): ()| STATE.symbol())
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
    unsafe extern "C" fn icc_threshold(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |icc: String| STATE.icc_threshold(icc.as_str()))
    }

    /*
     * Functions that need the owners' approval.
     */

    // pub fn set_token_contract(
    //     &mut self,
    //     new_token_contract: ContractId,
    //     sig: MultisigSignature,
    //     signers: &[u8],
    #[no_mangle]
    unsafe extern "C" fn set_token_contract(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_token_contract, sig, signers)| {
            STATE.set_token_contract(new_token_contract, sig, signers)
        })
    }

    // pub fn set_owners(
    //     &mut self,
    //     new_owners: Vec<PublicKey>,
    //     sig: MultisigSignature,
    //     signers: &[u8],
    // ) {
    #[no_mangle]
    unsafe extern "C" fn set_owners(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_owners, sig, signers)| {
            STATE.set_owners(new_owners, sig, signers)
        })
    }

    // pub fn set_operators(
    //     &mut self,
    //     new_operators: Vec<PublicKey>,
    //     sig: MultisigSignature,
    //     signers: &[u8],
    // ) {
    #[no_mangle]
    unsafe extern "C" fn set_operators(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_operators, sig, signers)| {
            STATE.set_operators(new_operators, sig, signers)
        })
    }

    // pub fn transfer_governance(
    //     &mut self,
    //     new_governance: Account,
    //     sig: MultisigSignature,
    //     signers: &[u8],
    // ) {
    #[no_mangle]
    unsafe extern "C" fn transfer_governance(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(new_governance, sig, signers)| {
            STATE.transfer_governance(new_governance, sig, signers)
        })
    }

    // pub fn renounce_governance(
    //     &mut self,
    //     sig: MultisigSignature,
    //     signers: &[u8],
    // ) {
    #[no_mangle]
    unsafe extern "C" fn renounce_governance(arg_len: u32) -> u32 {
        abi::wrap_call(arg_len, |(sig, signers)| {
            STATE.renounce_governance(sig, signers)
        })
    }
}
/*

#[no_mangle]
unsafe fn account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.account(arg))
}

#[no_mangle]
unsafe fn allowance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.allowance(arg))
}

#[no_mangle]
unsafe fn transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer(arg))
}

#[no_mangle]
unsafe fn transfer_from(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from(arg))
}

#[no_mangle]
unsafe fn transfer_from_contract(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from_contract(arg))
}

#[no_mangle]
unsafe fn approve(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.approve(arg))
}

/*
 * Supply management functions
 */

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn burn(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.burn(arg))
}
}
*/

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
    fn test_supermajority_lower_bount() {
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
