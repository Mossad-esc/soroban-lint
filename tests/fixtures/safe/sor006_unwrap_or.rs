//! SOR006 safe fixture: unwrap_or_default does not panic.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
    Count,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Safe: unwrap_or_default returns the type's default, never panics.
    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or_default()
    }

    /// Safe: unwrap_or with a literal fallback.
    pub fn count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Count)
            .unwrap_or(0u32)
    }
}
