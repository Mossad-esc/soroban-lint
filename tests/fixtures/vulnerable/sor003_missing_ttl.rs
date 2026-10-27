// Fixture: vulnerable/sor003_missing_ttl.rs
// Demonstrates SOR003: `register` writes to persistent storage but
// never calls extend_ttl, so the entry will expire.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    User(Address),
}

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    /// Registers a user.
    ///
    /// VULNERABLE: writes to persistent storage but does not extend the TTL.
    /// The entry will eventually expire and the user registration will be lost.
    pub fn register(env: Env, user: Address) {
        user.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::User(user), &true);
    }
}
