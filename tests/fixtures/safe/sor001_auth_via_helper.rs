// Fixture: safe/sor001_auth_via_helper.rs
// Demonstrates that SOR001 credits auth performed in a private helper.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey { Balance(Address) }

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Transfer via helper — auth is in `assert_caller`, not directly here.
    ///
    /// SAFE: auth is performed in the private helper `assert_caller`.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        Self::assert_caller(&from);
        let from_bal: i128 = env.storage().persistent()
            .get(&DataKey::Balance(from.clone())).unwrap_or(0);
        let to_bal: i128 = env.storage().persistent()
            .get(&DataKey::Balance(to.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Balance(from), &(from_bal - amount));
        env.storage().persistent().set(&DataKey::Balance(to), &(to_bal + amount));
    }

    fn assert_caller(who: &Address) {
        who.require_auth();
    }
}
