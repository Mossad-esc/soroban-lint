# CI/CD Integration

## GitHub Actions (using the reusable action)

```yaml
name: soroban-lint
on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read
  security-events: write   # required to upload SARIF

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run soroban-lint
        uses: your-org/soroban-lint@v0
        with:
          path: .
          format: sarif
          output: soroban-lint.sarif
          fail-on: error

      - name: Upload SARIF to GitHub code scanning
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: soroban-lint.sarif
```

## GitHub Actions (install via cargo)

```yaml
- uses: dtolnay/rust-toolchain@stable
- run: cargo install soroban-lint --locked
- run: soroban-lint check --fail-on warning
```

## Pre-commit hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: soroban-lint
        name: soroban-lint
        entry: soroban-lint check
        language: system
        pass_filenames: false
        files: \.rs$
```

## Recommended rollout for existing projects

1. Run `soroban-lint check` locally and review the findings.
2. Fix real issues, suppress justified exceptions, and commit a `soroban-lint.toml`.
3. Add the CI job with `fail-on = "error"`.
4. Tighten to `warning` once the codebase is clean.
