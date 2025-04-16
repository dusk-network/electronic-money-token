// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used to interact with the `emt-contract`.

#![no_std]
#![warn(missing_debug_implementations, unreachable_pub, rustdoc::all)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#![deny(unused_extern_crates)]
#![deny(unused_must_use)]
#![deny(
    rust_2018_idioms,
    rust_2018_compatibility,
    rust_2021_compatibility,
    rust_2024_compatibility
)]
#![deny(clippy::pedantic)]

extern crate alloc;

/// Types to interact with the token-contract.
pub mod token;
pub use token::account::{Account, AccountInfo};
pub use token::{ApproveEvent, TransferEvent, ZERO_ADDRESS};

/// Additional types used to interact with the governance-contract.
pub mod governance;
