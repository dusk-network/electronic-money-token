// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(target_family = "wasm")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]
#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::abi;
use dusk_core::transfer::data::ContractCall;
use emt_core::token::{error, events, sender_account};
use emt_core::{Account, AccountInfo, ZERO_ADDRESS};

/// The state of the token-contract.
struct TokenState {
    accounts: BTreeMap<Account, AccountInfo>,
    allowances: BTreeMap<Account, BTreeMap<Account, u64>>,
    supply: u64,

    ownership: Account,

    is_paused: bool,
}

impl TokenState {
    fn init(&mut self, accounts: Vec<(Account, u64)>, ownership: Account) {
        for (account, balance) in accounts {
            let account_entry =
                self.accounts.entry(account).or_insert(AccountInfo::EMPTY);
            account_entry.balance += balance;
            self.supply += balance;

            abi::emit(
                events::Transfer::MINT_TOPIC,
                events::Transfer {
                    sender: ZERO_ADDRESS,
                    spender: None,
                    receiver: account,
                    value: balance,
                },
            );
        }

        // Set the ownership
        self.ownership = ownership;

        // Always insert ownership
        self.accounts
            .entry(self.ownership)
            .or_insert(AccountInfo::EMPTY);

        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_TRANSFERRED,
            events::OwnershipTransferred {
                previous_ownership: ZERO_ADDRESS,
                new_ownership: ownership,
            },
        );
    }
}

static mut STATE: TokenState = TokenState {
    accounts: BTreeMap::new(),
    allowances: BTreeMap::new(),
    supply: 0,
    ownership: ZERO_ADDRESS,
    is_paused: false,
};

/// Access control implementation.
impl TokenState {
    fn ownership(&self) -> Account {
        self.ownership
    }

    fn ownership_info_mut(&mut self) -> &mut AccountInfo {
        self.accounts
            .get_mut(&self.ownership)
            .expect(error::OWNERSHIP_NOT_FOUND)
    }

    fn authorize_ownership(&self) {
        assert!(
            sender_account() == self.ownership,
            "{}",
            error::UNAUTHORIZED_ACCOUNT
        );
    }

    fn transfer_ownership(&mut self, new_ownership: Account) {
        self.authorize_ownership();

        let previous_ownership = self.ownership;

        self.ownership = new_ownership;
        // Always insert ownership
        self.accounts
            .entry(new_ownership)
            .or_insert(AccountInfo::EMPTY);

        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_TRANSFERRED,
            events::OwnershipTransferred {
                previous_ownership,
                new_ownership,
            },
        );
    }

    fn renounce_ownership(&mut self) {
        self.authorize_ownership();

        let previous_ownership = self.ownership;
        self.ownership = ZERO_ADDRESS;

        abi::emit(
            events::OwnershipTransferred::OWNERSHIP_RENOUNCED,
            events::OwnershipTransferred {
                previous_ownership,
                new_ownership: ZERO_ADDRESS,
            },
        );
    }

    fn blocked(&self, account: Account) -> bool {
        let ownership_account = self.accounts.get(&account);

        match ownership_account {
            Some(account) => account.is_blocked(),
            None => false,
        }
    }

    fn frozen(&self, account: Account) -> bool {
        let ownership_account = self.accounts.get(&account);

        match ownership_account {
            Some(account) => account.is_frozen(),
            None => false,
        }
    }

    fn block(&mut self, account: Account) {
        self.authorize_ownership();

        let account_info = self
            .accounts
            .get_mut(&account)
            .expect(error::OWNERSHIP_NOT_FOUND);

        account_info.block();

        abi::emit(
            events::AccountStatus::BLOCKED_TOPIC,
            events::AccountStatus::blocked(account),
        );
    }

    fn freeze(&mut self, account: Account) {
        self.authorize_ownership();

        let account_info = self
            .accounts
            .get_mut(&account)
            .expect(error::OWNERSHIP_NOT_FOUND);

        account_info.freeze();

        abi::emit(
            events::AccountStatus::FROZEN_TOPIC,
            events::AccountStatus::frozen(account),
        );
    }

    fn unblock(&mut self, account: Account) {
        self.authorize_ownership();

        let account_info = self
            .accounts
            .get_mut(&account)
            .expect(error::OWNERSHIP_NOT_FOUND);

        assert!(account_info.is_blocked(), "The account is not blocked");

        account_info.unblock();

        abi::emit(
            events::AccountStatus::UNBLOCKED_TOPIC,
            events::AccountStatus::unblocked(account),
        );
    }

    fn unfreeze(&mut self, account: Account) {
        self.authorize_ownership();

        let account_info = self
            .accounts
            .get_mut(&account)
            .expect(error::OWNERSHIP_NOT_FOUND);

        assert!(account_info.is_frozen(), "The account is not frozen");

        account_info.unfreeze();

        abi::emit(
            events::AccountStatus::UNFROZEN_TOPIC,
            events::AccountStatus::unfrozen(account),
        );
    }
}

