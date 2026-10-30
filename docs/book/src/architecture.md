# Architecture

## Pipeline overview

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

## Crates

| Crate | Path | Responsibility |
|---|---|---|
| `lint-core` | `crates/lint-core` | Rule trait, `Finding`/`Severity` types, `ContractModel`, call graph, discovery, config, suppression, baseline |
| `lint-rules` | `crates/lint-rules` | One module per rule (`sor001.rs` … `sor008.rs`) |
| `lint-report` | `crates/lint-report` | Pretty, JSON, and SARIF formatters |
| `lint-cli` | `crates/lint-cli` | `clap`-based binary (`soroban-lint`) |

## Analysis styles used by the rules

| Style | Example rules | Approach |
|---|---|---|
| AST pattern matching | SOR006, SOR008 | Find specific calls, macros, or types |
| Co-occurrence ("A should accompany B") | SOR001, SOR003 | Detect a storage write without a paired auth or TTL call |
| Manifest inspection | SOR002 (profile check) | Parse `Cargo.toml` |
| Naming/heuristic | SOR004, SOR007 | Match key and function names against configurable patterns |
| Dataflow-lite (planned) | SOR005 | Track a collection loaded, grown, and stored back |
