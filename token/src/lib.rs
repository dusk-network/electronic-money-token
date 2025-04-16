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

mod utils;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::abi;
use dusk_forge::contract;
use emt_core::admin_management::events::PauseToggled;
use emt_core::admin_management::PAUSED_MESSAGE;
use emt_core::governance::events::GovernanceTransferredEvent;
use emt_core::governance::{GOVERNANCE_NOT_FOUND, UNAUTHORIZED_ACCOUNT};
use emt_core::sanctions::events::AccountStatusEvent;
use emt_core::sanctions::{BLOCKED, FROZEN};
use emt_core::supply_management::events::{BURN_TOPIC, MINT_TOPIC};
use emt_core::supply_management::SUPPLY_OVERFLOW;
use emt_core::{
    Account, AccountInfo, ApproveEvent, TransferEvent, TransferInfo,
    ACCOUNT_NOT_FOUND, BALANCE_TOO_LOW, ZERO_ADDRESS,
};

use crate::utils::sender_account;

#[contract]
pub mod contract {
    use super::*;

    /// The state of the token-contract.
    pub struct TokenState {
        accounts: BTreeMap<Account, AccountInfo>,
        allowances: BTreeMap<Account, BTreeMap<Account, u64>>,
        supply: u64,

        governance: Account,

        is_paused: bool,
    }

    impl TokenState {
        const fn new() -> Self {
            TokenState {
                accounts: BTreeMap::new(),
                allowances: BTreeMap::new(),
                supply: 0,
                governance: ZERO_ADDRESS,
                is_paused: false,
            }
        }

        pub fn init(
            &mut self,
            accounts: Vec<(Account, u64)>,
            governance: Account,
        ) {
            for (account, balance) in accounts {
                let account_entry =
                    self.accounts.entry(account).or_insert(AccountInfo::EMPTY);
                account_entry.balance += balance;
                self.supply += balance;

                abi::emit(
                    MINT_TOPIC,
                    TransferEvent {
                        sender: ZERO_ADDRESS,
                        spender: None,
                        receiver: account,
                        value: balance,
                    },
                );
            }

            // Set the governance
            self.governance = governance;

            // Always insert governance
            self.accounts
                .entry(self.governance)
                .or_insert(AccountInfo::EMPTY);

            abi::emit(
                GovernanceTransferredEvent::GOVERNANCE_TRANSFERRED,
                GovernanceTransferredEvent {
                    previous_governance: ZERO_ADDRESS,
                    new_governance: governance,
                },
            );
        }
    }

    /// Access control implementation.
    impl TokenState {
        fn governance_info_mut(&mut self) -> &mut AccountInfo {
            self.accounts
                .get_mut(&self.governance)
                .expect(GOVERNANCE_NOT_FOUND)
        }

        fn authorize_governance(&self) {
            assert!(
                sender_account() == self.governance,
                "{}",
                UNAUTHORIZED_ACCOUNT
            );
        }

        pub fn governance(&self) -> Account {
            self.governance
        }

        pub fn transfer_governance(&mut self, new_governance: Account) {
            self.authorize_governance();

            let previous_governance = self.governance;

            self.governance = new_governance;
            // Always insert governance
            self.accounts
                .entry(new_governance)
                .or_insert(AccountInfo::EMPTY);

            abi::emit(
                GovernanceTransferredEvent::GOVERNANCE_TRANSFERRED,
                GovernanceTransferredEvent {
                    previous_governance,
                    new_governance,
                },
            );
        }

        pub fn renounce_governance(&mut self) {
            self.authorize_governance();

            let previous_governance = self.governance;
            self.governance = ZERO_ADDRESS;

            abi::emit(
                GovernanceTransferredEvent::GOVERNANCE_RENOUNCED,
                GovernanceTransferredEvent {
                    previous_governance,
                    new_governance: ZERO_ADDRESS,
                },
            );
        }

        pub fn blocked(&self, account: Account) -> bool {
            let governance_account = self.accounts.get(&account);

            match governance_account {
                Some(account) => account.is_blocked(),
                None => false,
            }
        }

        pub fn frozen(&self, account: Account) -> bool {
            let governance_account = self.accounts.get(&account);

            match governance_account {
                Some(account) => account.is_frozen(),
                None => false,
            }
        }

