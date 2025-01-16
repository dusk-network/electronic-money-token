// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_core::abi::{self, ContractId, CONTRACT_ID_BYTES};
use dusk_core::signatures::bls::Signature;
use ttoken_types::admin_management::arguments::PauseToggle;
use ttoken_types::admin_management::events::PauseToggled;
use ttoken_types::admin_management::PAUSED_MESSAGE;
use ttoken_types::ownership::arguments::{RenounceOwnership, TransferOwnership};
use ttoken_types::ownership::events::{OwnerShipRenouncedEvent, OwnershipTransferredEvent};
use ttoken_types::ownership::{
    EXPECT_CONTRACT, OWNER_NOT_FOUND, UNAUTHORIZED_CONTRACT, UNAUTHORIZED_EXT_ACCOUNT,
};
use ttoken_types::sanctions::arguments::Sanction;
use ttoken_types::sanctions::events::AccountStatusEvent;
use ttoken_types::sanctions::{BLOCKED, FROZEN};
use ttoken_types::supply_management::arguments::{Burn, Mint};
use ttoken_types::supply_management::events::{BurnEvent, MintEvent};
use ttoken_types::supply_management::SUPPLY_OVERFLOW;
use ttoken_types::*;

const DEFAULT_OWNER: Account = Account::Contract(ContractId::from_bytes([0; CONTRACT_ID_BYTES]));

/// The state of the token contract.
struct TokenState {
    accounts: BTreeMap<Account, AccountInfo>,
    allowances: BTreeMap<Account, BTreeMap<Account, u64>>,
    supply: u64,

    owner: Account,

    is_paused: bool,
}

impl TokenState {
    fn init(&mut self, accounts: Vec<(Account, u64)>, owner: Account) {
        for (account, balance) in accounts {
            let account = self.accounts.entry(account).or_insert(AccountInfo::EMPTY);
            account.balance += balance;
            self.supply += balance;
        }

        // Set the owner
        self.owner = owner;

        // Always insert owner
        self.accounts
            .entry(self.owner)
            .or_insert(AccountInfo::EMPTY);
    }
}

static mut STATE: TokenState = TokenState {
    accounts: BTreeMap::new(),
    allowances: BTreeMap::new(),
    supply: 0,
    owner: DEFAULT_OWNER,
    is_paused: false,
};

/// Access control implementation.
impl TokenState {
    fn owner_info_mut(&mut self) -> &mut AccountInfo {
        self.accounts.get_mut(&self.owner).expect(OWNER_NOT_FOUND)
    }

    fn authorize_owner(&self, sig_msg: Vec<u8>, sig: Signature) {
        match &self.owner {
            Account::External(pk) => {
                assert!(
                    abi::verify_bls(sig_msg, *pk, sig),
                    "{}",
                    UNAUTHORIZED_EXT_ACCOUNT
                )
            }
            Account::Contract(contract_id) => assert!(
                &abi::caller().expect(EXPECT_CONTRACT) == contract_id,
                "{}",
                UNAUTHORIZED_CONTRACT
            ),
        }
    }

    fn transfer_ownership(&mut self, transfer_owner: TransferOwnership) {
        let sig = *transfer_owner.signature();
        let sig_msg = transfer_owner.signature_message().to_vec();

        let prev_owner_account = self.owner_info_mut();

        if transfer_owner.nonce() != prev_owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let previous_owner = self.owner;
        match previous_owner {
            Account::External(pk) => {
                assert!(
                    abi::verify_bls(sig_msg, pk, sig),
                    "{}",
                    UNAUTHORIZED_EXT_ACCOUNT
                )
            }
            Account::Contract(contract_id) => assert!(
                abi::caller().expect(EXPECT_CONTRACT) == contract_id,
                "{}",
                UNAUTHORIZED_CONTRACT
            ),
        }

        let new_owner = *transfer_owner.new_owner();
        self.owner = new_owner;
        // Always insert owner
        self.accounts.entry(new_owner).or_insert(AccountInfo::EMPTY);

        abi::emit(
            OwnershipTransferredEvent::TOPIC,
            OwnershipTransferredEvent {
                previous_owner,
                new_owner,
            },
        );
    }

