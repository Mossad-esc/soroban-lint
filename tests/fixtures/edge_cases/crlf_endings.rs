// Windows CRLF line endings fixture.
// This file uses CRLF (\r\n) line endings.
// soroban-lint must parse and analyze it without panicking.

#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct CrlfContract;

#[contractimpl]
impl CrlfContract {
    pub fn safe_read(env: Env, key: Address) -> bool {
        env.storage().persistent().has(&key)
    }
}
