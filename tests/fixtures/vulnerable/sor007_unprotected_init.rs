//! SOR007 vulnerable fixture: initialize with no guard and no auth.
//!
//! This is the README's primary vulnerable example: anyone can call it,
//! and it can be called multiple times.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// No guard, no auth — anyone can seize ownership or reset configuration.
    pub fn initialize(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }
}
