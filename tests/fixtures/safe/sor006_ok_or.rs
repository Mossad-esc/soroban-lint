//! SOR006 safe fixture: ok_or with #[contracterror] enum.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env};

#[contracterror]
#[derive(Clone, Copy)]
pub enum Error {
    NotInitialized = 1,
    Unauthorized = 2,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Balance(Address),
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Safe: uses ok_or to propagate a typed error instead of panicking.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        let admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        Ok(admin)
    }

    /// Safe: unwrap_or_default does not panic.
    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(who))
            .unwrap_or_default()
    }
}
