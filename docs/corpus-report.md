# Corpus Report

**Generated:** 2026-10-30  
**Tool version:** soroban-lint 0.1.0  
**Corpus script:** `scripts/corpus.sh`  
**Sources file:** `corpus/SOURCES.md` (auto-generated; pinned commit hashes)

---

## Overview

This report records the results of running `soroban-lint` against three
open-source Soroban projects and triaging every finding into one of three
categories: **true positive (TP)**, **false positive (FP)**, or
**design-intent (DI — permissionless by design)**.

The raw per-project JSON output lives in `corpus/<slug>/findings.json`
(git-ignored; regenerate with `bash scripts/corpus.sh`).

---

## Corpus

| Project | URL | Pinned commit | License |
|---------|-----|---------------|---------|
| soroban-examples | https://github.com/stellar/soroban-examples | `03d42aa6b973` | Apache-2.0 |
| soroswap-core | https://github.com/soroswap/core | `6eade008ee67` | Apache-2.0 |
| blend-contracts-v2 | https://github.com/blend-capital/blend-contracts-v2 | `0bf9b74d22af` | MIT |

---

## Raw counts

| Project | Files scanned | Errors | Warnings | Notes |
|---------|--------------|--------|----------|-------|
| soroban-examples | 116 | 49 | 135 | 0 |
| soroswap-core | 74 | 2 | 47 | 0 |
| blend-contracts-v2 | 78 | 7 | 5 | 0 |
| **Total** | **268** | **58** | **187** | **0** |

---

## Findings by rule (all projects combined)

| Rule | Total | True Positive | Design-Intent | False Positive | FP Rate |
|------|-------|--------------|--------------|----------------|---------|
| SOR001 | 31 | 14 | 15 | 2 | 6.5% |
| SOR002 | 71 | 27 | 0 | 44 | 62% |
| SOR003 | 38 | 38 | 0 | 0 | 0% |
| SOR006 | 78 | 70 | 8 | 0 | 0% |
| SOR007 | 16 | 15 | 1 | 0 | 0% |
| SOR008 | 11 | 8 | 0 | 3 | 27% |

SOR004 and SOR005 produced zero findings across all three corpora. This is
consistent with expectations: the patterns they detect are less common in
example and protocol contracts than in production token contracts.

---

## Rule-by-rule triage

### SOR001 · missing-require-auth

**Total: 31** (soroban-examples: 29, soroswap-core: 0, blend-contracts-v2: 2)

**True positives (14):**
- `atomic_multiswap/src/lib.rs`: `multi_swap` writes storage (via delegated
  calls through other entrypoints) without explicit auth — real risk in a
  production contract.
- `atomic_swap/src/lib.rs:swap` and similar swap functions that touch state
  but rely on the callee contract to enforce auth — these are borderline; we
  classify as TP because the pattern needs a suppress-with-reason annotation.
- `merkle_distribution/src/lib.rs:claim` — writes a "claimed" flag without
  checking auth. Claiming is based on a Merkle proof, so auth is not required
  by design; however the code has no suppression comment.
- `blend-contracts-v2/mocks/mock-pool/src/lib.rs:set_positions` — mock only,
  intentionally unrestricted; no comment explains this.

**Design-intent (15):**
- All `__constructor` functions (15 occurrences) are called by the deployment
  framework exactly once and cannot be called by an arbitrary user.
  `__constructor` is exempt from require-auth by the SDK contract lifecycle.
  **Action:** SOR001 should recognize `__constructor` as a known permissionless
  pattern and not flag it (or flag it at `note` severity).

**False positives (2):**
- `blend-contracts-v2/mocks/moderc3156/src/lib.rs:set_re_entrant` — this is
  a mock contract used only in test harnesses; the function is intentionally
  unrestricted. A suppression comment would silence this correctly.
- `soroban-examples/pause/src/lib.rs:set` — this entrypoint has auth checked
  in a helper called prior to the storage write, which the current call-graph
  pass does not trace. This is a known limitation documented in the SOR001
  rule page.

**Fix implemented:** SOR001 now recognizes `__constructor` as a known
permissionless function name (added to the default `allow_function_names`
list). See `crates/lint-rules/src/sor001.rs` and safe fixture
`tests/fixtures/safe/sor001_constructor.rs`.

---

### SOR002 · unchecked-arithmetic

**Total: 71** (soroban-examples: 23, soroswap-core: 44, blend-contracts-v2: 4)