    fn renounce_ownership(&mut self, renounce_owner: RenounceOwnership) {
        let sig = *renounce_owner.signature();
        let sig_msg = renounce_owner.signature_message().to_vec();

        let owner_account = self.owner_info_mut();

        if renounce_owner.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }
        self.authorize_owner(sig_msg, sig);

        let previous_owner = self.owner;
        self.owner = DEFAULT_OWNER;

        abi::emit(
            OwnerShipRenouncedEvent::TOPIC,
            OwnerShipRenouncedEvent { previous_owner },
        );
    }

    fn blocked(&self, account: Account) -> bool {
        let owner_account = self.accounts.get(&account);

        match owner_account {
            Some(account) => account.is_blocked(),
            None => false,
        }
    }

    fn frozen(&self, account: Account) -> bool {
        let owner_account = self.accounts.get(&account);

        match owner_account {
            Some(account) => account.is_frozen(),
            None => false,
        }
    }

    fn block(&mut self, block_account: Sanction) {
        assert!(
            block_account.sanction_type() == AccountInfo::BLOCKED,
            "Invalid sanction type"
        );

        let sig = *block_account.signature();
        let sig_msg = block_account.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let owner_account = self.owner_info_mut();

        if block_account.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let account = *block_account.account();
        let account_info = self.accounts.get_mut(&account).expect(OWNER_NOT_FOUND);

        account_info.block();

        abi::emit(
            AccountStatusEvent::BLOCKED_TOPIC,
            AccountStatusEvent::blocked(account),
        );
    }

    fn freeze(&mut self, freeze_account: Sanction) {
        assert!(
            freeze_account.sanction_type() == AccountInfo::FROZEN,
            "Invalid sanction type"
        );

        let sig = *freeze_account.signature();
        let sig_msg = freeze_account.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let owner_account = self.owner_info_mut();

        if freeze_account.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let account = *freeze_account.account();
        let account_info = self.accounts.get_mut(&account).expect(OWNER_NOT_FOUND);

        account_info.freeze();

        abi::emit(
            AccountStatusEvent::FROZEN_TOPIC,
            AccountStatusEvent::frozen(account),
        );
    }

    fn unblock(&mut self, unblock_account: Sanction) {
        let sig = *unblock_account.signature();
        let sig_msg = unblock_account.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let owner_account = self.owner_info_mut();

        if unblock_account.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let account = *unblock_account.account();
        let account_info = self.accounts.get_mut(&account).expect(OWNER_NOT_FOUND);

        assert!(account_info.is_blocked(), "The account is not blocked");

        account_info.unblock();

        abi::emit(
            AccountStatusEvent::UNBLOCKED_TOPIC,
            AccountStatusEvent::unblocked(account),
        );
    }

    fn unfreeze(&mut self, unfreeze_account: Sanction) {
        let sig = *unfreeze_account.signature();
        let sig_msg = unfreeze_account.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let owner_account = self.owner_info_mut();

        if unfreeze_account.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let account = *unfreeze_account.account();
        let account_info = self.accounts.get_mut(&account).expect(OWNER_NOT_FOUND);

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
    fn mint(&mut self, mint: Mint) {
        let sig = *mint.signature();
        let sig_msg = mint.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let owner_account = self.owner_info_mut();

        if mint.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let receiver = *mint.receiver();
        let receiver_account = self.accounts.entry(receiver).or_insert(AccountInfo::EMPTY);

        let amount_minted = mint.amount();

        // Prevent overflow
        self.supply = match self.supply.checked_add(amount_minted) {
            Some(supply) => supply,
            None => panic!("{}", SUPPLY_OVERFLOW),
        };

        receiver_account.balance += amount_minted;

        abi::emit(
            MintEvent::TOPIC,
            MintEvent {
                amount_minted,
                receiver,
            },
        );
    }

    fn burn(&mut self, burn: Burn) {
        let sig = *burn.signature();
        let sig_msg = burn.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);

        let burn_account = self.owner_info_mut();

        if burn.nonce() != burn_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let value = burn.amount();
        if burn_account.balance < value {
            panic!("{}", BALANCE_TOO_LOW);
        } else {
            burn_account.balance -= value;
        }

        // this can never fail, as the balance is checked above
        self.supply -= value;

        abi::emit(
            BurnEvent::TOPIC,
            BurnEvent {
                amount_burned: value,
                burned_by: self.owner,
            },
        );
    }
}

