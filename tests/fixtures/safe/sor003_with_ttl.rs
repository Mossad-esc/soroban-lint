// Fixture: safe/sor003_with_ttl.rs
// Demonstrates the safe pattern for SOR003 (README example):
// persistent write with a matching persistent extend_ttl.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    User(Address),
}

#[contract]
pub struct Registry;

const TTL_THRESHOLD: u32 = 1000;
const TTL_EXTEND_TO: u32 = 10000;

#[contractimpl]
impl Registry {
    /// Registers a user.
    ///
    /// SAFE: writes to persistent storage and extends the TTL so the entry
    /// stays alive.
    pub fn register(env: Env, user: Address) {
        user.require_auth();
        let key = DataKey::User(user);
        env.storage().persistent().set(&key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}
