// Fixture: safe/sor003_instance_with_ttl.rs
// Demonstrates the safe pattern for SOR003 for instance storage:
// instance write with a matching instance extend_ttl.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct Token;

const TTL_THRESHOLD: u32 = 1000;
const TTL_EXTEND_TO: u32 = 10000;

#[contractimpl]
impl Token {
    /// Sets the admin address.
    ///
    /// SAFE: writes to instance storage and extends the instance TTL.
    pub fn set_admin(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}
