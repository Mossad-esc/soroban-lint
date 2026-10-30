# Introduction

**soroban-lint** is a static analyzer and linter for Soroban smart contracts on Stellar. It catches missing authorization, unchecked arithmetic, storage and TTL mistakes, and other known Soroban pitfalls before they reach the network. Think "Slither for Soroban."

## Why soroban-lint?

Soroban contracts run on a network where bugs are expensive and upgrades are hard. Many real-world contract vulnerabilities are not exotic. They are repeatable patterns:

- A function that moves funds but never calls `require_auth`.
- Integer arithmetic that silently overflows in a release build.
- Persistent data that expires because nobody extended its TTL.
- Critical state (an admin address, a balance) stored in temporary storage.
- A `Vec` that grows forever under a single storage key until it hits resource limits.
- An `initialize` function anyone can call twice.

These are exactly the kind of issues a static analyzer can find in seconds, in CI, on every pull request. `soroban-lint` is built to do that, with Soroban-specific knowledge that general-purpose tools like Clippy do not have.

### Who it's for

| Audience | How it helps |
|---|---|
| Contract developers | Fast feedback while writing code, in the terminal or editor |
| Security auditors | A first-pass triage tool to surface common issues before manual review |
| Maintainers | A CI gate that prevents known bug classes from being merged |
| Teams new to Soroban | Rule docs double as a checklist of Soroban best practices |

## Features

- **Soroban-aware rules** targeting auth, arithmetic, storage, TTL, error handling, and initialization.
- **Fast.** Pure source analysis built on `syn`, with no compilation step required.
- **Works on stable Rust.** No nightly toolchain needed for the core tool.
- **Multiple output formats:** human-readable terminal diagnostics, JSON, and SARIF for GitHub code scanning.
- **Configurable.** Enable, disable, and re-severity rules via `soroban-lint.toml`.
- **Inline suppression** with required justification support.
- **Workspace-aware.** Understands Cargo workspaces and finds contract crates automatically.
- **CI-first.** Ships with a reusable GitHub Action and sensible exit codes.
- **Documented rules.** Every rule has a page with a vulnerable example, a safe example, and rationale.

## Quick Start

```bash
# Install
cargo install soroban-lint

# Run from the root of a Soroban project
soroban-lint check

# Run on a specific path
soroban-lint check ./contracts/token

# Produce a SARIF report for GitHub code scanning
soroban-lint check --format sarif --output results.sarif
```

Example output:

```text
error[SOR001]: entrypoint `transfer` writes to storage but never calls `require_auth`
  --> contracts/token/src/lib.rs:42:12
   |
42 |     pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
   |            ^^^^^^^^
   |
   = help: call `from.require_auth()` before mutating state
   = docs: https://your-org.github.io/soroban-lint/rules/SOR001

warning[SOR003]: persistent storage written without a TTL extension
  --> contracts/token/src/lib.rs:58:9
   ...

Found 1 error, 1 warning in 2 files (0.04s)
```
