---
name: False positive
about: soroban-lint reported a finding that is not a real issue
title: "false positive: SORxxx — "
labels: false positive
assignees: ""
---

## Rule ID

<!-- e.g. SOR001, SOR003 -->

## Minimal Reproducing Example

<!-- Paste the smallest Rust snippet that triggers the false positive. Keep it self-contained. -->

```rust
// paste minimal contract code here
```

**Command used:**

```bash
soroban-lint check <path> [options]
```

**Full output:**

```
(paste the soroban-lint output here)
```

## Why This Is a False Positive

<!-- Explain why the flagged code is actually correct and should not be reported. 
     Include any relevant context about the design or intent of the code. -->

## Environment

- soroban-lint version: <!-- soroban-lint --version -->
- Rust toolchain: <!-- rustc --version -->
- Operating system: <!-- e.g. Ubuntu 22.04 -->
- soroban-sdk version (if relevant): 

## Suggested Fix

<!-- Optional: if you have an idea for how the rule could be improved to avoid this false positive, describe it here. -->