/// Supply management implementation.
impl TokenState {
    fn mint(&mut self, receiver: Account, amount: u64) {
        self.authorize_ownership();

        let receiver_account =
            self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

        // Prevent overflow
        self.supply = if let Some(supply) = self.supply.checked_add(amount) {
            supply
        } else {
            panic!("{}", error::SUPPLY_OVERFLOW)
        };

        receiver_account.balance += amount;

        abi::emit(
            events::Transfer::MINT_TOPIC,
            events::Transfer {
                sender: ZERO_ADDRESS,
                spender: None,
                receiver,
                value: amount,
            },
        );
    }

    fn burn(&mut self, amount: u64) {
        self.authorize_ownership();

        let burn_account = self.ownership_info_mut();

        if burn_account.balance < amount {
            panic!("{}", error::BALANCE_TOO_LOW);
        } else {
            burn_account.balance -= amount;
        }

        // this can never fail, as the balance is checked above
        self.supply -= amount;

        abi::emit(
            events::Transfer::BURN_TOPIC,
            events::Transfer {
                sender: self.ownership,
                spender: None,
                receiver: ZERO_ADDRESS,
                value: amount,
            },
        );
    }
}

/// Administrative functions.
impl TokenState {
    fn is_paused(&self) -> bool {
        self.is_paused
    }

    fn toggle_pause(&mut self) {
        self.authorize_ownership();

        self.is_paused = !self.is_paused;

        abi::emit(
            events::PauseToggled::TOPIC,
            events::PauseToggled {
                paused: self.is_paused,
            },
        );
    }

    /// note: this function will fail if the balance of the obliged sender is
    /// too low. It will **not** default to the maximum available balance.
    fn force_transfer(
        &mut self,
        obliged_sender: Account,
        receiver: Account,
        value: u64,
    ) {
        self.authorize_ownership();

        let obliged_sender_account = self
            .accounts
            .get_mut(&obliged_sender)
            .expect(error::ACCOUNT_NOT_FOUND);

        assert!(
            obliged_sender_account.balance >= value,
            "{}",
            error::BALANCE_TOO_LOW
        );

        obliged_sender_account.balance -= value;

        let receiver_account =
            self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

        // this can never overflow as value + balance is never higher than total
        // supply
        receiver_account.balance += value;

        abi::emit(
            events::Transfer::FORCE_TRANSFER_TOPIC,
            events::Transfer {
                sender: obliged_sender,
                spender: None,
                receiver,
                value,
            },
        );
    }
}

/// Basic token-contract implementation.
impl TokenState {
    fn name() -> String {
        String::from("Electronic Money Token")
    }

    fn symbol() -> String {
        String::from("EMT")
    }

    fn decimals() -> u8 {
        18
    }

    fn total_supply(&self) -> u64 {
        self.supply
    }

    fn account(&self, account: Account) -> AccountInfo {
        self.accounts
            .get(&account)
            .copied()
            .unwrap_or(AccountInfo::EMPTY)
    }

    fn balance_of(&self, account: Account) -> u64 {
        match self.accounts.get(&account) {
            Some(account_info) => account_info.balance,
            None => 0,
        }
    }

