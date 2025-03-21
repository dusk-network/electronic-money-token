
# Dusk - Electronic Money Token


## 📋 Features

### Normal Functions

```rust
// View/Query Functions
fn name() -> String
fn symbol() -> String
fn decimals() -> u8
fn total_supply() -> u64
fn account(account: Account) -> AccountInfo
fn allowance(allowance: Allowance) -> u64
fn governance() -> Account
fn is_paused() -> bool
fn blocked(account: Account) -> bool
fn frozen(account: Account) -> bool

// Standard Functions
fn transfer(transfer: Transfer)
fn transfer_from(transfer: TransferFrom)
fn approve(approve: Approve)
```

### Admin Functionality 

```rust
// Admin Only Functions
fn init(initial_accounts: Vec<(Account, u64)>, governance: Account)
fn transfer_governance(transfer_governance: TransferGovernance)
fn renounce_governance()
fn mint(receiver: Account, amount: u64)
fn burn(amount: u64)
fn toggle_pause()
fn force_transfer(transfer: Transfer, obliged_sender: Account)
fn freeze(freeze_account: Sanction)
fn unfreeze(unfreeze_account: Sanction)
fn block(block_account: Sanction)
fn unblock(unblock_account: Sanction)
```

### Events

```rust
GovernanceTransferredEvent::TOPIC  -> GovernanceTransferredEvent
GovernanceRenouncedEvent::TOPIC  -> GovernanceRenouncedEvent
BLOCKED_TOPIC  -> AccountStatusEvent
FROZEN_TOPIC  -> AccountStatusEvent
UNBLOCKED_TOPIC  -> AccountStatusEvent
UNFROZEN_TOPIC  -> AccountStatusEvent
MINT_TOPIC  -> TransferEvent
BURN_TOPIC  -> TransferEvent
PauseToggled::TOPIC  -> PauseToggled
FORCE_TRANSFER_TOPIC  -> TransferEvent
TRANSFER_TOPIC  -> TransferEvent
APPROVE_TOPIC,  -> ApproveEvent
```

### Roles

There are normal users & a governance account. The governance account can also be a contract for more sophisticated role-based access control.

### View/Query Functions

#### `name() -> String`
- Returns the name of the token

#### `symbol() -> String`
- Returns the token symbol

#### `decimals() -> u8`
- Returns the number of decimal places for the token

#### `total_supply() -> u64`
- Returns the total supply of tokens in circulation

#### `account(account: Account) -> AccountInfo`
- Returns account information for the given account, including balance and sanction status

#### `allowance(allowance: Allowance) -> u64`
- Returns the amount of tokens that a spender is allowed to spend on behalf of an owner

#### `governance() -> Account`
- Returns the current governance account that has administrative privileges

#### `is_paused() -> bool`
- Returns whether the contract is currently paused

#### `blocked(account: Account) -> bool`
- Returns whether an account is blocked (sanctioned)

#### `frozen(account: Account) -> bool`
- Returns whether an account is frozen (sanctioned)

### Standard Functions

#### `transfer(transfer: Transfer)`
##### Invariant
- Cannot be called if contract is paused
- Sender must not be blocked or frozen
- Receiver must not be blocked (can be frozen)

##### Functionality
- Transfers tokens from sender to receiver
- Emits `TransferEvent::TRANSFER_TOPIC` with `TransferEvent`

##### `transfer_from(transfer: TransferFrom)`
##### Invariant
- Cannot be called if contract is paused
- Spender and owner must not be blocked or frozen
- Receiver must not be blocked (can be frozen)

##### Functionality 
- Transfers tokens from an owner to a receiver. The tokens are transferred by a spender on behalf of the owner
- Requires prior approval from the owner through the `approve(approve: Approve)` function
- Emits `TransferEvent::TRANSFER_TOPIC` with `TransferEvent`

##### `approve(approve: Approve)`

##### Functionality 
- Allows an owner to authorize a spender to spend tokens on their behalf
- Emits `"approve"` with `ApproveEvent`

### Governance Only Functions

#### Global invariants

Any governance function can only be called by the governance account.

#### `transfer_governance(transfer_governance: TransferGovernance)`
- Transfers governance rights to a new account
- Emits `GovernanceTransferredEvent::TOPIC` with `GovernanceTransferredEvent`

#### `renounce_governance()`
- Renounces governance, setting governance to zero address
- Emits `GovernanceRenouncedEvent::TOPIC` with `GovernanceRenouncedEvent`

#### `mint(receiver: Account, amount: u64)`
- Creates new tokens and assigns them to the receiver
- Emits `MINT_TOPIC` with `TransferEvent`

#### `burn(amount: u64)`
- Destroys tokens from the governance account
- Emits `BURN_TOPIC` with `TransferEvent`

#### `toggle_pause()`
- Toggles the paused state of the contract
- Emits `PauseToggled::TOPIC` with `PauseToggled`

#### `force_transfer(transfer: Transfer, obliged_sender: Account)`
- Forces a transfer of tokens from one account to another
- Emits `TransferEvent::FORCE_TRANSFER_TOPIC` with `TransferEvent`

#### `block(block_account: Sanction)`
- Blocks an account from sending or receiving tokens
- Emits `AccountStatusEvent::BLOCKED_TOPIC` with `AccountStatusEvent`

#### `freeze(freeze_account: Sanction)`
- Freezes an account, preventing it from sending tokens but allowing receipt
- Emits `AccountStatusEvent::FROZEN_TOPIC` with `AccountStatusEvent`

#### `unblock(unblock_account: Sanction)`
- Removes a block sanction from an account
- Emits `AccountStatusEvent::UNBLOCKED_TOPIC` with `AccountStatusEvent`

#### `unfreeze(unfreeze_account: Sanction)`
- Removes a freeze sanction from an account
- Emits `AccountStatusEvent::UNFROZEN_TOPIC` with `AccountStatusEvent`

### Special functions

#### `init(initial_accounts: Vec<(Account, u64)>, governance: Account)`

##### Invariant

- Can only be called during deployment. This is a dusk/dusk-vm specific feature.

##### Functionality

- Initializes the token contract with initial account balances and sets the governance account
- Called only once when contract is deployed
- Does not emit events

## 🛠️ Build and Tests

Have [`rust`] and [`make`] installed and run:

```sh
make
```

To run tests:

```sh
make test
```

See also `make help` for all the available commands

[`rust`]: https://www.rust-lang.org/tools/install
[`make`]: https://www.gnu.org/software/make

### Additional Considerations

#### 32 vs 64-bit

Dusk supports both 32 and 64-bit WebAssembly contracts. Compiling this token-contract for 32-bit however,
comes with serious size constraints for the state the token-contract can manage - 4GiB. This would mean
that the token-contract would hit a limit in terms of the number of accounts it can manage that is too low
to be usable. As such, we include a script that downloads a compiler toolchain that supports 64-bit
WebAssembly and registers it with `rustup`, and use `make` to call this automatically when run.

#### Stripping WebAssembly token-Contract

Transaction sizes are a consideration for any chain, and given that deployment costs scale per byte
deployed it is in the best interests of token-contract developers to minimize the payload. As such we
include a script that downloads a tool that strips the compiled binary of any superfluous
information such as debug symbols.
