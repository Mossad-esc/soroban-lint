//! SOR008 vulnerable fixture: missing no_std, std import, and f64 usage.

// No #![no_std] — first violation.
use std::collections::HashMap; // std import — second violation.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Uses f64 — third violation (README's primary example).
    pub fn ratio(env: Env, a: i128, b: i128) -> f64 {
        a as f64 / b as f64
    }

    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or(0i128)
    }
}