/// Administrative functions.
impl TokenState {
    fn is_paused(&self) -> bool {
        self.is_paused
    }

    fn toggle_pause(&mut self, toggle: PauseToggle) {
        let sig = *toggle.signature();
        let sig_msg = toggle.signature_message().to_vec();

        self.authorize_owner(sig_msg, sig);
        let owner_account = self.owner_info_mut();

        if toggle.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        self.is_paused = !self.is_paused;

        abi::emit(
            PauseToggled::TOPIC,
            PauseToggled {
                paused: self.is_paused,
            },
        );
    }

    /// note: this function will fail if the balance of the obliged sender is too low. It will **not** default to the maximum available balance.
    fn force_transfer(&mut self, transfer: Transfer) {
        self.authorize_owner(transfer.signature_message().to_vec(), *transfer.signature());

        let obliged_sender = *transfer.sender();

        let owner_account = self.owner_info_mut();

        if transfer.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        let obliged_sender_account = self
            .accounts
            .get_mut(&obliged_sender)
            .expect(ACCOUNT_NOT_FOUND);

        let value = transfer.value();

        if obliged_sender_account.balance < value {
            panic!("{}", BALANCE_TOO_LOW);
        }

        obliged_sender_account.balance -= value;

        let to = *transfer.receiver();
        let to_account = self.accounts.entry(to).or_insert(AccountInfo::EMPTY);

        // this can never overflow as value + balance is never higher than total supply
        to_account.balance += value;

        abi::emit(
            TransferEvent::FORCE_TRANSFER_TOPIC,
            TransferEvent {
                sender: obliged_sender,
                spender: None,
                receiver: to,
                value,
            },
        );
    }
}

/// Basic token contract implementation.
impl TokenState {
    fn name(&self) -> String {
        String::from("Transparent Fungible Token Sample")
    }

    fn symbol(&self) -> String {
        String::from("TFTS")
    }

