//! SOR004 safe fixture: non-critical key (nonce cache) in temporary storage.
//!
//! Temporary storage is appropriate for short-lived, non-critical data like
//! rate-limiting nonces or replay-protection counters.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Nonce(Address),
    ReplayCache(Address),
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Stores a replay nonce in temporary storage — appropriate, as nonces are
    /// short-lived by design.
    pub fn use_nonce(env: Env, user: Address) {
        env.storage().temporary().set(&DataKey::Nonce(user), &true);
    }

    /// Caches replay data in temporary storage.
    pub fn cache_replay(env: Env, user: Address) {
        env.storage()
            .temporary()
            .set(&DataKey::ReplayCache(user), &1u32);
    }
}
