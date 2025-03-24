// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::signatures::bls::MultisigSignature;

pub mod instantiate;
use instantiate::TestKeys;

#[allow(dead_code)]
pub fn owner_signature<const O: usize, const P: usize, const H: usize>(
    keys: &TestKeys<O, P, H>,
    sig_msg: &[u8],
    signer_idx: &[u8],
) -> MultisigSignature {
    signature(keys, sig_msg, signer_idx, true)
}

#[allow(dead_code)]
pub fn operator_signature<const O: usize, const P: usize, const H: usize>(
    keys: &TestKeys<O, P, H>,
    sig_msg: &[u8],
    signer_idx: &[u8],
) -> MultisigSignature {
    signature(keys, sig_msg, signer_idx, false)
}

#[allow(dead_code)]
fn signature<const O: usize, const P: usize, const H: usize>(
    keys: &TestKeys<O, P, H>,
    sig_msg: &[u8],
    signer_idx: &[u8],
    is_owner: bool,
) -> MultisigSignature {
    let (sks, pks) = if is_owner {
        (&keys.owners_sk[..], &keys.owners_pk[..])
    } else {
        (&keys.operators_sk[..], &keys.operators_pk[..])
    };
    let sigs: Vec<MultisigSignature> = signer_idx
        .iter()
        .map(|idx| {
            sks[*idx as usize].sign_multisig(&pks[*idx as usize], sig_msg)
        })
        .collect();

    let multisig = sigs[0];
    if sigs.len() > 1 {
        multisig.aggregate(&sigs[1..])
    } else {
        multisig
    }
}