    fn decimals(&self) -> u8 {
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

    fn allowance(&self, allowance: Allowance) -> u64 {
        match self.allowances.get(&allowance.owner) {
            Some(allowances) => allowances.get(&allowance.spender).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// Initates a `Transfer` from the sender to the receiver with the specified value.
    ///
    /// Both the sender and the receiver are accounts.
    ///
    /// # Note
    /// the sender must not be blocked or frozen.
    /// the receiver must not be blocked but can be frozen.
    fn transfer(&mut self, transfer: Transfer) {
        assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

        let sender = *transfer.sender();

        let sender_account = self.accounts.get_mut(&sender).expect(ACCOUNT_NOT_FOUND);
        assert!(!sender_account.is_blocked(), "{}", BLOCKED);
        assert!(!sender_account.is_frozen(), "{}", FROZEN);

        let value = transfer.value();

        if sender_account.balance < value {
            panic!("{}", BALANCE_TOO_LOW);
        }

        if transfer.nonce() != sender_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        sender_account.balance -= value;

        // If the sender is an external account, the signature must be verified with the specified sender pk in the Account.
        // If the sender is a contract, the caller must be verified against the specified sender contract id in the Account.
        if let Account::External(sender_key) = *transfer.sender() {
            let sig = *transfer.signature();
            let sig_msg = transfer.signature_message().to_vec();

            if !abi::verify_bls(sig_msg, sender_key, sig) {
                panic!("Invalid signature");
            }
        } else {
            assert_eq!(
                *transfer.sender(),
                Account::Contract(abi::caller().expect(EXPECT_CONTRACT)),
                "{}",
                INVALID_CALLER
            )
        };

        let to = *transfer.receiver();
        let to_account = self.accounts.entry(to).or_insert(AccountInfo::EMPTY);

        assert!(!to_account.is_blocked(), "{}", BLOCKED);

        // this can never overflow as value + balance is never higher than total supply
        to_account.balance += value;

        abi::emit(
            TransferEvent::TRANSFER_TOPIC,
            TransferEvent {
                sender,
                spender: None,
                receiver: to,
                value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(contract) = to {
            if let Err(err) =
                abi::call::<_, ()>(contract, "token_received", &TransferInfo { sender, value })
            {
                panic!("Failed calling `token_received` on the receiving contract: {err}");
            }
        }
    }

    /// Note:
    /// the spender must not be blocked or frozen.
    /// the actual owner of the funds must not be blocked or frozen.
    /// the receiver must not be blocked but can be frozen.
    fn transfer_from(&mut self, transfer: TransferFrom) {
        assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

        let spender = *transfer.spender();

        let spender_account = self.accounts.entry(spender).or_insert(AccountInfo::EMPTY);
        assert!(!spender_account.is_blocked(), "{}", BLOCKED);
        assert!(!spender_account.is_frozen(), "{}", FROZEN);

        if transfer.nonce() != spender_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        // If the spender is an external account, the signature must be verified with the specified sender pk in the Account.
        // If the spender is a contract, the caller must be verified against the specified sender contract id in the Account.
        if let Account::External(sender_key) = *transfer.spender() {
            let sig = *transfer.signature();
            let sig_msg = transfer.signature_message().to_vec();

            if !abi::verify_bls(sig_msg, sender_key, sig) {
                panic!("Invalid signature");
            }
        } else {
            assert_eq!(
                *transfer.sender(),
                Account::Contract(abi::caller().expect(EXPECT_CONTRACT)),
                "{}",
                INVALID_CALLER
            )
        };

        let owner = *transfer.sender();

        let allowance = self
            .allowances
            .get_mut(&owner)
            .expect("The account has no allowances")
            .get_mut(&spender)
            .expect("The spender is not allowed to use the account");

        let value = transfer.value();
        if value > *allowance {
            panic!("The spender can't spent the defined amount");
        }

        let owner_account = self.accounts.get_mut(&owner).expect(ACCOUNT_NOT_FOUND);
        assert!(!owner_account.is_blocked(), "{}", BLOCKED);
        assert!(!owner_account.is_frozen(), "{}", FROZEN);

        if owner_account.balance < value {
            panic!("{}", BALANCE_TOO_LOW);
        }

        *allowance -= value;
        owner_account.balance -= value;

        let to = *transfer.receiver();
        let to_account = self.accounts.entry(to).or_insert(AccountInfo::EMPTY);
        assert!(!to_account.is_blocked(), "{}", BLOCKED);

        // this can never overflow as value + balance is never higher than total supply
        to_account.balance += value;

        abi::emit(
            TransferEvent::TRANSFER_TOPIC,
            TransferEvent {
                sender: owner,
                spender: Some(spender),
                receiver: to,
                value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(contract) = to {
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

    /// Note:
    /// the sender must not be blocked or frozen.
    /// the receiver must not be blocked but can be frozen.
    fn transfer_from_contract(&mut self, transfer: TransferFromContract) {
        assert!(!self.is_paused, "{}", PAUSED_MESSAGE);

        let contract = abi::caller().expect("Must be called by a contract");
        let contract = Account::Contract(contract);

        let contract_account = self.accounts.get_mut(&contract).expect(ACCOUNT_NOT_FOUND);
        assert!(!contract_account.is_blocked(), "{}", BLOCKED);
        assert!(!contract_account.is_frozen(), "{}", FROZEN);

        if contract_account.balance < transfer.value {
            panic!("{}", BALANCE_TOO_LOW);
        }

        contract_account.balance -= transfer.value;

        let to_account = self
            .accounts
            .entry(transfer.receiver)
            .or_insert(AccountInfo::EMPTY);
        assert!(!to_account.is_blocked(), "{}", BLOCKED);

        to_account.balance += transfer.value;

        abi::emit(
            "transfer",
            TransferEvent {
                sender: contract,
                spender: None,
                receiver: transfer.receiver,
                value: transfer.value,
            },
        );

        // if the transfer is to a contract, the acceptance function of said
        // contract is called. if it fails (panic or OoG) the transfer
        // also fails.
        if let Account::Contract(to_contract) = transfer.receiver {
            if let Err(err) = abi::call::<_, ()>(
                to_contract,
                "token_received",
                &TransferInfo {
                    sender: contract,
                    value: transfer.value,
                },
            ) {
                panic!("Failed calling `token_received` on the receiving contract: {err}");
            }
        }
    }

    fn approve(&mut self, approve: Approve) {
        let owner = approve.sender();

        let owner_account = self.accounts.entry(*owner).or_insert(AccountInfo::EMPTY);

        if approve.nonce() != owner_account.increment_nonce() {
            panic!("{}", NONCE_NOT_SEQUENTIAL);
        }

        // If the sender is an external account, the signature must be verified with the specified sender pk in the Account.
        // If the sender is a contract, the caller must be verified against the specified sender contract id in the Account.
        if let Account::External(sender_key) = *approve.sender() {
            let sig = *approve.signature();
            let sig_msg = approve.signature_message().to_vec();

            if !abi::verify_bls(sig_msg, sender_key, sig) {
                panic!("Invalid signature");
            }
        } else {
            assert_eq!(
                *approve.sender(),
                Account::Contract(abi::caller().expect(EXPECT_CONTRACT)),
                "{}",
                INVALID_CALLER
            )
        };

        let spender = *approve.spender();

        let allowances = self.allowances.entry(*owner).or_default();

        let value = approve.value();
        allowances.insert(spender, value);

        abi::emit(
            "approve",
            ApproveEvent {
                sender: *owner,
                spender,
                value,
            },
        );
    }
}

#[no_mangle]
unsafe fn init(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |(initial_accounts, owner)| {
        STATE.init(initial_accounts, owner)
    })
}

#[no_mangle]
unsafe fn name(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.name())
}

#[no_mangle]
unsafe fn symbol(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.symbol())
}

#[no_mangle]
unsafe fn decimals(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.decimals())
}

#[no_mangle]
unsafe fn total_supply(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.total_supply())
}

#[no_mangle]
unsafe fn account(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.account(arg))
}

#[no_mangle]
unsafe fn allowance(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.allowance(arg))
}

#[no_mangle]
unsafe fn transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer(arg))
}

