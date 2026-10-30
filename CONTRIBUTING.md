# Contributing to soroban-lint

Thank you for your interest in contributing! Contributions are welcome in all forms: new rules, bug reports, documentation improvements, test fixtures, and more.

Please read this guide before opening a pull request, and review the [Code of Conduct](CODE_OF_CONDUCT.md) — all contributors are expected to follow it.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Setup](#setup)
3. [Running Tests](#running-tests)
4. [Insta Snapshot Workflow](#insta-snapshot-workflow)
5. [Architecture in 10 Minutes](#architecture-in-10-minutes)
6. [Writing a New Rule](#writing-a-new-rule)
7. [Rule Quality Bar](#rule-quality-bar)
8. [Rule Proposal Checklist](#rule-proposal-checklist)
9. [Commit and PR Conventions](#commit-and-pr-conventions)
10. [Finding Help](#finding-help)

---

## Prerequisites

- **Rust (stable)** — 1.75 or newer. Install via [rustup](https://rustup.rs/).
- **cargo** — ships with Rust; no separate installation needed.
- **cargo-nextest** (optional, but recommended for faster test runs):
  ```bash
  cargo install cargo-nextest
  ```
- **cargo-insta** (required for snapshot tests):
  ```bash
  cargo install cargo-insta
  ```

---

## Setup

```bash
# 1. Fork the repository on GitHub, then clone your fork
git clone https://github.com/<your-username>/soroban-lint.git
cd soroban-lint

# 2. Build the workspace
cargo build

# 3. Verify everything compiles
cargo build --workspace

# 4. Run the test suite
cargo test --workspace
```

---

## Running Tests

```bash
# Standard test runner
cargo test --workspace

# Faster, with nextest
cargo nextest run --workspace

# Review and accept snapshot changes
cargo insta review

# Lints and formatting (must pass before a PR is merged)
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

## Insta Snapshot Workflow

`soroban-lint` uses [`insta`](https://github.com/mitsuhiko/insta) for snapshot testing of rule output. When you add a new rule or change an existing rule's output:

1. **Run the tests** — they will fail if no snapshot exists yet, or if the output changed.

2. **Generate or regenerate snapshots automatically:**
   ```bash
   INSTA_UPDATE=always cargo test --workspace
   ```
   This writes the new snapshot files to disk without pausing for review.

3. **Review changes interactively** (recommended before committing):
   ```bash
   cargo insta review
   ```
   This opens a diff for each changed snapshot so you can approve or reject each one.

4. **Commit the `.snap` files** together with the code change that produced them.

> Tip: if you only want to regenerate snapshots for a single rule, filter the test:
> ```bash
> INSTA_UPDATE=always cargo test sor001
> ```

---

## Architecture in 10 Minutes

The workspace is divided into four crates under `crates/`:

```
crates/
├── lint-core/     # Rule trait, Finding/Severity types, AST context, call graph
├── lint-rules/    # One module per rule (sor001.rs, sor002.rs, …)
├── lint-report/   # pretty, JSON, and SARIF formatters
└── lint-cli/      # clap-based binary (soroban-lint)
```

### `lint-core`

The foundation. Defines:

- **`Rule` trait** — the interface every rule must implement (`id`, `name`, `default_severity`, `check`).
- **`Finding` and `Severity` types** — the data a rule emits.
- **`RuleContext`** — carries the parsed AST, the crate's `Cargo.toml` path, configuration, and the per-crate call graph.
- **Call graph** — built with [`petgraph`](https://github.com/petgraph/petgraph); maps each function to the functions it calls. Used by SOR001 and SOR003 to credit auth/TTL calls in private helpers to their callers.

### `lint-rules`

One `pub struct` per rule, each in its own module. Rules implement `lint_core::Rule` and use `syn::visit::Visit` to walk the AST. Register new rules in `crates/lint-rules/src/lib.rs`.

### `lint-report`

Three formatter modules: `pretty` (compiler-style terminal output with source snippets), `json` (machine-readable), and `sarif` (SARIF 2.1.0 for GitHub code scanning). Each formatter receives a slice of `Finding`s and writes to a `dyn Write`.

### `lint-cli`

The binary crate. Uses [`clap`](https://github.com/clap-rs/clap) to parse arguments, drives file discovery via `cargo_metadata` and the `ignore` crate, orchestrates rule execution, passes findings to a formatter, and returns the correct exit code.

For a deeper dive see the [Architecture page](docs/book/src/architecture.md) in the book.

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

6. **Add snapshot tests** (see [Insta Snapshot Workflow](#insta-snapshot-workflow)).

7. **Update the catalog** in `README.md` and in `CHANGELOG.md`.

---

## Rule Quality Bar

A rule is considered ready to merge when it meets all of the following:

- **Clear rationale.** There is a concrete security or correctness reason the pattern is dangerous in Soroban specifically.
- **Low false-positive rate.** The rule should not fire on idiomatic, correct code. Test it against the corpus in `scripts/` before proposing.
- **Actionable message.** The diagnostic tells the user what to do to fix the issue, not just what is wrong.
- **Both fixture types.** At least one `vulnerable/` fixture that triggers the rule, and at least one `safe/` fixture that does not.
- **Snapshot tests pass** (`cargo insta review` with no pending changes).
- **Documented.** A `docs/rules/SORxxx.md` page exists.

---

## Rule Proposal Checklist

When opening a rule proposal issue, include all of the following:

- **The vulnerable pattern** and a real or realistic example.
- **Why it matters in Soroban specifically** — not just why it is bad Rust in general.
- **The proposed detection approach** and expected false-positive sources.
- **The recommended fix** — what should the developer do instead?

Use the [Rule Proposal issue template](.github/ISSUE_TEMPLATE/rule_proposal.md).

---

## Commit and PR Conventions

**Commits** follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(sor009): add my-rule
fix(sor001): avoid false positive on __constructor
docs: update installation instructions
chore: bump syn to 2.0.90
```

Common types: `feat`, `fix`, `docs`, `test`, `chore`, `refactor`, `perf`, `ci`.

**Pull requests**

- **Title:** ≤ 70 characters, conventional-commit style (e.g. `feat(sor009): detect missing custom auth helpers`).
- **Description:** summarize what changed, why, what was tested, and link the related issue with `Closes #NNN`.
- Fill in all items on the [PR template](.github/pull_request_template.md) checklist before requesting review.
- Keep PRs focused. Large PRs are harder to review; consider splitting unrelated changes.

---

## Finding Help

- Browse existing issues — especially `good first issue` and `help wanted` labels.
- Ask questions in issues or discussions — there are no stupid questions.
- For security issues, follow the process in [SECURITY.md](SECURITY.md) instead of opening a public issue.
