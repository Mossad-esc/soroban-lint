// Fixture: safe/sor002_checked_arithmetic.rs
// Demonstrates the safe pattern for SOR002: use checked_add / checked_sub
// and return a typed error on overflow.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    Overflow = 1,
    Underflow = 2,
}

#[contract]
pub struct Token;

#[contractimpl]
impl Token {
    /// Transfer `amount` from `from` to `to`.
    ///
    /// SAFE: uses `checked_sub` / `checked_add` and returns a typed error
    /// on overflow or underflow.
    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), Error> {
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

        let new_from = from_bal.checked_sub(amount).ok_or(Error::Underflow)?;
        let new_to = to_bal.checked_add(amount).ok_or(Error::Overflow)?;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(from), &new_from);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &new_to);
        Ok(())
    }
}