#[no_mangle]
unsafe fn transfer_from(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from(arg))
}

#[no_mangle]
unsafe fn transfer_from_contract(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_from_contract(arg))
}

#[no_mangle]
unsafe fn approve(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.approve(arg))
}

/*
 * Access control functions
 */

#[no_mangle]
unsafe fn transfer_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.transfer_ownership(arg))
}

#[no_mangle]
unsafe fn renounce_ownership(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.renounce_ownership(arg))
}

#[no_mangle]
unsafe fn owner(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.owner)
}

/*
* Supply management functions
*/

#[no_mangle]
unsafe fn mint(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.mint(arg))
}

#[no_mangle]
unsafe fn burn(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.burn(arg))
}

/*
 * Administrative functions
 */

#[no_mangle]
unsafe fn toggle_pause(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.toggle_pause(arg))
}

#[no_mangle]
unsafe fn is_paused(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |_: ()| STATE.is_paused())
}

#[no_mangle]
unsafe fn force_transfer(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.force_transfer(arg))
}

/*
 * Sanctions functions
 */

#[no_mangle]
unsafe fn block(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.block(arg))
}

#[no_mangle]
unsafe fn freeze(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.freeze(arg))
}

#[no_mangle]
unsafe fn unblock(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.unblock(arg))
}

#[no_mangle]
unsafe fn unfreeze(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.unfreeze(arg))
}

#[no_mangle]
unsafe fn blocked(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.blocked(arg))
}

#[no_mangle]
unsafe fn frozen(arg_len: u32) -> u32 {
    abi::wrap_call(arg_len, |arg| STATE.frozen(arg))
}
