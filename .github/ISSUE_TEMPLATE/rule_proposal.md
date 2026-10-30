---
name: Rule proposal
about: Propose a new soroban-lint rule
title: "rule proposal: SORxxx — "
labels: new rule
assignees: ""
---

## Rule Proposal

### The vulnerable pattern and a real or realistic example

<!-- Describe the code pattern that should be flagged.
     Include a concrete Rust snippet showing the vulnerable or risky code. -->

```rust
// paste a vulnerable or risky example here
```

### Why it matters in Soroban specifically

<!-- Explain the security or correctness impact in the context of Soroban contracts specifically.
     Why is this pattern more dangerous or meaningful in Soroban than in general Rust? 
     Reference Soroban-specific concepts (authorization model, storage tiers, TTL, WASM constraints, etc.) where relevant. -->

### The proposed detection approach and expected false-positive sources

<!-- Describe how the rule would detect the pattern:
     - What AST nodes or patterns would it look for?
     - Would it require a call graph, naming heuristics, or manifest inspection?
     - What are the likely sources of false positives and how could they be mitigated? -->

### The recommended fix

<!-- What should a developer do instead of the flagged pattern?
     Include a corrected code snippet if possible. -->

```rust
// paste the safe/corrected version here
```

---

## Additional Information

<!-- Optional: links to Soroban documentation, SDK changelog entries, audit reports, or related issues that support the proposal. -->

**Proposed rule ID (if you have a suggestion):** SOR  
**Proposed severity:** <!-- error / warning / note -->
