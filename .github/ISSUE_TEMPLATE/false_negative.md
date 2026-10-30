---
name: False negative (missed finding)
about: soroban-lint did NOT report something it should have caught
title: "false negative: SORxxx — "
labels: false negative
assignees: ""
---

## Rule ID

<!-- Which rule should have fired? e.g. SOR001. If you think a new rule is needed instead, use the Rule Proposal template. -->

## Minimal Reproducing Example

<!-- Paste the smallest Rust snippet that demonstrates the missed finding.
     This should be a synthesized, standalone example — do NOT include code from an unpatched deployed contract. -->

```rust
// paste minimal vulnerable contract code here
```

**Command used:**

```bash
soroban-lint check <path> [options]
```

**Full output (showing no finding was reported):**

```
(paste the soroban-lint output here)
```

## Why This Should Be Flagged

<!-- Explain why this code represents a real security or correctness issue in Soroban,
     and why soroban-lint should detect it under the stated rule. -->

## Environment

- soroban-lint version: <!-- soroban-lint --version -->
- Rust toolchain: <!-- rustc --version -->
- Operating system: <!-- e.g. Ubuntu 22.04 -->
- soroban-sdk version (if relevant):

## Additional Context

<!-- Any other context: related SDK documentation, known CVEs, etc. -->
