#!/usr/bin/env bash
# scripts/bench.sh — benchmark soroban-lint on a replicated fixture corpus.
#
# Usage:
#   bash scripts/bench.sh [N]
#
# N = number of times to replicate the fixture set (default: 50).
# Results are appended to docs/performance.md.
#
# Requirements: the soroban-lint binary must be in PATH or built in
# target/release (the script tries release first, then debug).

set -euo pipefail

N=${1:-50}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="${SCRIPT_DIR}/.."
FIXTURES="${WORKSPACE}/tests/fixtures"
BINARY=""

if [[ -x "${WORKSPACE}/target/release/soroban-lint" ]]; then
    BINARY="${WORKSPACE}/target/release/soroban-lint"
elif [[ -x "${WORKSPACE}/target/debug/soroban-lint" ]]; then
    BINARY="${WORKSPACE}/target/debug/soroban-lint"
else
    echo "error: soroban-lint binary not found. Run 'cargo build --release' first." >&2
    exit 1
fi

echo "Binary : $BINARY"
echo "N      : $N"

# Create a temporary directory populated with N copies of every fixture.
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

COUNT=0
for f in "${FIXTURES}"/vulnerable/*.rs "${FIXTURES}"/safe/*.rs; do
    [[ -f "$f" ]] || continue
    for i in $(seq 1 "$N"); do
        DEST="${TMPDIR}/$(basename "$f" .rs)_${i}.rs"
        cp "$f" "$DEST"
        COUNT=$((COUNT + 1))
    done
done

echo "Files  : $COUNT"
echo ""

# Warm-up run (not timed).
"$BINARY" check "$TMPDIR" --format json > /dev/null 2>&1 || true

# Timed run (3 iterations, take the median).
TIMES=()
for _i in 1 2 3; do
    START=$(date +%s%N)
    "$BINARY" check "$TMPDIR" --format json > /dev/null 2>&1 || true
    END=$(date +%s%N)
    MS=$(( (END - START) / 1000000 ))
    TIMES+=("$MS")
done

# Sort and pick median.
IFS=$'\n' SORTED=($(sort -n <<<"${TIMES[*]}")); unset IFS
MEDIAN="${SORTED[1]}"

echo "Run 1: ${TIMES[0]} ms"
echo "Run 2: ${TIMES[1]} ms"
echo "Run 3: ${TIMES[2]} ms"
echo "Median: ${MEDIAN} ms"

PERF_DOC="${WORKSPACE}/docs/performance.md"
DATE=$(date -u +"%Y-%m-%d")
CPUINFO=$(grep -m1 "model name" /proc/cpuinfo 2>/dev/null | sed 's/.*: //' || echo "unknown")

cat >> "$PERF_DOC" << EOF

## Benchmark run ${DATE}

| Metric | Value |
|--------|-------|
| Files analyzed | ${COUNT} |
| Fixture replications (N) | ${N} |
| Median wall time | ${MEDIAN} ms |
| Binary | $(basename "$BINARY") |
| CPU | ${CPUINFO} |
| Run times | ${TIMES[0]} ms, ${TIMES[1]} ms, ${TIMES[2]} ms |

EOF

echo ""
echo "Results appended to docs/performance.md"
