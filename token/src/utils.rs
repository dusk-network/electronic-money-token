// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utility & Helper functions for the token contract

use dusk_core::abi;
use emt_core::{Account, SHIELDED_NOT_SUPPORTED};

/// Determines and returns the sender of the current transfer.
///
/// If the sender is an external account, return the transaction origin.
/// If the sender is a contract, return the calling contract.
///
/// # Returns
///
/// An `Account` representing the token sender.
///
/// # Panics
///
/// - If no public sender is available (shielded transactions are not supported)
/// - If no caller can be determined (impossible case)
pub(crate) fn sender_account() -> Account {
    let tx_origin = abi::public_sender().expect(SHIELDED_NOT_SUPPORTED);

    let caller = abi::caller().expect("ICC expects a caller");

    // Identifies the sender by checking the call stack and transaction origin:
    // - For direct external account transactions (call stack length = 1),
    //   returns the transaction origin
    // - For non-protocol contracts that call the token (call stack length > 1),
    //   returns the immediate calling contract
    if abi::callstack().len() == 1 {
        // This also implies, that the call directly originates via the protocol
        // transfer contract i.e., the caller is the transfer
        // contract
        Account::External(tx_origin)
    } else {
        Account::Contract(caller)
    }
}
