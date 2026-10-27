// Fixture: vulnerable/sor002_raw_arithmetic.rs
// Demonstrates SOR002: raw `+` and `-` on i128 balance variables
// without checked_add / checked_sub.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Transfer `amount` from `from` to `to`.
    ///
    /// VULNERABLE: uses `+` and `-` directly on i128 balances; in a release
    /// build without `overflow-checks = true` these operations will silently
    /// wrap on overflow.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_bal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(from.clone()))
            .unwrap_or(0);
        let to_bal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        // VULNERABLE: unchecked subtraction and addition
        let new_from = from_bal - amount;
        let new_to = to_bal + amount;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_from);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &new_to);
    }

    /// Increment a counter using compound assignment on a u64.
    ///
    /// VULNERABLE: `count += 1u64` in a release build without
    /// `overflow-checks = true` will silently wrap when the counter
    /// reaches `u64::MAX`.
    pub fn increment(env: Env) {
        env.storage().instance().get::<_, ()>(&DataKey::Balance(
            soroban_sdk::Address::from_string(&env, &soroban_sdk::String::from_str(&env, "G...")),
        ));
        let mut count: u64 = 0u64;
        count += 1u64;
        env.storage()
            .instance()
            .set(&DataKey::Balance(soroban_sdk::Address::from_string(
                &env,
                &soroban_sdk::String::from_str(&env, "G..."),
            )), &count);
    }
}
