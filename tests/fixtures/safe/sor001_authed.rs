// Fixture: safe/sor001_authed.rs
// Demonstrates the README's safe SOR001 example: `transfer` correctly calls
// `from.require_auth()` before mutating storage.

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
    /// Transfers `amount` from `from` to `to`.
    ///
    /// SAFE: calls `from.require_auth()` before any storage mutation.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_bal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let to_bal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_bal - amount));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_bal + amount));
    }
}