    #[allow(clippy::large_types_passed_by_value)]
    fn allowance(&self, owner: Account, spender: Account) -> u64 {
        match self.allowances.get(&owner) {
            Some(allowances) => allowances.get(&spender).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// Initates a `Transfer` from the sender to the receiver with the specified
    /// value.
    ///
    /// Both the sender and the receiver are accounts.
    ///
    /// # Note
    /// the sender must not be blocked or frozen.
    /// the receiver must not be blocked but can be frozen.
    #[allow(clippy::large_types_passed_by_value)]
    fn transfer(&mut self, receiver: Account, value: u64) {
        assert!(!self.is_paused, "{}", error::PAUSED_MESSAGE);

        let sender = sender_account();

        let sender_account = self
            .accounts
            .get_mut(&sender)
            .expect(error::ACCOUNT_NOT_FOUND);
        assert!(!sender_account.is_blocked(), "{}", error::BLOCKED);
        assert!(!sender_account.is_frozen(), "{}", error::FROZEN);

        assert!(
            sender_account.balance >= value,
            "{}",
            error::BALANCE_TOO_LOW
        );

        sender_account.balance -= value;

        let receiver_account =
            self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

        assert!(!receiver_account.is_blocked(), "{}", error::BLOCKED);

        // this can never overflow as value + balance is never higher than total
        // supply
        receiver_account.balance += value;

        abi::emit(
            events::Transfer::TRANSFER_TOPIC,
            events::Transfer {
                sender,
                spender: None,
                receiver,
                value,
            },
        );
    }

    /// Transfers tokens to a contract receiver and call a specified function on
    /// that contract.
    ///
    /// # Behavior
    ///
    /// This function transfers the given `value` of tokens to the contract
    /// indicated by `contract_call.contract` and then calls the function
    /// specified by `contract_call.fn_name` with the provided
    /// `contract_call.fn_args`.
    ///
    /// If the contract function expects parameters, it is possible to pass
    /// incorrect arguments intentionally. The receiving contract is
    /// responsible for validating such arguments, as this token is unaware
    /// of arbitrary contract logic.
    ///
    ///
    /// # Notes
    ///
    /// - This function cannot be used if you need to transfer tokens to an
    ///   arbitrary account while calling a function on a contract. For
    ///   scenarios requiring multiple operations at once, consider implementing
    ///   a multicall solution.
    /// - `transfer_and_call` is atomic: if the function call on the receiving
    ///   contract fails (due to a panic or out of gas error), the token
    ///   transfer also fails and reverts.
    fn transfer_and_call(&mut self, value: u64, contract_call: &ContractCall) {
        let receiver = Account::from(contract_call.contract);
        self.transfer(receiver, value);

        // If the call to the contract fails (panic or OoG) the transfer
        // also fails.
        if let Err(err) = abi::call_raw(
            contract_call.contract,
            &contract_call.fn_name,
            &contract_call.fn_args,
        ) {
            panic!(
                "Failed calling `{}` on the contract: {err}",
                contract_call.fn_name
            );
        }
    }

    /// Note:
    /// the spender must not be blocked or frozen.
    /// the actual owner of the funds must not be blocked or frozen.
    /// the receiver must not be blocked but can be frozen.
    #[allow(clippy::large_types_passed_by_value)]
    fn transfer_from(&mut self, owner: Account, receiver: Account, value: u64) {
        assert!(!self.is_paused, "{}", error::PAUSED_MESSAGE);

        let spender = sender_account();

        let spender_account =
            self.accounts.entry(spender).or_insert(AccountInfo::EMPTY);
        assert!(!spender_account.is_blocked(), "{}", error::BLOCKED);
        assert!(!spender_account.is_frozen(), "{}", error::FROZEN);

        let allowance = self
            .allowances
            .get_mut(&owner)
            .expect("The account has no allowances")
            .get_mut(&spender)
            .expect("The spender is not allowed to use the account");

        assert!(
            value <= *allowance,
            "The spender can't spent the defined amount"
        );

        let owner_account = self
            .accounts
            .get_mut(&owner)
            .expect(error::ACCOUNT_NOT_FOUND);
        assert!(!owner_account.is_blocked(), "{}", error::BLOCKED);
        assert!(!owner_account.is_frozen(), "{}", error::FROZEN);

        assert!(owner_account.balance >= value, "{}", error::BALANCE_TOO_LOW);

        *allowance -= value;
        owner_account.balance -= value;

        let receiver_account =
            self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);
        assert!(!receiver_account.is_blocked(), "{}", error::BLOCKED);

        // this can never overflow as value + balance is never higher than total
        // supply
        receiver_account.balance += value;

        abi::emit(
            events::Transfer::TRANSFER_TOPIC,
            events::Transfer {
                sender: owner,
                spender: Some(spender),
                receiver,
                value,
            },
        );
    }

    fn approve(&mut self, spender: Account, value: u64) {
        // owner of the funds
        let owner = sender_account();

        let allowances = self.allowances.entry(owner).or_default();

        allowances.insert(spender, value);

        abi::emit(
            events::Approve::APPROVE_TOPIC,
            events::Approve {
                sender: owner,
                spender,
                value,
            },
        );
    }
}

#[no_mangle]
unsafe extern "C" fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(initial_accounts, ownership)| {
        STATE.init(initial_accounts, ownership);
    })
}

