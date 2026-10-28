//! SOR004 safe fixture: Admin in instance() storage.
//!
//! Critical data (admin address) is correctly stored in instance() storage,
//! which is appropriate for contract-wide configuration.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Correct: admin is stored in instance storage which persists.
    pub fn initialize(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().extend_ttl(1000u32, 10000u32);
    }

    /// Correct: per-user balances in persistent storage.
    pub fn set_balance(env: Env, user: Address, amount: i128) {
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user), &amount);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Balance(user), 1000u32, 10000u32);
    }
}
