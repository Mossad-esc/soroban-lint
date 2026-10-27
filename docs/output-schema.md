# JSON Output Schema

`soroban-lint check --format json` emits a single JSON object to stdout (or to
`--output <FILE>`).

## Top-level fields

| Field | Type | Description |
|---|---|---|
| `tool` | string | Always `"soroban-lint"` |
| `version` | string | The tool version (e.g. `"0.1.0"`) |
| `summary` | object | Aggregate counts — see below |
| `findings` | array | Zero or more finding objects — see below |

## `summary` object

| Field | Type | Description |
|---|---|---|
| `errors` | integer | Number of findings at `error` severity |
| `warnings` | integer | Number of findings at `warning` severity |
| `notes` | integer | Number of findings at `note` severity |
| `files_scanned` | integer | Total files analyzed (including files with parse errors) |

## `findings` array

Each element is a finding object:

| Field | Type | Description |
|---|---|---|
| `rule` | string | Rule identifier (e.g. `"SOR001"`) |
| `name` | string | Kebab-case rule name (e.g. `"missing-require-auth"`) |
| `severity` | string | `"error"`, `"warning"`, or `"note"` |
| `message` | string | Human-readable description of the problem |
| `file` | string | Relative file path, forward slashes on all OSes |
| `line` | integer | 1-based line number of the primary span |
| `column` | integer | 1-based column number of the primary span |
| `help` | string | *(optional)* Actionable suggestion |

## Example

```json
{
  "tool": "soroban-lint",
  "version": "0.1.0",
  "summary": {
    "errors": 1,
    "warnings": 1,
    "notes": 0,
    "files_scanned": 2
  },
  "findings": [
    {
      "rule": "SOR001",
      "name": "missing-require-auth",
      "severity": "error",
      "message": "entrypoint `transfer` writes to storage but never calls `require_auth`",
      "file": "contracts/token/src/lib.rs",
      "line": 42,
      "column": 12,
      "help": "call `from.require_auth()` before mutating state"
    },
    {
      "rule": "SOR003",
      "name": "missing-ttl-extension",
      "severity": "warning",
      "message": "entrypoint `transfer` writes to `persistent()` storage but never calls `extend_ttl` on that tier",
      "file": "contracts/token/src/lib.rs",
      "line": 42,
      "column": 12,
      "help": "add `env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO)` after the write to keep the entry alive"
    }
  ]
}
```

## Notes

- Findings are sorted by `(file, line, column, rule)` for deterministic output.
- The `help` field is omitted when no help text is available.
- File paths always use forward slashes regardless of the host OS.
