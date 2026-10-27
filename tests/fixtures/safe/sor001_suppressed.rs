// Fixture: safe/sor001_suppressed.rs
// Demonstrates inline suppression: the entrypoint is intentionally
// permissionless and is suppressed with a reason.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Balance(Address),
}

#[contract]
pub struct Vault;

#[contractimpl]
impl Vault {
    /// Deposits funds credited to the caller-supplied address.
    ///
    /// SAFE (suppressed): this function is intentionally permissionless;
    /// it credits the supplied address but does not require their signature
    /// because anyone may deposit on behalf of anyone else.
    // soroban-lint: allow(SOR001) reason="permissionless by design; credits caller-specified address"
    pub fn deposit(env: Env, to: Address, amount: i128) {
        let bal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::Balance(to), &(bal + amount));
    }
}
