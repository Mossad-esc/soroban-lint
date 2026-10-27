// Fixture: safe/sor003_ttl_via_helper.rs
// Demonstrates that SOR003 credits extend_ttl performed in a private helper.

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
    /// SAFE: extend_ttl is performed in the helper `bump`.
    pub fn register(env: Env, user: Address) {
        user.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::User(user.clone()), &true);
        Self::bump(&env);
    }

    fn bump(env: &Env) {
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::User, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}
