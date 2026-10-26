// Fixture: vulnerable/sor001_remove.rs
// Demonstrates SOR001: `revoke` removes from storage but never calls
// `require_auth`, so anyone can revoke any user's access.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Allowance(Address),
    User(Address),
}

#[contract]
pub struct AccessList;

#[contractimpl]
impl AccessList {
    /// Adds a user to the access list.
    ///
    /// VULNERABLE: no `require_auth` — anyone can add any user.
    pub fn add_user(env: Env, user: Address) {
        env.storage()
            .persistent()
            .set(&DataKey::User(user), &true);
    }

    /// Removes a user from the access list.
    ///
    /// VULNERABLE: no `require_auth` — anyone can remove any user.
    pub fn revoke(env: Env, user: Address) {
        env.storage()
            .persistent()
            .remove(&DataKey::User(user));
    }
}
