// Unusual attribute paths fixture.
// Tests that the tool handles attribute paths it doesn't recognise
// without panicking: #[cfg(...)], #[allow(...)], nested cfg_attr, etc.

#![no_std]
#![allow(dead_code)]
#![cfg_attr(all(not(test), feature = "production"), deny(warnings))]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[cfg(feature = "advanced")]
use soroban_sdk::token;

#[derive(Clone)]
#[contracttype]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum DataKey {
    Admin,
}

/// A contract with unusual-but-legal attribute combinations.
#[contract]
pub struct AttrContract;

#[contractimpl]
impl AttrContract {
    #[allow(clippy::missing_panics_doc)]
    pub fn read(env: Env, _caller: Address) -> bool {
        env.storage().instance().has(&DataKey::Admin)
    }
}
