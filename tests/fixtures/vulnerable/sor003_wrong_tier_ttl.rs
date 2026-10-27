// Fixture: vulnerable/sor003_wrong_tier_ttl.rs
// Demonstrates SOR003: persistent write + only an instance extend_ttl.
// The persistent entry is still at risk of expiry.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    User(Address),
    Admin,
}

#[contract]
pub struct Registry;

const TTL_THRESHOLD: u32 = 1000;
const TTL_EXTEND_TO: u32 = 10000;

#[contractimpl]
impl Registry {
    /// Registers a user and bumps the instance TTL but forgets the persistent
    /// TTL.
    ///
    /// VULNERABLE: persistent write without a persistent extend_ttl.
    pub fn register(env: Env, user: Address) {
        user.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::User(user), &true);
        // Only bumps instance storage — the persistent entry is still at risk.
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}
