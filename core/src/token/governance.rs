// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use crate::Account;

/// Event emitted when the governance of a contract is transferred.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct GovernanceTransferredEvent {
    /// The previous governance of the contract.
    pub previous_governance: Account,
    /// The new governance of the contract.
    pub new_governance: Account,
}

impl GovernanceTransferredEvent {
    /// Event Topic for transferring the governance.
    pub const GOVERNANCE_TRANSFERRED: &'static str = "governance_transferred";
    /// Event Topic for renouncing the governance.
    pub const GOVERNANCE_RENOUNCED: &'static str = "governance_renounced";
}

/// Event emitted when the governance of a contract is accepted in a two
/// step transfer process.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct GovernanceAcceptedEvent {
    /// The previous governance of the contract.
    pub previous_governance: Account,
    /// The new governance of the contract.
    pub new_governance: Account,
}

impl GovernanceAcceptedEvent {
    /// Event Topic
    pub const TOPIC: &'static str = "governance_accepted";
}
