// Fixture: safe/sor001_auth_for_args.rs
// Demonstrates the `require_auth_for_args` form: SOR001 must NOT flag an
// entrypoint that calls `require_auth_for_args` instead of `require_auth`.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, vec, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Approves `spender` to transfer up to `amount` on behalf of `from`.
    ///
    /// SAFE: uses `require_auth_for_args` — should not trigger SOR001.
    pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
        from.require_auth_for_args(
            (symbol_short!("approve"), spender.clone(), amount).into_val(&env),
        );
        env.storage()
            .persistent()
            .set(&DataKey::Balance(spender), &amount);
    }
}
