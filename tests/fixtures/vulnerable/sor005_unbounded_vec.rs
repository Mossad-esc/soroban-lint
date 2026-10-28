//! SOR005 vulnerable fixture: Vec loaded, grown, stored without a length cap.
//!
//! This is the README's primary example. A growing Vec under a single fixed
//! key will eventually hit resource limits, making the contract unusable.

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
    /// Appends a user to an unbounded list — can grow until it hits resource limits.
    pub fn add_user(env: Env, new_user: Address) {
        let mut users: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Users)
            .unwrap_or(Vec::new(&env));
        users.push_back(new_user);
        env.storage().persistent().set(&DataKey::Users, &users);
    }
}
