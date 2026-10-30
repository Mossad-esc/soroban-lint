// tests/fixtures/safe/sor007_constructor_exempt.rs
//
// SOR007 safe: __constructor is exempt from the "unprotected-initialize" check
// because the Soroban runtime guarantees it runs exactly once at deploy time.
// Corpus triage confirmed 15 false positives from this pattern in soroban-examples.

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    /// Called once by the deployment framework. No guard or auth needed here.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }
}
