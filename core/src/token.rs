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
