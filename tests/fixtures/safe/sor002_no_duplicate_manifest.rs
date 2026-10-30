// tests/fixtures/safe/sor002_no_duplicate_manifest.rs
//
// SOR002 safe: This file is a sub-module of a crate (not lib.rs / main.rs).
// The Cargo-manifest check should NOT fire here, only in the crate root.
// This fixture guards against the regression where the manifest finding was
// emitted N times — once per .rs file in the crate.

#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct Helper;

#[contractimpl]
impl Helper {
    /// Helper module — the SOR002 manifest check fires only in lib.rs, not here.
    pub fn get_value(_env: Env) -> u32 {
        42
    }
}
