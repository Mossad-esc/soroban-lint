# Suppressing Findings

Use inline comments when a finding is a justified exception.

```rust
// soroban-lint: allow(SOR001) reason="permissionless by design; credits caller-specified address"
pub fn deposit(env: Env, to: Address, amount: i128) {
    // ...
}
```

## Supported forms

```rust
// soroban-lint: allow(SOR001)                 // applies to the next item
// soroban-lint: allow(SOR001, SOR003)         // multiple rules
#![allow(soroban_lint::SOR002)]                // file or crate level (planned)
```

## Best practices

- Always include a `reason`. Reviewers should be able to tell why a check was skipped.
- Prefer narrow, per-function suppression over disabling a rule globally.
- Consider enabling `require_suppression_reason = true` in config to enforce this in CI (planned).