        pub fn block(&mut self, account: Account) {
            self.authorize_governance();

            let account_info =
                self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

            account_info.block();

            abi::emit(
                AccountStatusEvent::BLOCKED_TOPIC,
                AccountStatusEvent::blocked(account),
            );
        }

        pub fn freeze(&mut self, account: Account) {
            self.authorize_governance();

            let account_info =
                self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

            account_info.freeze();

            abi::emit(
                AccountStatusEvent::FROZEN_TOPIC,
                AccountStatusEvent::frozen(account),
            );
        }

        pub fn unblock(&mut self, account: Account) {
            self.authorize_governance();

            let account_info =
                self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

            assert!(account_info.is_blocked(), "The account is not blocked");

            account_info.unblock();

            abi::emit(
                AccountStatusEvent::UNBLOCKED_TOPIC,
                AccountStatusEvent::unblocked(account),
            );
        }

        pub fn unfreeze(&mut self, account: Account) {
            self.authorize_governance();

            let account_info =
                self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

            assert!(account_info.is_frozen(), "The account is not frozen");

            account_info.unfreeze();

            abi::emit(
                AccountStatusEvent::UNFROZEN_TOPIC,
                AccountStatusEvent::unfrozen(account),
            );
        }
    }

    /// Supply management implementation.
    impl TokenState {
        pub fn mint(&mut self, receiver: Account, amount: u64) {
            self.authorize_governance();

            let receiver_account =
                self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

            // Prevent overflow
            self.supply = if let Some(supply) = self.supply.checked_add(amount)
            {
                supply
            } else {
                panic!("{}", SUPPLY_OVERFLOW)
            };

            receiver_account.balance += amount;

            abi::emit(
                MINT_TOPIC,
                TransferEvent {
                    sender: ZERO_ADDRESS,
                    spender: None,
                    receiver,
                    value: amount,
                },
            );
        }

        pub fn burn(&mut self, amount: u64) {
            self.authorize_governance();

            let burn_account = self.governance_info_mut();

            if burn_account.balance < amount {
                panic!("{}", BALANCE_TOO_LOW);
            } else {
                burn_account.balance -= amount;
            }

            // this can never fail, as the balance is checked above
            self.supply -= amount;

            abi::emit(
                BURN_TOPIC,
                TransferEvent {
                    sender: self.governance,
                    spender: None,
                    receiver: ZERO_ADDRESS,
                    value: amount,
                },
            );
        }
    }

    /// Administrative functions.
    impl TokenState {
        pub fn is_paused(&self) -> bool {
            self.is_paused
        }

        pub fn toggle_pause(&mut self) {
            self.authorize_governance();

            self.is_paused = !self.is_paused;

            abi::emit(
                PauseToggled::TOPIC,
                PauseToggled {
                    paused: self.is_paused,
                },
            );
        }

