// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used to inteact with the `ttoken-contract`.

#![no_std]
#![deny(missing_docs)]

/// Types used for administrative functions.
pub mod admin_management;
/// Types used for access control through governance.
pub mod governance;
/// Types used for sanctions.
pub mod sanctions;
/// Types used for supply management.
pub mod supply_management;
/// Implementation of the base token.
pub(crate) mod token;
pub use token::account::{
    Account, AccountInfo, ACCOUNT_NOT_FOUND, BALANCE_TOO_LOW, INVALID_CALLER,
    SHIELDED_NOT_SUPPORTED,
};
pub use token::{
    Allowance, Approve, ApproveEvent, Transfer, TransferEvent, TransferFrom,
    TransferInfo, ZERO_ADDRESS,
};
