//! SOR007 safe fixture: only the already-initialized guard (no auth).
//!
//! Per the README and rule docs, having only one protection is allowed but
//! discouraged. The rule does NOT flag this case.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env};

#[contracterror]
#[derive(Clone, Copy)]
pub enum Error {
    AlreadyInitialized = 1,
}

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Only a guard, no auth — allowed (not flagged), but discouraged.
    // soroban-lint: allow(SOR001) reason="tests guard-only initialize for SOR007"
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }
}
