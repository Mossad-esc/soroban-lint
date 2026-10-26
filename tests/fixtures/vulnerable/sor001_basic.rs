// Fixture: vulnerable/sor001_basic.rs
// Demonstrates the README's SOR001 example: `transfer` writes to persistent
// storage but never calls `require_auth`.

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
    /// Transfers `amount` from `from` to `to`.
    ///
    /// VULNERABLE: no `require_auth` call — anyone can move `from`'s funds.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
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
        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &(from_bal - amount));
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(to_bal + amount));
    }
}
