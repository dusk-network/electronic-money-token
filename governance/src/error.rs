// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Error message given when the state is about to be updated to an empty set of
/// owner.
pub const EMTPY_OWNER: &str = "The owner-set must not be empty";

/// Error message given when the state is about to be updated to a set of owners
/// that is larger than u8::MAX.
pub const TOO_MANY_OWNERS: &str = "The owner-set cannot be larger than u8::MAX";

/// Error message given when the state is about to be updated to a set of
/// operators that is larger than u8::MAX.
pub const TOO_MANY_OPERATORS: &str =
    "The operator-set cannot be larger than u8::MAX";

/// Error message given when the state is about to be updated to an empty set of
/// owner.
pub const ALLREADY_INITIALIZED: &str =
    "The contract has already been initialized";

/// Error message given when a given operation is not registered in the
/// operations map.
pub const OPERATION_NOT_FOUND: &str = "The given operation is not registered";

/*
/// Error message given when the state is about to be updated to an empty set of
/// operator.
pub const EMTPY_OPERATOR: &str = "The operator-set must not be empty";
*/
