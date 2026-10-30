# Installation

## Requirements

- Rust (stable, 1.75 or newer recommended) if building from source
- A Soroban contract project to analyze. The linter reads source files, so it does not need the Stellar CLI or a network connection.

## From crates.io

```bash
cargo install soroban-lint
```

## From source

```bash
git clone https://github.com/Mossad-esc/soroban-lint.git
cd soroban-lint
cargo build --release
./target/release/soroban-lint --version
```

## Prebuilt binaries

Download binaries for Linux, macOS, and Windows from the [Releases](https://github.com/Mossad-esc/soroban-lint/releases) page.

## As a cargo subcommand

If the binary is named `cargo-soroban-lint`, you can also run:

```bash
cargo soroban-lint check
```