        /// note: this function will fail if the balance of the obliged sender
        /// is too low. It will **not** default to the maximum available
        /// balance.
        pub fn force_transfer(
            &mut self,
            obliged_sender: Account,
            receiver: Account,
            value: u64,
        ) {
            self.authorize_governance();

            let obliged_sender_account = self
                .accounts
                .get_mut(&obliged_sender)
                .expect(ACCOUNT_NOT_FOUND);

            assert!(
                obliged_sender_account.balance >= value,
                "{}",
                BALANCE_TOO_LOW
            );

            obliged_sender_account.balance -= value;

            let receiver_account =
                self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

            // this can never overflow as value + balance is never higher than
            // total supply
            receiver_account.balance += value;

            abi::emit(
                TransferEvent::FORCE_TRANSFER_TOPIC,
                TransferEvent {
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
        pub fn name(&self) -> String {
            String::from("Electronic Money Token")
        }

        pub fn symbol(&self) -> String {
            String::from("EMT")
        }

        pub fn decimals(&self) -> u8 {
            18
        }

        pub fn total_supply(&self) -> u64 {
            self.supply
        }

        pub fn account(&self, account: Account) -> AccountInfo {
            self.accounts
                .get(&account)
                .copied()
                .unwrap_or(AccountInfo::EMPTY)
        }

        #[allow(clippy::large_types_passed_by_value)]
        pub fn allowance(&self, owner: Account, spender: Account) -> u64 {
            match self.allowances.get(&owner) {
                Some(allowances) => {
                    allowances.get(&spender).copied().unwrap_or(0)
                }
                None => 0,
            }
        }

        /// Initates a `Transfer` from the sender to the receiver with the
        /// specified value.
        ///
        /// Both the sender and the receiver are accounts.
        ///
        /// # Note
        /// the sender must not be blocked or frozen.
        /// the receiver must not be blocked but can be frozen.
        #[allow(clippy::large_types_passed_by_value)]
        pub fn transfer(&mut self, receiver: Account, value: u64) {
            assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

            let sender = sender_account();

            let sender_account =
                self.accounts.get_mut(&sender).expect(ACCOUNT_NOT_FOUND);
            assert!(!sender_account.is_blocked(), "{}", BLOCKED);
            assert!(!sender_account.is_frozen(), "{}", FROZEN);

            assert!(sender_account.balance >= value, "{}", BALANCE_TOO_LOW);

            sender_account.balance -= value;

            let receiver_account =
                self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

            assert!(!receiver_account.is_blocked(), "{}", BLOCKED);

            // this can never overflow as value + balance is never higher than
            // total supply
            receiver_account.balance += value;

            abi::emit(
                TransferEvent::TRANSFER_TOPIC,
                TransferEvent {
                    sender,
                    spender: None,
                    receiver,
                    value,
                },
            );

            // if the transfer is to a contract, the acceptance function of said
            // contract is called. if it fails (panic or OoG) the transfer
            // also fails.
            if let Account::Contract(contract) = receiver {
                if let Err(err) = abi::call::<_, ()>(
                    contract,
                    "token_received",
                    &TransferInfo { sender, value },
                ) {
                    panic!("Failed calling `token_received` on the receiving contract: {err}");
                }
            }
        }

        /// Note:
        /// the spender must not be blocked or frozen.
        /// the actual owner of the funds must not be blocked or frozen.
        /// the receiver must not be blocked but can be frozen.
        #[allow(clippy::large_types_passed_by_value)]
        pub fn transfer_from(
            &mut self,
            owner: Account,
            receiver: Account,
            value: u64,
        ) {
            assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

            let spender = sender_account();

            let spender_account =
                self.accounts.entry(spender).or_insert(AccountInfo::EMPTY);
            assert!(!spender_account.is_blocked(), "{}", BLOCKED);
            assert!(!spender_account.is_frozen(), "{}", FROZEN);

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

            let owner_account =
                self.accounts.get_mut(&owner).expect(ACCOUNT_NOT_FOUND);
            assert!(!owner_account.is_blocked(), "{}", BLOCKED);
            assert!(!owner_account.is_frozen(), "{}", FROZEN);

            assert!(owner_account.balance >= value, "{}", BALANCE_TOO_LOW);

            *allowance -= value;
            owner_account.balance -= value;

            let receiver_account =
                self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);
            assert!(!receiver_account.is_blocked(), "{}", BLOCKED);

            // this can never overflow as value + balance is never higher than
            // total supply
            receiver_account.balance += value;

            abi::emit(
                TransferEvent::TRANSFER_TOPIC,
                TransferEvent {
                    sender: owner,
                    spender: Some(spender),
                    receiver,
                    value,
                },
            );

            // if the transfer is to a contract, the acceptance function of said
            // contract is called. if it fails (panic or OoG) the transfer
            // also fails.
            if let Account::Contract(contract) = receiver {
                if let Err(err) = abi::call::<_, ()>(
                    contract,
                    "token_received",
                    &TransferInfo {
                        sender: owner,
                        value,
                    },
                ) {
                    panic!("Failed calling `token_received` on the receiving contract: {err}");
                }
            }
        }

        pub fn approve(&mut self, spender: Account, value: u64) {
            // owner of the funds
            let owner = sender_account();

            let allowances = self.allowances.entry(owner).or_default();

            allowances.insert(spender, value);

            abi::emit(
                ApproveEvent::APPROVE_TOPIC,
                ApproveEvent {
                    sender: owner,
                    spender,
                    value,
                },
            );
        }
    }
}
