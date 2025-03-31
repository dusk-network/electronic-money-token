// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_core::abi::StandardBufSerializer;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_vm::{Error as VMError, Session};
use rkyv::ser::serializers::{
    BufferScratch, BufferSerializer, CompositeSerializer,
};
use rkyv::ser::Serializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{check_archived_root, Archive, Deserialize, Infallible, Serialize};

const GAS_LIMIT: u64 = 0x10_000_000;

/// Desrialize function return using `rkyv`.
/// This mimics the deserialization done in piecrust.
pub fn rkyv_deserialize<R>(serialized: impl AsRef<[u8]>) -> R
where
    R: Archive,
    R::Archived:
        Deserialize<R, Infallible> + for<'b> CheckBytes<DefaultValidator<'b>>,
{
    let ta = check_archived_root::<R>(&serialized.as_ref())
        .expect("Failed to deserialize data");
    ta.deserialize(&mut Infallible)
        .expect("Failed to deserialize using rkyv")
}

/// Serialize function call arguments using `rkyv`.
/// This mimics the serialization done in piecrust.
pub fn rkyv_serialize<A>(fn_arg: &A) -> Vec<u8>
where
    A: for<'b> Serialize<StandardBufSerializer<'b>>,
    A::Archived: for<'b> CheckBytes<DefaultValidator<'b>>,
{
    // scratch-space and page-size values taken from piecrust-uplink
    const SCRATCH_SPACE: usize = 1024;
    const PAGE_SIZE: usize = 0x1000;

    let mut sbuf = [0u8; SCRATCH_SPACE];
    let scratch = BufferScratch::new(&mut sbuf);
    let mut buffer = [0u8; PAGE_SIZE];
    let ser = BufferSerializer::new(&mut buffer[..]);
    let mut ser = CompositeSerializer::new(ser, scratch, Infallible);

    ser.serialize_value(fn_arg)
        .expect("Failed to rkyv serialize fn_arg");
    let pos = ser.pos();

    buffer[..pos].to_vec()
}

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
