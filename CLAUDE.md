# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Rotary is a Rust-based CLI and dashboard that audits secret health across existing vaults. It reads only metadata (key names, rotation dates, owners) — never secret values. The CLI is open-source; the backend/dashboard are closed-source.

## Build Commands

```bash
cargo check                          # Fast type-check across workspace
cargo build                          # Build all crates
cargo test --workspace               # Run all tests
cargo test -p rotary-core            # Test a single crate
cargo test -p rotary-scanner -- scan_stale  # Run a specific test
cargo clippy --workspace             # Lint

# CLI usage
cargo run -p rotary-cli -- init                                                  # Create rotary.toml
cargo run -p rotary-cli -- scan                                                  # Scan all sources from rotary.toml
cargo run -p rotary-cli -- scan --source dotenv --path .env --env production     # Ad-hoc single source
cargo run -p rotary-cli -- scan --json                                           # JSON output
cargo run -p rotary-cli -- details STRIPE_SECRET_KEY                             # Key details + playbook
cargo run -p rotary-cli -- details STRIPE_SECRET_KEY --source dotenv --path .env # Ad-hoc details
```

The CLI binary is named `rotary` (defined in `rotary-cli/Cargo.toml`).

## Architecture

This is the **public open-source repo** (`rotaryoss/rotary`). The closed-source backend and dashboard live in a separate private repo (`rotaryoss/rotary-cloud`).

Cargo workspace with four crates under `crates/`:

- **rotary-core** — The `SecretSource` trait, shared types (`SecretMetadata`, `AuditEntry`), health check types (`HealthCheck`, `HealthReport`, `HealthScore`), and error types. Every other crate depends on this. The `SecretSource` trait is the central abstraction — connectors implement it, the scanner consumes it, and the trait boundary is what enforces that Rotary never touches secret values.

- **rotary-connectors** — Implementations of `SecretSource` for each vault. Currently: `DotEnvSource` (parses `.env` files, derives rotation timestamps from filesystem metadata). Stubs planned for Doppler and AWS Secrets Manager. Each connector lives in `src/connectors/<name>.rs`.

- **rotary-scanner** — Takes any `&dyn SecretSource`, runs three health check rules (rotation age, missing owner, unreferenced in codebase), computes a 0–100 health score. The `usage` module walks the project tree using the `ignore` crate (respects `.gitignore`, skips binary files) to find keys not referenced anywhere. Rules are configured via `ScanConfig` (default: 90-day max age, 75-day warning threshold). The score formula penalizes Critical findings at 1.0 and Warnings at 0.5, normalized over total secret count.

- **rotary-core** also contains `RotaryConfig` — the `rotary.toml` schema. Config is found by walking up from the current directory. Relative paths in the config resolve against the config file's directory.

- **rotary-cli** — The user-facing `rotary` binary. Uses `clap` for arg parsing, `colored` for terminal output. Subcommands live in `src/commands/`. Commands: `scan`, `details <KEY>`, `init`. Both `scan` and `details` support `--source` for ad-hoc mode or read from `rotary.toml`. Shared source construction logic is in `commands/sources.rs`.

Non-Rust:
- `playbooks/` — TOML files describing step-by-step rotation procedures for specific secret types. Each playbook has a `patterns` field (case-insensitive substrings) that matches against secret key names. The `details` command walks up from the current directory to find `playbooks/` and matches automatically.

## Config (`rotary.toml`)

The `scan` command reads from `rotary.toml` when run without `--source`. The config walks up parent directories (like `.gitignore`). Key fields:
- `[scan]` — `max_age_days`, `warning_threshold_days`, `project_root` (for unused-secret detection)
- `[[sources]]` — each entry has `name`, `type`, `path`, `environment`, plus arbitrary connector-specific keys via `#[serde(flatten)]` into a `settings: HashMap<String, String>`. For example a Doppler source has `token`, `project`, `config` as extra keys on the same TOML table.

`rotary init` generates a commented starter file. `rotary scan --source dotenv --path .env` bypasses the config entirely for ad-hoc use.

## Adding a New Connector

1. Create `crates/rotary-connectors/src/connectors/<name>.rs`
2. Implement `SecretSource` for your struct (the trait is in `rotary-core`)
3. Add `pub mod <name>;` to `connectors/mod.rs`
4. Re-export from `lib.rs`
5. Add a match arm in `crates/rotary-cli/src/commands/sources.rs` — the `build_source` function receives the full `SourceEntry`, so connector-specific settings are available via `entry.settings` (a `HashMap<String, String>`)

## Key Design Decisions

- **Metadata-only**: The `SecretSource` trait returns `SecretMetadata` (key name, dates, owner) — never the secret value. This is both an architectural constraint and a trust/marketing feature.
- **`async_trait`**: All source methods are async (via `async-trait` crate) to support HTTP-based connectors like Doppler/AWS.
- **Severity ordering**: `Severity` derives `Ord` with `Ok < Warning < Critical`, so `max()` gives the worst finding. The scanner sorts results worst-first.
- **Health score**: `HealthScore::compute()` — Critical = 1.0 deduction, Warning = 0.5 deduction, normalized over count. Score of 100 means all secrets are healthy.
