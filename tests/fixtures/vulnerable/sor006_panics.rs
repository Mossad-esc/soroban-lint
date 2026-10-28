//! SOR006 vulnerable fixture: unwrap, expect, panic!, and direct indexing.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};

#[contracttype]
pub enum DataKey {
    Admin,
    Items,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Uses unwrap — panics if Admin not set.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    /// Uses expect — panics with a message.
    pub fn require_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("admin must be set")
    }

    /// Uses panic! directly.
    pub fn not_implemented(env: Env) {
        panic!("this function is not implemented yet");
    }

    /// Uses direct vec indexing with a non-constant index.
    pub fn get_item(env: Env, idx: u32) -> u32 {
        let items: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::Items)
            .unwrap_or(Vec::new(&env));
        items[idx]
    }
}
