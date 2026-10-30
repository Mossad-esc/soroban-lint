# Configuration

`soroban-lint` looks for `soroban-lint.toml` in the current directory and walks up to the workspace root. Generate a starter file with `soroban-lint init`.

```toml
# soroban-lint.toml

[general]
# Paths to skip (glob patterns, relative to this file)
exclude = ["target/**", "**/tests/**", "**/test_snapshots/**"]
# Analyze #[cfg(test)] code
include_tests = false
# Exit non-zero at or above this severity: "note" | "warning" | "error"
fail_on = "error"

[rules.SOR001]
enabled = true
severity = "error"

[rules.SOR002]
enabled = true
severity = "warning"
# Also verify overflow-checks in Cargo.toml release profile
check_cargo_profile = true

[rules.SOR003]
enabled = true
severity = "warning"

[rules.SOR004]
enabled = true
severity = "error"
# Key names treated as critical (case-insensitive substring match)
critical_keys = ["admin", "owner", "balance", "allowance", "config", "supply"]

[rules.SOR005]
enabled = true
severity = "warning"

[rules.SOR006]
enabled = true
severity = "warning"

[rules.SOR007]
enabled = true
severity = "error"
# Function names considered initializers
init_names = ["initialize", "init", "setup"]

[rules.SOR008]
enabled = true
severity = "error"

# Functions that are intentionally permissionless and should not trigger SOR001
[rules.SOR001.allow_functions]
names = ["bump_ttl", "get_price"]
```

## Configuration precedence

| Priority | Source |
|---|---|
| 1 (highest) | Inline suppression comments |
| 2 | CLI flags (`--select`, `--ignore`, `--fail-on`, `--min-severity`) |
| 3 | `soroban-lint.toml` |
| 4 | Built-in defaults |
