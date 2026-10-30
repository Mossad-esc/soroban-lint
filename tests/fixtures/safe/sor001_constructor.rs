// tests/fixtures/safe/sor001_constructor.rs
//
// SOR001 safe: __constructor is exempt from require-auth because the Soroban
// deployment framework calls it exactly once during contract deployment.
// Corpus triage confirmed 15 false positives from this pattern in soroban-examples.

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Counter,
}

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    /// Called once at deploy time by the Soroban runtime.
    /// Cannot be called again by any user — no require_auth needed.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Counter, &0_u64);
    }

    /// Regular entrypoint — must require auth.
    pub fn increment(env: Env, caller: Address) {
        caller.require_auth();
        let n: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::Counter, &(n + 1));
    }
}
