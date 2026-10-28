//! SOR008 safe fixture: #![cfg_attr(not(test), no_std)] form.
//!
//! Some projects gate `no_std` with `cfg_attr` so tests can use `std`.
//! The rule treats this as equivalent to `#![no_std]`.

#![cfg_attr(not(test), no_std)]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or(0i128)
    }
}

#[cfg(test)]
mod tests {
    // In tests we can use std freely.
    use std::vec::Vec;

    #[test]
    fn test_example() {
        let _v: Vec<u32> = vec![1, 2, 3];
    }
}
