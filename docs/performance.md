# soroban-lint Performance

This document records benchmark results for the parallel file analysis
introduced in Commit 8. Results are from `scripts/bench.sh`.

## Methodology

- `scripts/bench.sh N` replicates every file in `tests/fixtures/` N times
  into a temporary directory and times three `soroban-lint check` runs,
  reporting the median.
- The binary is built with `cargo build --release`.
- `RAYON_NUM_THREADS` is unset (rayon uses all available cores).
- The "warm-up" run is discarded.

## Benchmark run 2026-10-28

| Metric | Value |
|--------|-------|
| Files analyzed | 1800 |
| Fixture replications (N) | 50 |
| Median wall time | 568 ms |
| Binary | soroban-lint (release) |
| CPU | Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz |
| Run times | 573 ms, 562 ms, 568 ms |

