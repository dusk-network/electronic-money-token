// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// The error messages given by the access-control-contract.

/// Error message given when the state is about to be updated to an empty set of
/// admins.
pub const EMPTY_ADMINS: &str = "The admins-set must not be empty";

/// Error message given when the state is about to be updated to a set of admins
/// that is larger than `u8::MAX`.
pub const TOO_MANY_ADMINS: &str =
    "The admins-set cannot be larger than u8::MAX";

/// Error message given when the state is about to be updated to a set of
/// operators that is larger than `u8::MAX`.
pub const TOO_MANY_OPERATORS: &str =
    "The operator-set cannot be larger than u8::MAX";

/// Error message given when the contract has already been initialized and init
/// is called.
pub const ALLREADY_INITIALIZED: &str =
    "The contract has already been initialized";

/// Error message given when a given token-contract call is not registered.
pub const TOKEN_CALL_NOT_FOUND: &str =
    "The given token-contract call is not registered";

/// Error message given when there are duplicate admins-keys.
pub const DUPLICATE_ADMINS: &str = "Duplicate admins-key found";

/// Error message given when there are duplicate operator-keys.
pub const DUPLICATE_OPERATOR: &str = "Duplicate operator-key found";

/// Error message given when there are duplicate signer-keys.
pub const DUPLICATE_SIGNER: &str = "Duplicate signer-key found";

/// Error message given when one of the signer indices doesn't exist.
pub const SIGNER_NOT_FOUND: &str = "The given signer doesn't exist";

/// Error message given in case of an invalid signature.
pub const INVALID_SIGNATURE: &str = "The signature is invalid";

/// Error message given when the signature threshold for calling a function on
/// the token-contract is not met.
pub const THRESHOLD_NOT_MET: &str =
    "The required threshold of signatures has not been met";

/// Error given when the threshold is 0 at the signature authorization.
pub const THRESHOLD_ZERO: &str =
    "The threshold shouldn't be 0 at authorization";

/// Error message given when an operator tries to trigger an inter-contract call
/// that only the admins can authorize.
pub const UNAUTHORIZED_TOKEN_CALL: &str =
    "This inter-contract call need admins authorization";

/// Error message given when an operator token-contract call panics
pub const OPERATOR_TOKEN_CALL_PANIC: &str =
    "Calling the specified operator function on the token-contract should succeed";
