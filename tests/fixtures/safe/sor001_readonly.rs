// Fixture: safe/sor001_readonly.rs
// Demonstrates a read-only entrypoint: `balance` only calls `get`, never
// `set`/`remove`, so SOR001 must NOT flag it.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Returns the balance for `who`.
    ///
    /// SAFE: read-only; no storage mutation, no auth required by SOR001.
    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or(0)
    }
}
