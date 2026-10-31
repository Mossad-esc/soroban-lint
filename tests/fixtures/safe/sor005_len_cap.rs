//! SOR005 safe fixture: explicit length cap with typed error.
//!
//! Growth is guarded by a `.len()` check and a `return Err(...)` before
//! appending, preventing unbounded growth.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
pub enum DataKey {
    Users,
}

#[contract]
pub struct Contract;

const MAX_USERS: u32 = 100;

#[contractimpl]
impl Contract {
    /// Enforces a maximum size before appending — safe.
    pub fn add_user(env: Env, new_user: Address) -> Result<(), u32> {
        new_user.require_auth();
        let mut users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Users)
            .unwrap_or(Vec::new(&env));
        if users.len() >= MAX_USERS {
            return Err(1u32); // Error::Full
        }
        users.push_back(new_user);
        env.storage().persistent().set(&DataKey::Users, &users);
        Ok(())
    }
}
