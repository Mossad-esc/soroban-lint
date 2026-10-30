## Summary

<!-- Describe what this PR changes and why. -->

## Related Issue

Closes #

## Checklist

- [ ] Added/updated fixtures (vulnerable AND safe) for any changed rule
- [ ] Ran `cargo insta review` and accepted all snapshots
- [ ] Rule documentation updated in `docs/rules/`
- [ ] CHANGELOG.md updated under [Unreleased]
- [ ] README.md only changed when the change is intentional (new rule, new CLI flag, etc.)
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes

## Testing

<!-- Describe how you tested this change. Reference specific test files or commands. -->

## Notes for Reviewers

<!-- Anything the reviewer should pay special attention to, design tradeoffs made, or known follow-up work. -->
