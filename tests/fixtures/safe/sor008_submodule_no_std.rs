// tests/fixtures/safe/sor008_submodule_no_std.rs
//
// SOR008 safe: This file is NOT named lib.rs or main.rs.  Even though the
// contract model may identify it as a contract file (it has #[contractimpl]),
// the "missing #![no_std]" finding must NOT fire here.  The no_std attribute
// is crate-level and lives in lib.rs; emitting it for every sub-module is a
// false positive confirmed in corpus triage (soroban-examples token crate,
// three test-only files in mint-lock, modular_account, and privacy-pools).

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct SubContract;

#[contractimpl]
impl SubContract {
    pub fn get(_env: Env) -> u32 {
        0
    }
}
