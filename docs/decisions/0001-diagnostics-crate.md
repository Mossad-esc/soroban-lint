# 0001 — Diagnostics crate choice: `annotate-snippets`

**Date:** 2026-10-26  
**Status:** Accepted

---

## Context

The `pretty` formatter in `lint-report` needs to render compiler-style
diagnostics: a labelled severity+code header, a `-->` file/line/col pointer,
a source-snippet with the flagged identifier underlined, and `= help:` /
`= docs:` annotation lines.  Two popular crates provide this:

| Crate | Approach | Stability | Notable users |
|---|---|---|---|
| **`miette`** | Diagnostic *derive* + trait objects; ships its own renderer | Stable | many Rust CLI tools |
| **`ariadne`** | Builder API producing ANSI-coloured snippets | Stable | niche, fewer deps |
| **`annotate-snippets`** | Low-level snippet renderer, no derive magic | Stable | rustc itself |

A fourth option is **no external crate**: hand-roll the renderer with `std`
and ANSI escape codes.

---

## Decision

Use **`annotate-snippets`** (the crate that backs rustc's own diagnostic
output), with a thin hand-rolled wrapper that:

1. Builds an `annotate_snippets::Snippet` for each finding.
2. Passes it through `annotate_snippets::Renderer` with colour enabled/disabled
   at call time.
3. Renders to a `String` so the output can be snapshot-tested without touching
   stdout.

---

## Rationale

### Why not `miette`?

`miette` is excellent but it is designed around Rust's `std::error::Error` and
`thiserror` derive macros.  `Finding` is not an error; it is a value type.
Fitting `Finding` into `miette`'s `Diagnostic` trait would require either
wrapping each finding in a newtype or implementing a custom renderer, negating
the convenience.  `miette` also pulls in `unicode-width`, `supports-color`,
`supports-unicode`, `owo-colors`, and several other transitive dependencies
that we don't otherwise need.

### Why not `ariadne`?

`ariadne` has a nice API but it requires providing byte-range spans into the
source.  `syn` gives us line/column, not byte offsets.  Converting
line/column → byte offset requires a pass over the source text.  This is
doable, but adds complexity for no gain over `annotate-snippets`.

### Why `annotate-snippets`?

- Ships as a dependency of `rustc` (battle-tested, well-maintained).
- Accepts line/column spans directly (matches what `syn` gives us).
- Pure renderer: no derive macros, no trait objects, just build a data
  structure and call `render()`.
- Produces output that is visually identical to rustc's own errors, which is
  the style the README sample output shows.
- Small dependency footprint.

### Why not hand-roll?

Hand-rolling ANSI rendering is error-prone (column counting, multi-byte
characters, `NO_COLOR`/`--no-color` interaction) and would need its own tests.
`annotate-snippets` gives all of that for free.

---

## Consequences

- `lint-report/Cargo.toml` gains a dependency on `annotate-snippets`.
- The `Reporter` trait returns a `String` so callers control where it is
  written; snapshot tests inject a deterministic duration.
- Color detection (`NO_COLOR`, `--no-color`, TTY check) lives in `lint-cli`,
  which passes a `bool use_color` to the reporter.
- JSON and SARIF reporters (Phase 2) implement the same `Reporter` trait and
  do not use `annotate-snippets`.
