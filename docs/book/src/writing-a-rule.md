# Writing a New Rule

## Step-by-step

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

4. **Register it** in `crates/lint-rules/src/lib.rs` — add a `registry.register(...)` call.
5. **Document it** in `docs/rules/SOR009.md` with: description, rationale, vulnerable example, safe example, configuration options, known false positives, and references.
6. **Add snapshot tests** in `crates/lint-rules/tests/sor009_tests.rs` using `assert_rule_snapshot!`.
7. **Update the catalog** in the README and in `CHANGELOG.md`.

## Rule quality bar

A rule should have:
- A clear security or correctness rationale
- A low false-positive rate on real contracts (run `bash scripts/corpus.sh` to verify)
- An actionable message that tells the user how to fix the issue
- Tests for both the vulnerable and the safe case

## Rule proposal checklist

Before implementing a new rule, open an issue with:

- The vulnerable pattern and a real or realistic example
- Why it matters in Soroban specifically
- The proposed detection approach and expected false-positive sources
- The recommended fix
