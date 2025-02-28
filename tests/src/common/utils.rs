// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_vm::{Error as VMError, Session};

const GAS_LIMIT: u64 = 0x10_000_000;

pub fn chain_id(session: &mut Session) -> Result<u8, VMError> {
    session
        .call(TRANSFER_CONTRACT, "chain_id", &(), GAS_LIMIT)
        .map(|r| r.data)
}

pub fn account(
    session: &mut Session,
    pk: &AccountPublicKey,
) -> Result<AccountData, VMError> {
    session
        .call(TRANSFER_CONTRACT, "account", pk, GAS_LIMIT)
        .map(|r| r.data)
}