#[no_mangle]
unsafe extern "C" fn name(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| TokenState::name())
}

#[no_mangle]
unsafe extern "C" fn symbol(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| TokenState::symbol())
}

#[no_mangle]
unsafe extern "C" fn decimals(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| TokenState::decimals())
}

#[no_mangle]
unsafe extern "C" fn total_supply(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.total_supply())
}

#[no_mangle]
unsafe extern "C" fn account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.account(arg))
}

#[no_mangle]
unsafe extern "C" fn balance_of(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |account| STATE.balance_of(account))
}

#[no_mangle]
unsafe extern "C" fn allowance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(owner, spender)| STATE.allowance(owner, spender))
}

#[no_mangle]
unsafe extern "C" fn transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(receiver, value)| STATE.transfer(receiver, value))
}

#[no_mangle]
unsafe extern "C" fn transfer_and_call(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(transfer, contract_call)| {
        STATE.transfer_and_call(transfer, &contract_call);
    })
}

#[no_mangle]
unsafe extern "C" fn transfer_from(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(owner, receiver, value)| {
        STATE.transfer_from(owner, receiver, value);
    })
}

#[no_mangle]
unsafe extern "C" fn approve(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(spender, value)| STATE.approve(spender, value))
}

/*
 * Access control functions
 */

#[no_mangle]
unsafe extern "C" fn transfer_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |new_ownership| {
        STATE.transfer_ownership(new_ownership);
    })
}

#[no_mangle]
unsafe extern "C" fn renounce_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.renounce_ownership())
}

#[no_mangle]
unsafe extern "C" fn ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.ownership())
}

/*
 * Supply management functions
 */

#[no_mangle]
unsafe extern "C" fn mint(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(receiver, amount)| STATE.mint(receiver, amount))
}

#[no_mangle]
unsafe extern "C" fn burn(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.burn(arg))
}

/*
 * Administrative functions
 */

#[no_mangle]
unsafe extern "C" fn toggle_pause(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.toggle_pause())
}

#[no_mangle]
unsafe extern "C" fn is_paused(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.is_paused())
}

#[no_mangle]
unsafe extern "C" fn force_transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(obliged_sender, receiver, value)| {
        STATE.force_transfer(obliged_sender, receiver, value);
    })
}

/*
 * Sanctions functions
 */

#[no_mangle]
unsafe extern "C" fn block(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |acc| STATE.block(acc))
}

#[no_mangle]
unsafe extern "C" fn freeze(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |acc| STATE.freeze(acc))
}

#[no_mangle]
unsafe extern "C" fn unblock(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |acc| STATE.unblock(acc))
}

#[no_mangle]
unsafe extern "C" fn unfreeze(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |acc| STATE.unfreeze(acc))
}

#[no_mangle]
unsafe extern "C" fn blocked(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |acc| STATE.blocked(acc))
}

#[no_mangle]
unsafe extern "C" fn frozen(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.frozen(arg))
}
