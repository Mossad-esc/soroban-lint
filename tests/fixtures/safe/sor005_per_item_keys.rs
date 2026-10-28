//! SOR005 safe fixture: per-item keys with a counter.
//!
//! Each user is stored under their own key. No single entry grows unboundedly.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    User(Address),
    UserCount,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Stores each user under their own key — does not accumulate in one entry.
    pub fn add_user(env: Env, new_user: Address) {
        new_user.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::User(new_user.clone()), &true);
        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::UserCount)
            .unwrap_or(0u32);
        env.storage()
            .persistent()
            .set(&DataKey::UserCount, &(count + 1u32));
    }
}
