// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The error messages given by the governance contract.

/// Error message given when the state is about to be updated to an empty set of
/// owner.
pub const EMTPY_OWNER: &str = "The owner-set must not be empty";

/// Error message given when the state is about to be updated to a set of owners
/// that is larger than `u8::MAX`.
pub const TOO_MANY_OWNERS: &str = "The owner-set cannot be larger than u8::MAX";

/// Error message given when the state is about to be updated to a set of
/// operators that is larger than `u8::MAX`.
pub const TOO_MANY_OPERATORS: &str =
    "The operator-set cannot be larger than u8::MAX";

/// Error message given when the state is about to be updated to an empty set of
/// owner.
pub const ALLREADY_INITIALIZED: &str =
    "The contract has already been initialized";

/// Error message given when a given operation is not registered in the
/// operations map.
pub const OPERATION_NOT_FOUND: &str = "The given operation is not registered";

/// Error message given when there are duplicate owner-keys.
pub const DUPLICATE_OWNER: &str = "Duplicate owner-key found";

/// Error message given when there are duplicate owner-keys.
pub const DUPLICATE_OPERATOR: &str = "Duplicate operator-key found";

/// Error message given when one of the signer indices is out of bounds for the
/// owner-keys.
pub const OWNER_NOT_FOUND: &str = "The given owner index doesn't exist";

/// Error message given when one of the signer indices is out of bounds for the
/// operator-keys.
pub const OPERATOR_NOT_FOUND: &str = "The given operator index doesn't exist";

/// Error message given in case of an invalid signature.
pub const INVALID_SIGNATURE: &str = "The signature is invalid";

/// Error message given not enough signatures have been collected for the given
/// operation.
pub const THRESHOLD_NOT_MET: &str =
    "The required threshold of signatures has not been met";

/// Error message given when one of the keys used in a signature is not a valid
/// point.
pub const INVALID_PUBLIC_KEY: &str = "One of the keys used for";

/// Error message given when the nonce used for a signature is incorrect.
pub const INVALID_NONCE: &str = "The given nonce is not correct";