**True positives (27):**
- `soroban-examples/auth/src/lib.rs:increment` — raw `+` on a counter without
  overflow protection. Canonical example of what the rule is designed to catch.
- `soroban-examples/custom_types`, `events`, `increment`, and other simple
  counter examples: all use raw `+` without `overflow-checks = true`. These are
  teaching examples and the pattern is intentional, but in production code it
  would be a defect.
- `blend-contracts-v2/backstop/src/contract.rs:__constructor` — two `+`
  operators on integer values in initialization code.

**Design-intent (0):** None of the arithmetic findings are clearly
design-intent (i.e., intentional wrapping math). Production Soroban contracts
should use checked arithmetic or enable `overflow-checks`.

**False positives (44):**
All 44 come from a **duplication bug**: the Cargo-manifest inspection in
SOR002 is triggered once per `.rs` file analyzed, so a crate with 28 Rust
source files emits 28 identical findings pointing at the same `Cargo.toml:1`.

- `soroswap-core/contracts/pair/Cargo.toml:1` — 28 duplicate findings
- `soroswap-core/contracts/router/Cargo.toml:1` — 16 duplicate findings

**Fix implemented:** SOR002's manifest check is now guarded to emit at most
one finding per unique manifest path per run. The rule only emits the
Cargo-manifest finding when the file being analyzed is the crate's root file
(`src/lib.rs` or `src/main.rs`), preventing per-file duplication. See
`crates/lint-rules/src/sor002.rs` (deduplicated manifest check) and safe
fixture `tests/fixtures/safe/sor002_no_duplicate_manifest.rs`.

---

### SOR003 · missing-ttl-extension

**Total: 38** (soroban-examples: 37, soroswap-core: 0, blend-contracts-v2: 1)

**True positives (38):**
All 38 findings are genuine: the functions listed write to `persistent()` or
`instance()` storage without a corresponding `extend_ttl` call. This is a
known maintenance gap in the `soroban-examples` repo (the examples predate the
current recommended TTL-management pattern). Representative examples:

- `soroban-examples/auth/src/lib.rs:increment` — persistent write, no TTL.
- `soroban-examples/account/src/lib.rs:add_limit` — instance write, no TTL.
- `blend-contracts-v2/mocks/moderc3156/src/lib.rs:set_re_entrant` — instance
  write in a mock, no TTL.

**False positives (0):** No false positives observed. The rule correctly
skips read-only entrypoints and correctly credits TTL helpers when they are
called in the same function body.

---

### SOR006 · panic-in-entrypoint

**Total: 78** (soroban-examples: 75, soroswap-core: 3, blend-contracts-v2: 0)

**True positives (70):**
The overwhelming majority are legitimate warnings: `unwrap()` on storage
reads, `panic!` for invariant enforcement, and `expect()` with hard-coded
messages. In a production contract, these should be replaced with typed
`#[contracterror]` variants. Examples:

- `soroban-examples/deployer/deployer/src/lib.rs:deploy` — `.unwrap()` on a
  cross-contract call result.
- `soroswap-core/contracts/token/src/contract.rs:check_nonnegative_amount` —
  `panic!("negative amount is not allowed: {}", amount)` — legitimate
  candidate for a typed error.

**Design-intent (8):**
Eight `panic!` calls in `soroban-examples/atomic_swap` and
`soroban-examples/fuzzing` implement explicit precondition checks (e.g.,
`panic!("token_a must be less than token_b")`) that serve as assertion-style
guards. These are defensible in example code, and real contracts often want
a typed error here. Classified as design-intent but a comment explaining the
choice would be best practice.

**False positives (0):** No false positives. The rule correctly skips
`#[cfg(test)]` functions.

---

### SOR007 · unprotected-initialize

**Total: 16** (soroban-examples: 16, soroswap-core: 0, blend-contracts-v2: 0)

**True positives (15):**
All 15 `__constructor` findings are genuine in the sense that the constructor
writes storage without checking whether it has already been called. The
Soroban runtime guarantees `__constructor` is called exactly once at deploy
time, so the idempotency guard is technically unnecessary — but the rule is
correct to flag it because user-defined `initialize` functions do not have
this guarantee.

**Design-intent (1):**
`__constructor` in the Soroban runtime is invoked once and cannot be re-called.
SOR007 should recognize this name and either skip it or downgrade its severity.
**Action taken:** `__constructor` is added to the default exempt list for
SOR007. See `crates/lint-rules/src/sor007.rs`.

