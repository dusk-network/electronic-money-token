// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use dusk_core::abi;
use emt_core::allowlist::{error, events, Address, Role};
use emt_core::token::sender_account;
use emt_core::{Account, ZERO_ADDRESS};

/// The state of the allowlist-contract.
pub struct AllowList {
    /// The map of allowed addresses and their roles stored in the allowlist
    allowed: BTreeMap<Address, Role>,
    /// The ownership of the allowlist-contract
    ownership: Account,
}

/// The state of the allowlist-contract at deployment.
pub static mut STATE: AllowList = AllowList {
    allowed: BTreeMap::new(),
    ownership: ZERO_ADDRESS,
};

impl AllowList {
    /// Initialize the allowlist-contract state the contracts ownership
    /// `Account` and with a set of allowed user's `Address`es and `Role`s.
    ///
    /// # Panics
    /// This function will panic if:
    /// - There are duplicate user addresses in the given set.
    /// - This function is called after the contract had been initialized
    ///   already.
    pub fn init(&mut self, allowed: Vec<(Address, Role)>, ownership: Account) {
        // panic if the contract has already been initialized
        assert!(self.allowed.is_empty(), "{}", error::ALLREADY_INITIALIZED);
        assert!(
            self.ownership == ZERO_ADDRESS,
            "{}",
            error::ALLREADY_INITIALIZED
        );

        // insert the allowed addresses
        for (user, role) in allowed {
            // panic if the user is already registered
            assert!(
                !self.allowed.contains_key(&user),
                "{}",
                error::DUPLICATE_USER
            );

            // add user and role
            self.allowed.insert(user, role);

            // notify network of the registered addresses
            abi::emit(
                events::UpdateAllowList::REGISTER,
                events::UpdateAllowList { user, role },
            );
        }

        // set the ownership
        self.ownership = ownership;

        // notify network of added ownership
        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_TRANSFERRED,
            events::OwnershipTransferred {
                previous_ownership: ZERO_ADDRESS,
                new_ownership: ownership,
            },
        );
    }
}

/// Basic functionality of the allowlist-contract
impl AllowList {
    /// Checks whether a given user has been registered in the allowlist.
    ///
    /// Returns `true` if the user is registered and `false` if not.
    pub fn is_allowed(&self, user: &Address) -> bool {
        self.allowed.contains_key(user)
    }

    /// Returns the `Role` of a given user's `Address`.
    ///
    /// Returns `None` if the user is not listed in the allowlist.
    pub fn has_role(&self, user: &Address) -> Option<Role> {
        self.allowed.get(user).copied()
    }
}

/// Functions only allowed to be executed by the registered ownership contract.
impl AllowList {
    /// Register a new user in the allowlist.
    ///
    /// # Panics
    /// This method will panic if the request was not sent from the `Account`
    /// registered as ownership, or if there is already an entry for the
    /// given user.
    pub fn register(&mut self, user: Address, role: Role) {
        // make sure the sender is the account registered as ownership
        self.authorize_ownership();

        // panic if the user already exists
        assert!(
            !self.allowed.contains_key(&user),
            "{}",
            error::DUPLICATE_USER
        );

        // add user and role
        self.allowed.insert(user, role);

        // notify network of the newly registered user
        abi::emit(
            events::UpdateAllowList::REGISTER,
            events::UpdateAllowList { user, role },
        );
    }

    /// Update the role of an already registered user in the allowlist.
    ///
    /// # Panics
    /// This method will panic if the request was not sent from the `Account`
    /// registered as ownership, or if there is no entry for the given user.
    pub fn update(&mut self, user: Address, role: Role) {
        // make sure the sender is the account registered as ownership
        self.authorize_ownership();

        // update the user's role if it already exists
        if let Some(value) = self.allowed.get_mut(&user) {
            *value = role;
        } else {
            panic!("{}", error::ADDRESS_NOT_FOUND);
        }

        // notify network of the updated user's role
        abi::emit(
            events::UpdateAllowList::UPDATE,
            events::UpdateAllowList { user, role },
        );
    }

    /// Remove a user from the allowlist.
    ///
    /// # Panics
    /// This method will panic if the request was not sent from the `Account`
    /// registered as ownership, or if there is no entry for the given user.
    pub fn remove(&mut self, user: Address) {
        // make sure the sender is the account registered as ownership
        self.authorize_ownership();

        // remove the user
        if let Some(entry) = self.allowed.remove_entry(&user) {
            // notify network of the newly registered user
            abi::emit(
                events::UpdateAllowList::REMOVE,
                events::UpdateAllowList {
                    user: entry.0,
                    role: entry.1,
                },
            );
        } else {
            // panic if the user wasn't stored in the allowlist
            panic!("{}", error::ADDRESS_NOT_FOUND);
        }
    }
}

/// Access control implementation.
impl AllowList {
    /// Return the ownership of the allowlist.
    ///
    /// Only the `Account` listed as ownership is allowed to change the
    /// ownership of the allowlist, register or remove addresses.
    ///
    /// If the `ZERO_ADDRESS` is listed as ownership, the allowlist has no
    /// ownership and no changes to its state can be made.
    pub fn ownership(&self) -> Account {
        self.ownership
    }

    /// Assert that the call was made from the account listed as `ownership`.
    ///
    /// # Panics
    /// This method will panic if the call hasn't been made from the account
    /// listed under `ownership` or if the call didn't use the transfer-contract
    /// as entry point.
    fn authorize_ownership(&self) {
        assert!(
            sender_account() == self.ownership,
            "{}",
            error::UNAUTHORIZED_ACCOUNT
        );
    }

    /// Transfer the ownership of the allowlist to a new account.
    ///
    /// # Panics
    /// This method will panic if the call hasn't been made from the account
    /// listed under `ownership` or if the call didn't use the transfer-contract
    /// as entry point.
    pub fn transfer_ownership(&mut self, new_ownership: Account) {
        self.authorize_ownership();

        let previous_ownership = self.ownership;

        self.ownership = new_ownership;

        // notify network of the change in ownership
        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_TRANSFERRED,
            events::OwnershipTransferred {
                previous_ownership,
                new_ownership,
            },
        );
    }

    /// Renounce the ownership of the allowlist to a new account.
    /// After calling this method the allowlist doesn't have a registered
    /// ownership anymore and no further changes to its state are possible.
    ///
    /// # Panics
    /// This method will panic if the call hasn't been made from the account
    /// listed under `ownership` or if the call didn't use the transfer-contract
    /// as entry point.
    pub fn renounce_ownership(&mut self) {
        self.authorize_ownership();

        let previous_ownership = self.ownership;
        self.ownership = ZERO_ADDRESS;

        // notify network of the removal of ownership
        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_RENOUNCED,
            events::OwnershipTransferred {
                previous_ownership,
                new_ownership: ZERO_ADDRESS,
            },
        );
    }
}
