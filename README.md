# Dusk - Electronic Money Token

> Unstable : No guarantees can be made regarding the function signatures stability, the project is in development.

## Introduction

Dusk's chain supports cheaply verifying zero-knowledge proofs passed to a token-contract. Tokens
leveraging this functionality will often obfuscate certain pieces of data - such as the amount being
sent, or the receiver - while still keeping tranfers secure. Here we are *not* making use of this
functionality, and are instead defining a classic token - here called a *transparent* token due to
its properties.

The token is fungible, i.e. each token is exactly the same as another.

## Build

Have [`rust`] and [`make`] installed and run:

```sh
make
```

[`rust`]: https://www.rust-lang.org/tools/install
[`make`]: https://www.gnu.org/software/make

## Features

Contracts implementing this standard will allow the user to:

- Transfer tokens from one account to another
- Get the total supply of tokens
- Approve third-parties spending tokens of an account
- Get the current token balance of an account

### Data Structures

Some of the functionality of the token-contract requires data to be sent to it that assures ownership of a
given public key, as well as ensuring the non-repeatability of certain calls. The data that a user
will use to interact with this token-contract is defined in the [`core` crate] in this repository. The
example token-contract implementation in the [`token` crate] makes use of [`rkyv`] serialization. This
is convenient for the implementation since [`dusk-core`] abi supports it natively, however it is not a
requirement, and implementors may choose any serialization they wish. This will result in different
gas costs. As a consequence, this specification *does not require* specific serialization from
contracts wishing to implement it.

[`core` crate]: ./core
[`token` crate]: ./token
[`rkyv`]: https://github.com/rkyv/rkyv
[`dusk-abi`]: https://github.com/dusk-network/rusk/core/src/abi.rs

### Functions

The following functions will be defined by a token-contract implementing this specification. `&self` and
`&mut self` are used to denote whether a function mutates the state handled by the token-contract, and
closely matches its use in the implementation.

```rust
fn name(&self) -> String;
fn symbol(&self) -> String;
fn decimals(&self) -> u8;
fn total_supply(&self) -> u64;
fn account(&self, _: PublicKey) -> AccountData;
fn allowance(&self, _: Allowance) -> u64;
fn transfer(&mut self, _: Transfer);
fn transfer_from(&mut self, _: TransferFrom);
fn approve(&mut self, _: Approve);
```

For this token-contract we use BLS12_381 public keys, since Dusk has native support for them. However,
implementers of the token standard may choose a different type of cryptography for their own token.

### Events

On a `transfer`, `transfer_from`, and `approve` events are emitted related to the action performed.
The data included with these events is defined with the `TransferEvent` and `ApproveEvent`.

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
