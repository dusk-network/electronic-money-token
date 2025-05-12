// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Module for the account implementation.
pub(crate) mod account;
use account::Account;

/// Error messages given by token-contract panics.
pub mod error;
/// Events emitted by the token-contract.
pub mod events;

use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};

/// Zero address.
/// TODO: Consider having this in core & make it a reserved address so that no
/// one can ever use it.
pub const ZERO_ADDRESS: Account =
    Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]));

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
#[cfg(target_family = "wasm")]
#[must_use]
pub fn sender_account() -> Account {
    use dusk_core::abi;

    // Identifies the sender by checking the call stack and transaction origin:
    // - For direct external account transactions (call stack length = 1),
    //   returns the transaction origin
    // - For non-protocol contracts that call the token (call stack length > 1),
    //   returns the immediate calling contract
    // - This function panics for contract queries because their call stack
    //   length is 0. It is not intended for "view" calls to use this function,
    //   as they don't create traceable transactions with a sender/caller.
    //   Instead, query functions should explicitly take information to query
    //   for as an argument.
    if abi::callstack().len() == 1 {
        // This also implies, that the call directly originates via the protocol
        // transfer contract i.e., the caller is the transfer
        // contract
        Account::External(
            abi::public_sender().expect(error::SHIELDED_NOT_SUPPORTED),
        )
    } else {
        Account::Contract(abi::caller().expect("ICC expects a caller"))
    }
}
