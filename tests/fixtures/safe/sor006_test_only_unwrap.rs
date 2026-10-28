//! SOR006 safe fixture: unwrap() inside #[cfg(test)] is not flagged.
//!
//! Test code routinely uses unwrap() for brevity. This is acceptable because
//! tests do not run on the network.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Safe production entrypoint — uses ok_or, no panics.
    pub fn get_admin(env: Env) -> Result<Address, u32> {
        let admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(1u32)?;
        Ok(admin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_admin() {
        // unwrap() in test code: acceptable, not flagged by SOR006.
        let _: Address = Token::get_admin(todo!()).unwrap();
    }
}
