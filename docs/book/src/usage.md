# Usage

```text
soroban-lint <COMMAND>

Commands:
  check     Analyze contract source and report findings
  rules     List available rules
  explain   Show detailed documentation for a rule
  init      Create a default soroban-lint.toml
  help      Print help
```

## `check`

```bash
soroban-lint check [PATH ...] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `PATH` | Files or directories to analyze | `.` |
| `--format <FMT>` | `pretty`, `json`, or `sarif` | `pretty` |
| `--output <FILE>` | Write the report to a file instead of stdout | stdout |
| `--config <FILE>` | Path to a config file | `./soroban-lint.toml` |
| `--select <IDS>` | Run only these rules (comma-separated) | all enabled |
| `--ignore <IDS>` | Skip these rules (comma-separated) | none |
| `--min-severity <LVL>` | Report only `note`, `warning`, or `error` and above | `note` |
| `--fail-on <LVL>` | Exit non-zero at or above this severity | `error` |
| `--no-color` | Disable colored output | auto-detected |
| `--include-tests` | Also analyze `#[cfg(test)]` modules and `tests/` | off |
| `--baseline <FILE>` | Ignore findings already recorded in this baseline file | none |
| `--update-baseline` | Write current findings to the `--baseline` file and exit 0 | off |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | No findings at or above the `--fail-on` level |
| `1` | Findings at or above the `--fail-on` level |
| `2` | Usage, configuration, or internal error |

## Baselines

Baselines let you gate CI on *new* findings only, ignoring pre-existing ones.

**Create a baseline** from the current state of the codebase:

```bash
soroban-lint check --baseline baseline.json --update-baseline
```

**Check against it** on every subsequent run:

```bash
soroban-lint check --baseline baseline.json
```

Any finding whose stable fingerprint (rule id + file + enclosing function + normalised message) is present in the baseline is silently dropped before severity filtering and exit-code computation. The pretty summary line reports how many findings were suppressed this way.

A `--baseline` path that does not exist (without `--update-baseline`) is an exit-code 2 error.

## `rules` and `explain`

```bash
soroban-lint rules                 # table of all rules, status, default severity
soroban-lint explain SOR001        # full documentation in the terminal
```

## `init`

```bash
soroban-lint init                  # writes soroban-lint.toml with defaults
```
