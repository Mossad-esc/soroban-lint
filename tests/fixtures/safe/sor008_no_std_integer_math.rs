//! SOR008 safe fixture: #![no_std] with integer math (no floats, no std).

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

/// Fixed-point scaling factor: 1_000_000 = 6 decimal places.
const SCALE: i128 = 1_000_000;

#[contractimpl]
impl Token {
    /// Safe: uses integer fixed-point math instead of f64.
    pub fn ratio(env: Env, a: i128, b: i128) -> i128 {
        // Multiply by SCALE first to preserve precision.
        (a * SCALE) / b
    }

    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or(0i128)
    }
}