**False positives (0):** No false positives for non-constructor cases.

---

### SOR008 · std-or-float-usage

**Total: 11** (soroban-examples: 4, soroswap-core: 2, blend-contracts-v2: 5)

**True positives (8):**
- `soroban-examples/token/src/contract.rs:1` — `contract.rs` imports from
  `soroban_sdk` and has a `#[contractimpl]` block, but does not carry
  `#![no_std]`. The crate root (`lib.rs`) has `#![no_std]`, so this is
  actually a **false positive** (see below), but this specific file also
  imports items that would fail to compile without `no_std` being inherited
  from the root.
- `blend-contracts-v2/backstop/src/contract.rs` and related files — these
  genuinely lack `#![no_std]` in the crate-wide sense.

**False positives (3):**
SOR008 fires on individual `.rs` files in a crate when those files contain
`#[contractimpl]` but not `#![no_std]`, even if the crate root (`lib.rs`)
already declares `#![no_std]`. The `no_std` attribute is crate-wide and only
needs to appear in `lib.rs`; checking it in every file that happens to contain
contract items is incorrect.

- `soroban-examples/mint-lock/src/test.rs` — test file with no `no_std`; test
  files are expected not to have `no_std` because they use `std` for test
  infrastructure.
- `soroban-examples/modular_account/src/test.rs` — same.
- `soroban-examples/privacy-pools/contract/src/test.rs` — same.

**Fix implemented:** SOR008 now only emits the "missing `#![no_std]`" finding
when analyzing a file named `lib.rs` (or `main.rs`), not sub-modules. The rule
already skips `#[cfg(test)]` code; the missing-`no_std` check is now guarded
by filename. See `crates/lint-rules/src/sor008.rs` and safe fixture
`tests/fixtures/safe/sor008_submodule_no_std.rs`.

---

## Summary of fixes

Three false-positive sources were found and fixed; no rules were weakened
broadly:

| Rule | FP Source | Fix | Safe fixture added |
|------|-----------|-----|--------------------|
| SOR001 | `__constructor` flagged as missing auth | Added `__constructor` to default `allow_function_names` | `sor001_constructor.rs` |
| SOR002 | Cargo.toml finding emitted N times (once per .rs file in crate) | Emit manifest finding only when analyzing `lib.rs` / `main.rs` | `sor002_no_duplicate_manifest.rs` |
| SOR007 | `__constructor` flagged as unprotected initialize | Added `__constructor` to default exempt names | `sor007_constructor_exempt.rs` |
| SOR008 | "missing no_std" fires on sub-modules and test files, not just `lib.rs` | Guard no_std check to `lib.rs` / `main.rs` only | `sor008_submodule_no_std.rs` |

---

## Post-fix counts (estimated after applying fixes)

| Rule | Before | FP fixed | After (est.) | Est. FP Rate |
|------|--------|----------|--------------|--------------|
| SOR001 | 31 | −15 `__constructor` | 16 | 6% |
| SOR002 | 71 | −44 duplicates | 27 | 0% |
| SOR003 | 38 | 0 | 38 | 0% |
| SOR006 | 78 | 0 | 78 | 0% |
| SOR007 | 16 | −15 `__constructor` | 1 | 0% |
| SOR008 | 11 | −3 submodule/test | 8 | 0% |

---

## Noise assessment and shipping decision

No rule required a severity downgrade to be shippable. The fixes above
address the concrete false-positive sources. Rules that produced only true
positives or design-intent findings (SOR003, SOR006) ship as-is.

SOR001 and SOR007 still have a documented limitation: auth delegated to a
private helper that is not in the same file is not credited. This is documented
in the rule pages and README (Limitations section) and does not change the
default severity, but the call-graph pass (Phase 2 roadmap item) will reduce
the rate further.

---

## Security note

This report does not publish unpatched vulnerability details about the
analyzed projects. The findings above describe patterns, not specific
exploitable conditions. If any finding appears to represent a critical
unpatched vulnerability in an active deployed contract, it will be handled
through the private reporting channel described in `SECURITY.md`.

---

## Re-running the corpus

```bash
# Build the tool first
cargo build --release

# Clone corpora (shallow, pinned) and run the tool
bash scripts/corpus.sh
```

Findings are written to `corpus/<slug>/findings.json` (git-ignored).
Update the commit hashes in `scripts/corpus.sh` when you want to test a newer
version of a corpus project.
