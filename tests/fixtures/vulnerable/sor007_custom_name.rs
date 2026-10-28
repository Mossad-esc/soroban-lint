//! SOR007 vulnerable fixture: custom init name via config.
//!
//! Used to test that rules.SOR007.init_names works. This file is run
//! in the test with custom init_names = ["bootstrap"].

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Named "bootstrap" — flagged only when init_names includes "bootstrap".
    pub fn bootstrap(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }
}
