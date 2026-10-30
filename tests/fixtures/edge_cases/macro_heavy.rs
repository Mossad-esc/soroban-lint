// Macro-heavy fixture.
// soroban-lint operates on pre-expansion source, so macro-generated code is
// not analyzed. The tool must not panic on heavy macro usage.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env};

#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub enum DataKey {
    Counter,
    Paused,
}

#[contract]
pub struct MacroContract;

#[contractimpl]
impl MacroContract {
    pub fn get(env: Env) -> u64 {
        // symbol_short! is a proc-macro; the tool handles it gracefully.
        let _sym = symbol_short!("counter");
        env.storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::Counter)
            .unwrap_or(0)
    }
}
