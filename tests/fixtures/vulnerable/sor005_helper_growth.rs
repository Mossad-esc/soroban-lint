//! SOR005 vulnerable fixture: growth delegated to a helper function.
//!
//! The load-grow-store pattern happens inside a helper rather than directly
//! in the entrypoint. The rule should flag the helper.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
pub enum DataKey {
    Users,
}

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    /// Delegates to a helper that does the unbounded growth.
    pub fn register(env: Env, new_user: Address) {
        Self::append_user(&env, new_user);
    }

    fn append_user(env: &Env, user: Address) {
        let mut users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Users)
            .unwrap_or(Vec::new(env));
        users.push_back(user);
        env.storage().persistent().set(&DataKey::Users, &users);
    }
}
