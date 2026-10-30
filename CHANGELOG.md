# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

_No unreleased changes since v0.1.0-rc.1._

---

## [0.1.0-rc.1] — 2026-10-30

First release candidate. All eight rules are implemented and tested. The
crate names `soroban-lint`, `lint-core`, `lint-rules`, `lint-report`, and
`lint-cli` are available on crates.io and reserved for this project.

**Do not publish to crates.io until I confirm the release candidate.**
Tag only after review: `git tag v0.1.0-rc.1 && git push --tags`.

### Added

- **SOR001 `missing-require-auth`** — detects public entrypoints in `#[contractimpl]` blocks that write to storage without calling `require_auth()` or `require_auth_for_args()`.
- **SOR002 `unchecked-arithmetic`** — detects raw arithmetic operators on integer values inside entrypoints and checks the Cargo.toml release profile for `overflow-checks = true`.
- **SOR003 `missing-ttl-extension`** — detects writes to `persistent()` or `instance()` storage without a corresponding `extend_ttl` call.
- **SOR004 `critical-data-in-temporary-storage`** — detects keys whose names suggest critical state (admin, owner, balance, allowance, config, supply) written to `temporary()` storage.
- **SOR005 `unbounded-storage-growth`** — detects `Vec` or `Map` values loaded from a fixed storage key, appended to, and written back without a visible length bound.
- **SOR006 `panic-in-entrypoint`** — detects `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, and unchecked indexing in public entrypoints.
- **SOR007 `unprotected-initialize`** — detects initializer functions that neither check an "already initialized" flag nor require authorization.
- **SOR008 `std-or-float-usage`** — detects missing `#![no_std]`, `std::` path usage, and `f32`/`f64` types and literals in contract crates.
- **Pretty terminal reporter** — compiler-style diagnostics with source snippets, help text, and documentation links.
- **JSON reporter** — machine-readable output (`--format json`) with a structured findings array and summary object.
- **SARIF 2.1.0 reporter** — (`--format sarif`) for GitHub code scanning, VS Code SARIF viewers, and other security dashboards.
- **`soroban-lint.toml` configuration** — per-rule enable/disable, severity overrides, and rule-specific options (critical key patterns, init function names, etc.). Generated via `soroban-lint init`.
- **Inline suppression comments** — `// soroban-lint: allow(SORxxx) reason="..."` syntax to suppress individual findings with a required justification.
- **Baseline files** — `--baseline` and `--update-baseline` flags to gate CI on new findings only, ignoring pre-existing ones.
- **Reusable GitHub Action** (`action.yml`) for running soroban-lint in CI with configurable path, format, output, and fail-on options.
- **Workspace-aware discovery** — uses `cargo_metadata` to find workspace members; crates depending on `soroban-sdk` are treated as contract crates. Respects `.gitignore` via the `ignore` crate.
- **Parallel scanning with rayon** — files are analyzed concurrently for faster runs on large workspaces.
- **Intra-crate call graph** — built with `petgraph`; credits auth or TTL calls made in private helper functions to their public callers (SOR001, SOR003).
- **Corpus testing script** (`scripts/corpus.sh`) — shallow-clones three open-source Soroban projects (stellar/soroban-examples, soroswap/core, blend-capital/blend-contracts-v2) and runs soroban-lint against them; produces `corpus/<slug>/findings.json` and `corpus/SOURCES.md`.
- **Corpus triage report** (`docs/corpus-report.md`) — triages all 245 findings across 268 files into true positives, false positives, and design-intent, with per-rule counts and analysis.
- **Panic hook** — catches unexpected panics in rule visitors and converts them to exit code 2 with a "please file an issue" message, preventing silent failures.
- **Edge-case fixture hardening** — empty file, comments-only, Windows CRLF, syntax errors, deep nesting, macro-heavy code, unusual attributes, unreadable files, non-UTF-8 bytes, huge files. All must exit 0, 1, or 2; never panic.
- **Contributor foundation** — `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CODEOWNERS`, issue templates (bug, false positive, false negative, rule proposal), PR template, labels definition (`.github/labels.yml`).
- **mdBook documentation** (`docs/book/`) — Introduction, Installation, Usage, Configuration, Suppression, Output Formats, CI/CD Integration, per-rule pages, Architecture, Writing a New Rule, Limitations.
- **GitHub Pages workflow** (`.github/workflows/pages.yml`) — builds and deploys the mdBook on every push to `main` that touches `docs/`.
- **Release pipeline** — `.github/workflows/release.yml` builds prebuilt binaries for Linux (glibc + musl), macOS (x86_64 + aarch64), and Windows on version tags; `[workspace.metadata.dist]` in `Cargo.toml` configures cargo-dist 0.32.0.

### Fixed

- **SOR001** — false positive on `__constructor`-style setup functions (15 occurrences in soroban-examples corpus). The Soroban runtime calls `__constructor` exactly once at deploy time; no auth is possible or required.
- **SOR002** — duplicate findings: the Cargo.toml manifest check was emitting one finding per `.rs` file in a crate instead of once per crate root. Fixed by guarding the check to `lib.rs`/`main.rs` only. (Reduced 44 duplicate findings in soroswap-core to 2.)
- **SOR007** — false positive on `__constructor`-style functions (same corpus source as SOR001 fix).
- **SOR008** — false positive when `#![no_std]` is declared in the crate root (`lib.rs`) but the rule fired on sub-modules and test files that don't redeclare it. The missing-`no_std` finding is now only emitted from `lib.rs` or `main.rs`.

---

<!-- Link definitions -->
[Unreleased]: https://github.com/Mossad-esc/soroban-lint/compare/v0.1.0-rc.1...HEAD
[0.1.0-rc.1]: https://github.com/Mossad-esc/soroban-lint/releases/tag/v0.1.0-rc.1
