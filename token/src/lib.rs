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

use core::cell::Cell;

use dusk_core::abi;
use emt_core::admin_management::events::PauseToggled;
use emt_core::admin_management::PAUSED_MESSAGE;
use emt_core::governance::events::GovernanceTransferredEvent;
use emt_core::governance::{GOVERNANCE_NOT_FOUND, UNAUTHORIZED_ACCOUNT};
use emt_core::sanctions::events::AccountStatusEvent;
use emt_core::sanctions::{BLOCKED, FROZEN};
use emt_core::supply_management::events::{BURN_TOPIC, MINT_TOPIC};
use emt_core::supply_management::SUPPLY_OVERFLOW;
use emt_core::{
    Account, AccountInfo, ApproveEvent, Condition, TransferEvent, TransferInfo,
    ACCOUNT_NOT_FOUND, BALANCE_TOO_LOW, SHIELDED_NOT_SUPPORTED, ZERO_ADDRESS,
};

type Accounts = BTreeMap<Account, Cell<AccountInfo>>;

/// The state of the token-contract.
struct TokenState {
    accounts: Accounts,
    allowances: BTreeMap<Account, BTreeMap<Account, u64>>,
    supply: u64,

    governance: Account,

    is_paused: bool,
}

impl Condition for TokenState {
    fn precondition(
        &self,
        sender: &Cell<AccountInfo>,
        receiver: &Cell<AccountInfo>,
        value: u64,
    ) {
        assert!(sender.get().balance >= value, "{}", BALANCE_TOO_LOW);
        assert!(!self.is_paused, "{}", PAUSED_MESSAGE);
        assert!(!sender.get().is_blocked(), "{}", BLOCKED);
        assert!(!sender.get().is_frozen(), "{}", FROZEN);
        assert!(!receiver.get().is_blocked(), "{}", BLOCKED);
    }

    #[allow(unused_variables)]
    fn postcondition(
        &self,
        sender: &Cell<AccountInfo>,
        receiver: &Cell<AccountInfo>,
        value: u64,
    ) {
        unimplemented!("postcondition not needed");
    }
}

impl TokenState {
    fn init(&mut self, accounts: Vec<(Account, u64)>, governance: Account) {
        for (account, balance) in accounts {
            let account_entry = self
                .accounts
                .entry(account)
                .or_insert(Cell::new(AccountInfo::EMPTY));
            account_entry.get_mut().balance += balance;
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
            .or_insert(Cell::new(AccountInfo::EMPTY));

        abi::emit(
            GovernanceTransferredEvent::GOVERNANCE_TRANSFERRED,
            GovernanceTransferredEvent {
                previous_governance: ZERO_ADDRESS,
                new_governance: governance,
            },
        );
    }
}

static mut STATE: TokenState = TokenState {
    accounts: BTreeMap::new(),
    allowances: BTreeMap::new(),
    supply: 0,
    governance: ZERO_ADDRESS,
    is_paused: false,
};

/// Access control implementation.
impl TokenState {
    fn governance_info_mut(&mut self) -> &mut AccountInfo {
        self.accounts
            .get_mut(&self.governance)
            .expect(GOVERNANCE_NOT_FOUND)
            .get_mut()
    }

    fn authorize_governance(&self) {
        assert!(
            sender_account() == self.governance,
            "{}",
            UNAUTHORIZED_ACCOUNT
        );
    }

    fn transfer_governance(&mut self, new_governance: Account) {
        self.authorize_governance();

        let previous_governance = self.governance;

        self.governance = new_governance;
        // Always insert governance
        self.accounts
            .entry(new_governance)
            .or_insert(Cell::new(AccountInfo::EMPTY));

        abi::emit(
            GovernanceTransferredEvent::GOVERNANCE_TRANSFERRED,
            GovernanceTransferredEvent {
                previous_governance,
                new_governance,
            },
        );
    }

