/// Error messages for overflow when minting tokens.
pub const SUPPLY_OVERFLOW: &str = "Supply overflow";

/// Events emitted by supply management transactions.
pub mod events {
    /// The topic of the mint event.
    pub const MINT_TOPIC: & str = "mint";
    /// The topic of the burn event.
    pub const BURN_TOPIC: & str = "burn";
}
