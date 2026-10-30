// Deep nesting fixture — tests stack safety of the AST visitor.
// Deeply nested function calls and blocks should not overflow the stack
// or cause a panic. Exit code must be 0 or 1, never 2.

#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct DeepContract;

#[allow(clippy::too_many_arguments)]
fn add_deep(a: i64, b: i64, c: i64, d: i64, e: i64, f: i64, g: i64, h: i64) -> i64 {
    ((((a + b) + c) + d) + e) + f + g + h
}

#[contractimpl]
impl DeepContract {
    pub fn compute(_env: Env) -> i64 {
        // Nested function calls exercising the visitor's recursion.
        add_deep(
            add_deep(1, 2, 3, 4, 5, 6, 7, 8),
            add_deep(9, 10, 11, 12, 13, 14, 15, 16),
            add_deep(17, 18, 19, 20, 21, 22, 23, 24),
            add_deep(25, 26, 27, 28, 29, 30, 31, 32),
            add_deep(33, 34, 35, 36, 37, 38, 39, 40),
            add_deep(41, 42, 43, 44, 45, 46, 47, 48),
            add_deep(49, 50, 51, 52, 53, 54, 55, 56),
            add_deep(57, 58, 59, 60, 61, 62, 63, 64),
        )
    }
}
