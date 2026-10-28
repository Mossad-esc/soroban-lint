//! SOR007 safe fixture: initialize with both guard and auth.
//!
//! This is the README's safe example.

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
    /// Safe: checks for existing admin (guard) and requires auth.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }
}
