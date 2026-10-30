# Output Formats

## `pretty` (default)

Compiler-style diagnostics with source snippets, help text, and documentation links.

## `json`

```bash
soroban-lint check --format json
```

```json
{
  "tool": "soroban-lint",
  "version": "0.1.0",
  "summary": { "errors": 1, "warnings": 1, "notes": 0, "files_scanned": 2 },
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
    }
  ]
}
```

## `sarif`

SARIF 2.1.0, suitable for GitHub code scanning, VS Code SARIF viewers, and other security dashboards.

```bash
soroban-lint check --format sarif --output soroban-lint.sarif
```

The full JSON schema is documented in [`docs/output-schema.md`](../../output-schema.md).
