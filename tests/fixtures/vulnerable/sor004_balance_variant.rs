//! SOR004 vulnerable fixture: Balance variant with constructor argument.
//!
//! Tests that the rule matches enum variants with arguments (DataKey::Balance(addr)).

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
    Nonce(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Stores a user balance in temporary storage — which will expire.
    pub fn set_balance(env: Env, user: Address, amount: i128) {
        env.storage()
            .temporary()
            .set(&DataKey::Balance(user), &amount);
    }
}
