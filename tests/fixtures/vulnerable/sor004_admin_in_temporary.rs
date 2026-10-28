//! SOR004 vulnerable fixture: Admin written to temporary() storage.
//!
//! This matches the README example verbatim.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
    Balance(Address),
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Stores the admin in temporary storage — which can silently expire.
    pub fn setup(env: Env, admin: Address) {
        env.storage().temporary().set(&DataKey::Admin, &admin);
    }
}
