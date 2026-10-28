//! SOR005 safe fixture: collection loaded but not written back.
//!
//! Reading a collection and returning values from it without writing it
//! back is read-only and safe from an unbounded-growth perspective.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
pub enum DataKey {
    Users,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Returns how many users are stored — read-only, no growth.
    pub fn user_count(env: Env) -> u32 {
        let users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Users)
            .unwrap_or(Vec::new(&env));
        users.len()
    }
}
