# soroban-lint

**A static analyzer and linter for Soroban smart contracts on Stellar.**
Catch missing authorization, unchecked arithmetic, storage and TTL mistakes, and other known Soroban pitfalls before they reach the network. Think "Slither for Soroban."

[![CI](https://img.shields.io/badge/CI-passing-brightgreen)](#)
[![Crates.io](https://img.shields.io/badge/crates.io-soroban--lint-orange)](#)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-informational)](#requirements)
[![Status](https://img.shields.io/badge/status-early%20development-yellow)](#project-status)

> **Project status:** Early development. This README describes the intended design and CLI. Items are marked with their status in the [rule catalog](#rule-catalog) and [roadmap](#roadmap). Replace the `your-org` placeholders and badge URLs with your real ones when you publish.

---

## Table of Contents

1. [Why soroban-lint?](#why-soroban-lint)
2. [Features](#features)
3. [Quick Start](#quick-start)
4. [Installation](#installation)
5. [Usage](#usage)
6. [Rule Catalog](#rule-catalog)
7. [Rule Details and Examples](#rule-details-and-examples)
8. [Configuration](#configuration)
9. [Suppressing Findings](#suppressing-findings)
10. [Output Formats](#output-formats)
11. [CI/CD Integration](#cicd-integration)
12. [How It Works](#how-it-works)
13. [Project Structure](#project-structure)
14. [Writing a New Rule](#writing-a-new-rule)
15. [Testing](#testing)
16. [Limitations and Known Gaps](#limitations-and-known-gaps)
17. [Roadmap](#roadmap)
18. [Contributing](#contributing)
19. [Security](#security)
20. [FAQ](#faq)
21. [Acknowledgements](#acknowledgements)
22. [License](#license)

---

## Why soroban-lint?

Soroban contracts run on a network where bugs are expensive and upgrades are hard. Many real-world contract vulnerabilities are not exotic. They are repeatable patterns:

- A function that moves funds but never calls `require_auth`.
- Integer arithmetic that silently overflows in a release build.
- Persistent data that expires because nobody extended its TTL.
- Critical state (an admin address, a balance) stored in temporary storage.
- A `Vec` that grows forever under a single storage key until it hits resource limits.
- An `initialize` function anyone can call twice.

These are exactly the kind of issues a static analyzer can find in seconds, in CI, on every pull request. `soroban-lint` is built to do that, with Soroban-specific knowledge that general-purpose tools like Clippy do not have.

**Who it's for**

| Audience | How it helps |
|---|---|
| Contract developers | Fast feedback while writing code, in the terminal or editor |
| Security auditors | A first-pass triage tool to surface common issues before manual review |
| Maintainers | A CI gate that prevents known bug classes from being merged |
| Teams new to Soroban | Rule docs double as a checklist of Soroban best practices |

---

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

---

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

---

## Installation

### Requirements

- Rust (stable, 1.75 or newer recommended) if building from source
- A Soroban contract project to analyze. The linter reads source files, so it does not need the Stellar CLI or a network connection.

### From crates.io

```bash
cargo install soroban-lint
```

### From source

```bash
git clone https://github.com/your-org/soroban-lint.git
cd soroban-lint
cargo build --release
./target/release/soroban-lint --version
```

### Prebuilt binaries

Download binaries for Linux, macOS, and Windows from the [Releases](https://github.com/your-org/soroban-lint/releases) page.

### As a cargo subcommand

If the binary is named `cargo-soroban-lint`, you can also run:

```bash
cargo soroban-lint check
```

---

## Usage

```text
soroban-lint <COMMAND>

Commands:
  check     Analyze contract source and report findings
  rules     List available rules
  explain   Show detailed documentation for a rule
  init      Create a default soroban-lint.toml
  help      Print help
```

### `check`

```bash
soroban-lint check [PATH ...] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `PATH` | Files or directories to analyze | `.` |
| `--format <FMT>` | `pretty`, `json`, or `sarif` | `pretty` |
| `--output <FILE>` | Write the report to a file instead of stdout | stdout |
| `--config <FILE>` | Path to a config file | `./soroban-lint.toml` |
| `--select <IDS>` | Run only these rules (comma-separated) | all enabled |
| `--ignore <IDS>` | Skip these rules (comma-separated) | none |
| `--min-severity <LVL>` | Report only `note`, `warning`, or `error` and above | `note` |
| `--fail-on <LVL>` | Exit non-zero at or above this severity | `error` |
| `--no-color` | Disable colored output | auto-detected |
| `--include-tests` | Also analyze `#[cfg(test)]` modules and `tests/` | off |

### `rules` and `explain`

```bash
soroban-lint rules                 # table of all rules, status, default severity
soroban-lint explain SOR001        # full documentation in the terminal
```

### `init`

```bash
soroban-lint init                  # writes soroban-lint.toml with defaults
```

### Exit codes

| Code | Meaning |
|---|---|
| `0` | No findings at or above the `--fail-on` level |
| `1` | Findings at or above the `--fail-on` level |
| `2` | Usage, configuration, or internal error (for example, a file that could not be parsed) |

---

## Rule Catalog

| ID | Name | Default severity | Category | Status |
|---|---|---|---|---|
| [SOR001](#sor001--missing-require-auth) | `missing-require-auth` | error | Access control | Planned (MVP) |
| [SOR002](#sor002--unchecked-arithmetic) | `unchecked-arithmetic` | warning | Arithmetic | Planned (MVP) |
| [SOR003](#sor003--missing-ttl-extension) | `missing-ttl-extension` | warning | Storage / TTL | Planned (MVP) |
| [SOR004](#sor004--critical-data-in-temporary-storage) | `critical-data-in-temporary` | error | Storage | Planned |
| [SOR005](#sor005--unbounded-storage-growth) | `unbounded-storage-growth` | warning | Storage / DoS | Planned |
| [SOR006](#sor006--panic-in-entrypoint) | `panic-in-entrypoint` | warning | Error handling | Planned |
| [SOR007](#sor007--unprotected-initialize) | `unprotected-initialize` | error | Access control | Planned |
| [SOR008](#sor008--std-or-float-usage) | `std-or-float-usage` | error | Compatibility | Planned |

Severity levels: `error` (likely a real bug or vulnerability), `warning` (risky pattern, review needed), `note` (style or best-practice suggestion).

---

## Rule Details and Examples

Each rule below also has its own page under [`docs/rules/`](docs/rules/). Examples are illustrative. Always verify guidance against the current [soroban-sdk documentation](https://docs.rs/soroban-sdk) and Stellar's published security guidance, since SDK APIs and best practices evolve.

### SOR001 · missing-require-auth

**What it detects:** A public entrypoint inside a `#[contractimpl]` block that mutates contract storage but never calls `require_auth()` or `require_auth_for_args()` (directly or, in later versions, through a helper).

**Why it matters:** In Soroban, authorization is explicit. If an entrypoint that changes balances, ownership, or configuration does not require authorization from the right `Address`, anyone can call it.

**Vulnerable**

```rust
#[contractimpl]
impl Token {
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // No authorization: anyone can move `from`'s funds.
        let from_bal = Self::balance(&env, &from);
        let to_bal = Self::balance(&env, &to);
        env.storage().persistent().set(&DataKey::Balance(from), &(from_bal - amount));
        env.storage().persistent().set(&DataKey::Balance(to), &(to_bal + amount));
    }
}
```

**Safe**

```rust
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    from.require_auth();
    // ... balance updates
}
```

**False-positive notes:** Intentionally permissionless functions (for example, a public `deposit` credited to the caller-supplied address, or `bump_ttl`) should be suppressed with a justification. See [Suppressing Findings](#suppressing-findings).

---

### SOR002 · unchecked-arithmetic

**What it detects:**

1. Raw `+`, `-`, `*`, `/`, `%`, and compound assignment operators on integer values inside entrypoints and their helpers.
2. A `Cargo.toml` release profile that does not set `overflow-checks = true`.

**Why it matters:** Release builds of Rust wrap on overflow by default. For financial logic, silent wrap-around is a serious bug. Soroban contracts should either enable `overflow-checks` in the release profile or use `checked_*` operations and return a typed error.

**Vulnerable**

```rust
let new_balance = balance + amount;
```

**Safe**

```rust
let new_balance = balance
    .checked_add(amount)
    .ok_or(Error::Overflow)?;
```

**Recommended `Cargo.toml`**

```toml
[profile.release]
overflow-checks = true
```

**Notes:** Without type information, the analyzer cannot always tell whether an operand is an integer, a `Duration`, or a custom type with an `Add` impl. The rule uses heuristics (literal types, annotated types, known SDK return types) and may be configured to be stricter or looser. See [Limitations](#limitations-and-known-gaps).

---

### SOR003 · missing-ttl-extension

**What it detects:** Writes to `persistent()` or `instance()` storage in an entrypoint with no corresponding `extend_ttl` call for the same storage tier in the same function or its helpers.

**Why it matters:** Soroban state entries have a time-to-live. Entries that are not extended eventually expire (persistent entries become archived and require restoration; instance and temporary behave differently). Contracts that forget TTL management can break in production long after tests pass.

**Vulnerable**

```rust
pub fn register(env: Env, user: Address) {
    user.require_auth();
    env.storage().persistent().set(&DataKey::User(user), &true);
}
```

**Safe**

```rust
pub fn register(env: Env, user: Address) {
    user.require_auth();
    let key = DataKey::User(user);
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}
```

**Notes:** Instance storage TTL is extended with `env.storage().instance().extend_ttl(...)`. Some contracts centralize TTL bumping in a dedicated function. The planned call-graph pass will recognize that pattern.

---

### SOR004 · critical-data-in-temporary-storage

**What it detects:** Keys whose names or enum variants suggest critical state (for example `Admin`, `Owner`, `Balance`, `Allowance`, `Config`) written to `temporary()` storage.

**Why it matters:** Temporary storage is intended for short-lived data and can disappear when its TTL runs out. Losing an admin address or a user balance because it lived in the wrong storage tier is a correctness and security failure.

**Vulnerable**

```rust
env.storage().temporary().set(&DataKey::Admin, &admin);
```

**Safe**

```rust
env.storage().instance().set(&DataKey::Admin, &admin);
```

The list of "critical" key-name patterns is configurable.

---

### SOR005 · unbounded-storage-growth

**What it detects:** A `Vec` or `Map` loaded from storage under a fixed key, appended to (`push_back`, `set`, `insert`), and written back, with no visible length bound.

**Why it matters:** Entries have size limits, and every operation that reads or writes a large entry costs more resources. A list that grows with every user action can eventually make the contract unusable (a denial of service on itself).

**Vulnerable**

```rust
let mut users: Vec<Address> = env.storage().persistent().get(&DataKey::Users).unwrap_or(Vec::new(&env));
users.push_back(new_user);
env.storage().persistent().set(&DataKey::Users, &users);
```

**Safer pattern**

Store each item under its own key (for example `DataKey::User(addr)`) and keep a counter, or enforce a maximum length and return an error when it is reached.

---

### SOR006 · panic-in-entrypoint

**What it detects:** `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, and unchecked indexing in public entrypoints.

**Why it matters:** Panics produce opaque failures. Typed errors defined with `#[contracterror]` give callers and integrators meaningful, stable error codes and make behavior explicit.

**Vulnerable**

```rust
let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
```

**Safe**

```rust
let admin: Address = env
    .storage()
    .instance()
    .get(&DataKey::Admin)
    .ok_or(Error::NotInitialized)?;
```

---

### SOR007 · unprotected-initialize

**What it detects:** Functions named like `initialize`, `init`, or `__constructor`-style setup functions that neither check an "already initialized" flag nor require authorization from a deployer or admin.

**Why it matters:** An initializer that can be called by anyone, or called twice, lets an attacker seize ownership or reset configuration.

**Vulnerable**

```rust
pub fn initialize(env: Env, admin: Address) {
    env.storage().instance().set(&DataKey::Admin, &admin);
}
```

**Safe**

```rust
pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::AlreadyInitialized);
    }
    admin.require_auth();
    env.storage().instance().set(&DataKey::Admin, &admin);
    Ok(())
}
```

---

### SOR008 · std-or-float-usage

**What it detects:** A contract crate root missing `#![no_std]`, usage of `std::` paths, or `f32`/`f64` types and literals.

**Why it matters:** Soroban contracts compile to WebAssembly in a constrained environment. The standard library and floating-point arithmetic are not appropriate for contract code and can cause build failures or nondeterminism concerns.

**Vulnerable**

```rust
let ratio: f64 = a as f64 / b as f64;
```

**Safe**

Use fixed-point integer math with explicit scaling and rounding behavior.

---

## Configuration

`soroban-lint` looks for `soroban-lint.toml` in the current directory and walks up to the workspace root. Generate a starter file with `soroban-lint init`.

```toml
# soroban-lint.toml

[general]
# Paths to skip (glob patterns, relative to this file)
exclude = ["target/**", "**/tests/**", "**/test_snapshots/**"]
# Analyze #[cfg(test)] code
include_tests = false
# Exit non-zero at or above this severity: "note" | "warning" | "error"
fail_on = "error"

[rules.SOR001]
enabled = true
severity = "error"

[rules.SOR002]
enabled = true
severity = "warning"
# Also verify overflow-checks in Cargo.toml release profile
check_cargo_profile = true

[rules.SOR003]
enabled = true
severity = "warning"

[rules.SOR004]
enabled = true
severity = "error"
# Key names treated as critical (case-insensitive substring match)
critical_keys = ["admin", "owner", "balance", "allowance", "config", "supply"]

[rules.SOR005]
enabled = true
severity = "warning"

[rules.SOR006]
enabled = true
severity = "warning"

[rules.SOR007]
enabled = true
severity = "error"
# Function names considered initializers
init_names = ["initialize", "init", "setup"]

[rules.SOR008]
enabled = true
severity = "error"

# Functions that are intentionally permissionless and should not trigger SOR001
[rules.SOR001.allow_functions]
names = ["bump_ttl", "get_price"]
```

**Precedence (highest to lowest):** inline suppression, CLI flags, config file, built-in defaults.

---

## Suppressing Findings

Use inline comments when a finding is a justified exception.

```rust
// soroban-lint: allow(SOR001) reason="permissionless by design; credits caller-specified address"
pub fn deposit(env: Env, to: Address, amount: i128) {
    // ...
}
```

Supported forms:

```rust
// soroban-lint: allow(SOR001)                 // applies to the next item
// soroban-lint: allow(SOR001, SOR003)         // multiple rules
#![allow(soroban_lint::SOR002)]                // file or crate level (planned)
```

Good practices:

- Always include a `reason`. Reviewers should be able to tell why a check was skipped.
- Prefer narrow, per-function suppression over disabling a rule globally.
- Consider enabling `require_suppression_reason = true` in config to enforce this in CI (planned).

---

## Output Formats

### `pretty` (default)

Compiler-style diagnostics with source snippets, help text, and documentation links.

### `json`

```bash
soroban-lint check --format json
```

```json
{
  "tool": "soroban-lint",
  "version": "0.1.0",
  "summary": { "errors": 1, "warnings": 1, "notes": 0, "files_scanned": 2 },
  "findings": [
    {
      "rule": "SOR001",
      "name": "missing-require-auth",
      "severity": "error",
      "message": "entrypoint `transfer` writes to storage but never calls `require_auth`",
      "file": "contracts/token/src/lib.rs",
      "line": 42,
      "column": 12,
      "help": "call `from.require_auth()` before mutating state"
    }
  ]
}
```

### `sarif`

SARIF 2.1.0, suitable for GitHub code scanning, VS Code SARIF viewers, and other security dashboards.

```bash
soroban-lint check --format sarif --output soroban-lint.sarif
```

---

## CI/CD Integration

### GitHub Actions (using the reusable action)

```yaml
name: soroban-lint
on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read
  security-events: write   # required to upload SARIF

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run soroban-lint
        uses: your-org/soroban-lint@v0
        with:
          path: .
          format: sarif
          output: soroban-lint.sarif
          fail-on: error

      - name: Upload SARIF to GitHub code scanning
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: soroban-lint.sarif
```

### GitHub Actions (install via cargo)

```yaml
- uses: dtolnay/rust-toolchain@stable
- run: cargo install soroban-lint --locked
- run: soroban-lint check --fail-on warning
```

### Pre-commit hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: soroban-lint
        name: soroban-lint
        entry: soroban-lint check
        language: system
        pass_filenames: false
        files: \.rs$
```

### Recommended rollout for existing projects

1. Run `soroban-lint check` locally and review the findings.
2. Fix real issues, suppress justified exceptions, and commit a `soroban-lint.toml`.
3. Add the CI job with `fail-on = "error"`.
4. Tighten to `warning` once the codebase is clean.

---

## How It Works

```text
 ┌──────────────┐   ┌───────────────┐   ┌──────────────┐   ┌─────────────┐
 │  Discovery   │──▶│    Parsing    │──▶│  Rule engine │──▶│  Reporting  │
 │ cargo_metadata│  │  syn → AST    │   │ Visitors +   │   │ pretty/JSON │
 │ + ignore walk │  │ + span info   │   │ call graph   │   │ /SARIF      │
 └──────────────┘   └───────────────┘   └──────────────┘   └─────────────┘
```

1. **Discovery.** `cargo_metadata` finds workspace members. Crates depending on `soroban-sdk` are treated as contract crates. Files are collected with the `ignore` crate so `.gitignore` is respected.
2. **Parsing.** Each `.rs` file is parsed with `syn` into an AST. `proc-macro2` span locations provide line and column data.
3. **Context building.** The engine identifies `#[contract]` structs, `#[contractimpl]` blocks, `#[contracttype]` / `#[contracterror]` definitions, and public entrypoints.
4. **Rule execution.** Each rule implements a `Rule` trait and uses `syn::visit::Visit` to walk relevant nodes. Cross-function rules use a per-crate call graph (built with `petgraph`) so that auth or TTL logic in a private helper is credited to its callers.
5. **Suppression and config.** Findings are filtered by inline suppressions, config, and CLI flags.
6. **Reporting.** Findings are rendered in the selected format, and the exit code is computed from `--fail-on`.

### Analysis styles used by the rules

| Style | Example rules | Approach |
|---|---|---|
| AST pattern matching | SOR006, SOR008 | Find specific calls, macros, or types |
| Co-occurrence ("A should accompany B") | SOR001, SOR003 | Detect a storage write without a paired auth or TTL call |
| Manifest inspection | SOR002 (profile check) | Parse `Cargo.toml` |
| Naming/heuristic | SOR004, SOR007 | Match key and function names against configurable patterns |
| Dataflow-lite (planned) | SOR005 | Track a collection loaded, grown, and stored back |

---

## Project Structure

```text
soroban-lint/
├── Cargo.toml                  # Cargo workspace
├── crates/
│   ├── lint-core/              # Rule trait, Finding/Severity types, AST context, call graph
│   ├── lint-rules/             # One module per rule (sor001.rs, sor002.rs, ...)
│   ├── lint-report/            # pretty, JSON, and SARIF formatters
│   └── lint-cli/               # clap-based binary (`soroban-lint`)
├── tests/
│   └── fixtures/
│       ├── vulnerable/         # Contracts that SHOULD trigger a rule
│       └── safe/               # Contracts that should NOT trigger
├── docs/
│   ├── rules/                  # SOR001.md ... SOR008.md
│   └── book/                   # mdBook sources
├── action.yml                  # Reusable GitHub Action
├── soroban-lint.toml           # Example config
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
├── LICENSE-MIT
└── LICENSE-APACHE
```

---

## Writing a New Rule

1. **Pick an ID and name.** Use the next free `SORxxx` number and a kebab-case name.
2. **Create fixtures.** Add at least one contract in `tests/fixtures/vulnerable/` that triggers the rule and one in `tests/fixtures/safe/` that must not.
3. **Implement the rule** in `crates/lint-rules/src/sorxxx.rs`:

```rust
use lint_core::{Finding, Rule, RuleContext, Severity};
use syn::visit::Visit;

pub struct MyRule;

impl Rule for MyRule {
    fn id(&self) -> &'static str { "SOR009" }
    fn name(&self) -> &'static str { "my-rule" }
    fn default_severity(&self) -> Severity { Severity::Warning }

    fn check(&self, ctx: &RuleContext) -> Vec<Finding> {
        let mut visitor = MyVisitor { findings: Vec::new(), ctx };
        visitor.visit_file(ctx.ast());
        visitor.findings
    }
}

struct MyVisitor<'a> {
    findings: Vec<Finding>,
    ctx: &'a RuleContext<'a>,
}

impl<'ast, 'a> Visit<'ast> for MyVisitor<'a> {
    // override visit_* methods and push Findings
}
```

4. **Register it** in `crates/lint-rules/src/lib.rs`.
5. **Document it** in `docs/rules/SOR009.md` with: description, rationale, vulnerable example, safe example, configuration options, known false positives, and references.
6. **Add snapshot tests** (see [Testing](#testing)).
7. **Update the catalog** in this README and in `CHANGELOG.md`.

**Rule quality bar.** A rule should have a clear security or correctness rationale, a low false-positive rate on real contracts, an actionable message that tells the user how to fix the issue, and tests for both the vulnerable and the safe case.

---

## Testing

```bash
# Run all tests
cargo test --workspace

# Faster, with nextest
cargo nextest run --workspace

# Review and accept snapshot changes (insta)
cargo insta review

# Lints and formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Dependency hygiene
cargo deny check
cargo audit
```

**Test layers**

| Layer | Tooling | Purpose |
|---|---|---|
| Unit tests | built-in `#[test]` | Helpers, config parsing, suppression logic |
| Fixture tests | `insta` snapshots | Each rule's findings against vulnerable and safe contracts |
| CLI tests | `assert_cmd`, `predicates`, `tempfile` | Exit codes, flags, output formats |
| Corpus runs | script in `scripts/` | Run against real open-source Soroban contracts and track false-positive rates |

---

## Limitations and Known Gaps

Being honest about what a static tool cannot do is part of making it trustworthy.

- **Not an audit.** `soroban-lint` finds common patterns. It does not prove a contract is secure, and a clean report is not a guarantee. Use it alongside tests, fuzzing, formal methods, and professional audits.
- **Pre-expansion analysis.** The core engine reads source before macro expansion, so logic generated by custom macros is invisible to it.
- **No type resolution.** Because the analyzer uses `syn`, it does not know the real type of every expression. Some rules (notably SOR002 and SOR005) rely on heuristics and can produce false positives or negatives.
- **Intra-crate focus.** Cross-crate and cross-contract call analysis is limited. Auth enforced in a dependency or through another contract's call may not be recognized.
- **Heuristic naming rules.** SOR004 and SOR007 depend on naming conventions, which projects may not follow.
- **SDK drift.** Soroban's SDK and recommended patterns change. Rules are reviewed against the current SDK and may need updates between releases.

Type-aware analysis via `dylint` is on the [roadmap](#roadmap) to address several of these.

---

## Roadmap

**Phase 1: MVP**
- [ ] Workspace scaffolding, CLI, directory discovery
- [ ] `#[contractimpl]` and entrypoint detection
- [ ] SOR001 `missing-require-auth`
- [ ] SOR002 `unchecked-arithmetic` (including `Cargo.toml` profile check)
- [ ] SOR003 `missing-ttl-extension`
- [ ] Pretty terminal output
- [ ] Fixture and snapshot test suite

**Phase 2: Usable in teams**
- [ ] JSON and SARIF output
- [ ] `soroban-lint.toml` configuration
- [ ] Inline suppression comments
- [ ] Reusable GitHub Action
- [ ] Intra-crate call graph (credit helpers for auth and TTL)
- [ ] SOR004, SOR005, SOR006
- [ ] mdBook documentation site with one page per rule

**Phase 3: Hardening**
- [ ] SOR007, SOR008
- [ ] Corpus testing against open-source Soroban projects and a published false-positive report
- [ ] Prebuilt binaries via `cargo-dist`
- [ ] Parallel scanning with `rayon`
- [ ] Baseline files (ignore existing findings, fail only on new ones)

**Phase 4: Advanced**
- [ ] `--fix` for simple, safe rewrites
- [ ] Language Server (`tower-lsp`) and VS Code extension
- [ ] Type-aware rules via `dylint`
- [ ] Storage-layout and upgrade-safety checks
- [ ] WebAssembly playground

Have an idea? [Open an issue](https://github.com/Mossad-esc/soroban-lint/issues).

---

## Contributing

Contributions are welcome, from new rules and bug reports to documentation improvements.

1. **Find or open an issue.** Look for `good first issue` and `help wanted` labels, or propose a new rule using the *Rule proposal* issue template.
2. **Fork and branch.** `git checkout -b feat/sor009-my-rule`
3. **Make your change** following [Writing a New Rule](#writing-a-new-rule) when relevant.
4. **Run the checks** listed in [Testing](#testing).
5. **Open a pull request** with a clear description, linked issue, and screenshots or sample output where useful.

**Rule proposals should include**

- The vulnerable pattern and a real or realistic example
- Why it matters in Soroban specifically
- The proposed detection approach and expected false-positive sources
- The recommended fix

Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

---

## Security

This is a security-oriented tool, so please handle vulnerabilities responsibly.

- **Vulnerabilities in `soroban-lint` itself** (for example, a crafted input that causes a crash or unsafe behavior): please do not open a public issue. Follow the process in [`SECURITY.md`](SECURITY.md).
- **Vulnerabilities you found in someone else's contract using this tool:** follow responsible disclosure and contact the maintainers of that project privately.
- **Missed vulnerabilities (false negatives):** these are welcome as public issues or pull requests when they concern a general pattern rather than a specific unpatched deployed contract.

---

## FAQ

**Does it need the Stellar CLI or network access?**
No. It analyzes source files only.

**Does it compile my contract?**
No. That keeps it fast and usable on code that does not currently build. The trade-off is the lack of type and macro-expansion information.

**How is this different from Clippy?**
Clippy is a general Rust linter. `soroban-lint` encodes Soroban-specific knowledge: contract entrypoints, the authorization model, storage tiers, and TTL behavior. They complement each other, so run both.

**How is this different from fuzzing or formal verification?**
Static analysis finds known bad patterns without running code. Fuzzing explores runtime behavior with generated inputs, and formal verification proves properties. A mature security workflow uses several of these together.

**I got a false positive. What should I do?**
Suppress it inline with a reason, then open an issue with a minimal example so the rule can be improved.

**Can it run on a single file?**
Yes: `soroban-lint check src/lib.rs`.

**Does it support Soroban SDK versions other than the latest?**
Rules target the current SDK API surface. Each rule's documentation notes the SDK behavior it assumes. Version-specific behavior is tracked in the changelog.

---

## Acknowledgements

- The [Stellar Development Foundation](https://stellar.org) and the Soroban team for the platform, SDK, and documentation.
- [Slither](https://github.com/crytic/slither) and [Trail of Bits](https://www.trailofbits.com/) for popularizing practical static analysis for smart contracts.
- The maintainers of [`syn`](https://github.com/dtolnay/syn), [`dylint`](https://github.com/trailofbits/dylint), [`insta`](https://github.com/mitsuhiko/insta), and the broader Rust tooling ecosystem.
- Everyone who files issues, proposes rules, and improves the docs.

---

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

> **Disclaimer:** `soroban-lint` is a developer aid and is provided as-is. It is not a substitute for a professional security audit, and its results do not constitute a guarantee about the safety of any smart contract.
