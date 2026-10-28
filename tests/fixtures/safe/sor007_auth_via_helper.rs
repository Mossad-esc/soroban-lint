//! SOR007 safe fixture: auth performed via a private helper (call graph credit).

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
    /// Safe: delegates auth to a helper.
    pub fn initialize(env: Env, admin: Address) {
        Self::assert_deployer_auth(&admin);
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    fn assert_deployer_auth(who: &Address) {
        who.require_auth();
    }
}