    fn renounce_governance(&mut self) {
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

    fn blocked(&self, account: Account) -> bool {
        let governance_account = self.accounts.get(&account);

        match governance_account {
            Some(account) => account.get().is_blocked(),
            None => false,
        }
    }

    fn frozen(&self, account: Account) -> bool {
        let governance_account = self.accounts.get(&account);

        match governance_account {
            Some(account) => account.get().is_frozen(),
            None => false,
        }
    }

    fn block(&mut self, account: Account) {
        self.authorize_governance();

        let account_info =
            self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

        account_info.get_mut().block();

        abi::emit(
            AccountStatusEvent::BLOCKED_TOPIC,
            AccountStatusEvent::blocked(account),
        );
    }

    fn freeze(&mut self, account: Account) {
        self.authorize_governance();

        let account_info =
            self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

        account_info.get_mut().freeze();

        abi::emit(
            AccountStatusEvent::FROZEN_TOPIC,
            AccountStatusEvent::frozen(account),
        );
    }

    fn unblock(&mut self, account: Account) {
        self.authorize_governance();

        let account_info =
            self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

        assert!(
            account_info.get().is_blocked(),
            "The account is not blocked"
        );

        account_info.get_mut().unblock();

        abi::emit(
            AccountStatusEvent::UNBLOCKED_TOPIC,
            AccountStatusEvent::unblocked(account),
        );
    }

    fn unfreeze(&mut self, account: Account) {
        self.authorize_governance();

        let account_info =
            self.accounts.get_mut(&account).expect(GOVERNANCE_NOT_FOUND);

        assert!(account_info.get().is_frozen(), "The account is not frozen");

        account_info.get_mut().unfreeze();

        abi::emit(
            AccountStatusEvent::UNFROZEN_TOPIC,
            AccountStatusEvent::unfrozen(account),
        );
    }
}

/// Supply management implementation.
impl TokenState {
    fn mint(&mut self, receiver: Account, amount: u64) {
        self.authorize_governance();

        let receiver_account = self
            .accounts
            .entry(receiver)
            .or_insert(Cell::new(AccountInfo::EMPTY));

        // Prevent overflow
        self.supply = if let Some(supply) = self.supply.checked_add(amount) {
            supply
        } else {
            panic!("{}", SUPPLY_OVERFLOW)
        };

        receiver_account.get_mut().balance += amount;

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

    fn burn(&mut self, amount: u64) {
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
    fn is_paused(&self) -> bool {
        self.is_paused
    }

    fn toggle_pause(&mut self) {
        self.authorize_governance();

        self.is_paused = !self.is_paused;

        abi::emit(
            PauseToggled::TOPIC,
            PauseToggled {
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
        self.authorize_governance();

        self.accounts
            .entry(receiver)
            .or_insert(Cell::new(AccountInfo::EMPTY)); // note: find a way how to avoid this

        let obliged_sender_account =
            self.accounts.get(&obliged_sender).expect(ACCOUNT_NOT_FOUND);

        assert!(
            obliged_sender_account.get().balance >= value,
            "{}",
            BALANCE_TOO_LOW
        );

        let receiver_account =
            self.accounts.get(&receiver).expect(ACCOUNT_NOT_FOUND);

        self.transfer_tokens(obliged_sender_account, receiver_account, value);

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
    fn name() -> String {
        String::from("Transparent Fungible Token Sample")
    }

    fn symbol() -> String {
        String::from("TFTS")
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
            .map_or(AccountInfo::EMPTY, Cell::get)
    }

    #[allow(clippy::large_types_passed_by_value)]
    fn allowance(&self, owner: Account, spender: Account) -> u64 {
        match self.allowances.get(&owner) {
            Some(allowances) => allowances.get(&spender).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// Internal function intended to be used by any transfer functions instead
    /// of duplicating transfer logic.
    ///
    /// This function does not check anything.
    fn transfer_tokens(
        &self,
        sender: &Cell<AccountInfo>,
        receiver: &Cell<AccountInfo>,
        value: u64,
    ) {
        // EMT specific precondition checks for the transfer
        self.precondition(sender, receiver, value);

        sender.set(sender.get().decrease_balance(value));

        // this can never overflow as value + balance is never higher than total
        // supply
        receiver.set(receiver.get().increase_balance(value));
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
        let sender = sender_account();
        self.accounts
            .entry(receiver)
            .or_insert(Cell::new(AccountInfo::EMPTY)); // note: find a way how to avoid this

        let sender_account =
            self.accounts.get(&sender).expect(ACCOUNT_NOT_FOUND);
        let receiver_account =
            self.accounts.get(&receiver).expect(ACCOUNT_NOT_FOUND);

        self.transfer_tokens(sender_account, receiver_account, value);

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
    fn transfer_from(&mut self, owner: Account, receiver: Account, value: u64) {
        assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

        let spender = sender_account();
        self.accounts
            .entry(receiver)
            .or_insert(Cell::new(AccountInfo::EMPTY)); // note: find a way how to avoid this

        if let Some(spender_account) = self.accounts.get(&spender) {
            assert!(!spender_account.get().is_blocked(), "{}", BLOCKED);
            assert!(!spender_account.get().is_frozen(), "{}", FROZEN);
        }

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

        *allowance -= value;

        let owner_account = self.accounts.get(&owner).expect(ACCOUNT_NOT_FOUND);

        let receiver_account =
            self.accounts.get(&receiver).expect(ACCOUNT_NOT_FOUND);

        self.transfer_tokens(owner_account, receiver_account, value);

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

    fn approve(&mut self, spender: Account, value: u64) {
        // owner of the funds
        let owner = sender_account();

        let allowances = self.allowances.entry(owner).or_default();

        allowances.insert(spender, value);

        abi::emit(
            "approve",
            ApproveEvent {
                sender: owner,
                spender,
                value,
            },
        );
    }
}

#[no_mangle]
unsafe extern "C" fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(initial_accounts, governance)| {
        STATE.init(initial_accounts, governance);
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
unsafe extern "C" fn allowance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(owner, spender)| STATE.allowance(owner, spender))
}

#[no_mangle]
unsafe extern "C" fn transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(receiver, value)| STATE.transfer(receiver, value))
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
unsafe extern "C" fn transfer_governance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |new_governance| {
        STATE.transfer_governance(new_governance);
    })
}

#[no_mangle]
unsafe extern "C" fn renounce_governance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.renounce_governance())
}

#[no_mangle]
unsafe extern "C" fn governance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(): ()| STATE.governance)
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

/*
 * Helper functions
 */

/// Determines and returns the sender of the current transfer.
///
/// If the sender is an external account, return the transaction origin.
/// If the sender is a contract, return the calling contract.
///
/// # Returns
///
/// An `Account` representing the token sender.
///
/// # Panics
///
/// - If no public sender is available (shielded transactions are not supported)
/// - If no caller can be determined (impossible case)
fn sender_account() -> Account {
    let tx_origin = abi::public_sender().expect(SHIELDED_NOT_SUPPORTED);

    let caller = abi::caller().expect("ICC expects a caller");

    // Identifies the sender by checking the call stack and transaction origin:
    // - For direct external account transactions (call stack length = 1),
    //   returns the transaction origin
    // - For non-protocol contracts that call the token (call stack length > 1),
    //   returns the immediate calling contract
    if abi::callstack().len() == 1 {
        // This also implies, that the call directly originates via the protocol
        // transfer contract i.e., the caller is the transfer
        // contract
        Account::External(tx_origin)
    } else {
        Account::Contract(caller)
    }
}
