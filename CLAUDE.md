# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Rotary is a Rust-based CLI and dashboard that audits secret health across existing vaults. It reads only metadata (key names, rotation dates, owners) — never secret values. The CLI is open-source; the backend/dashboard are closed-source.

## Build Commands

```bash
cargo check                          # Fast type-check across workspace
cargo build                          # Build all crates
cargo test --workspace               # Run all tests
cargo test -p rotaryoss-core         # Test a single crate
cargo test -p rotaryoss-scanner -- scan_stale  # Run a specific test
cargo clippy --workspace             # Lint
cargo fmt --all                      # Format

# CLI usage
cargo run -p rotaryoss-cli -- init                                                  # Create rotary.toml
cargo run -p rotaryoss-cli -- scan                                                  # Scan all sources from rotary.toml
cargo run -p rotaryoss-cli -- scan --source dotenv --path .env --env production     # Ad-hoc single source
cargo run -p rotaryoss-cli -- scan --json                                           # JSON output
cargo run -p rotaryoss-cli -- details STRIPE_SECRET_KEY                             # Key details + playbook
cargo run -p rotaryoss-cli -- check --threshold 70                                  # CI health gate
```

The CLI binary is named `rotary` (defined in `rotary-cli/Cargo.toml`). Crate names use the `rotaryoss-*` prefix on crates.io.

## Architecture

This is the **public open-source repo** (`rotaryoss/rotary`). The closed-source backend and dashboard live in a separate private repo (`rotaryoss/rotary-cloud`).

Cargo workspace with four crates under `crates/`:

- **rotary-core** — The `SecretSource` trait, shared types (`SecretMetadata`, `AuditEntry`), health check types (`HealthCheck`, `HealthReport`, `HealthScore`), error types, `OwnersConfig` (`.rotary-owners.toml` pattern-based owner mapping), and `RotaryConfig` (`rotary.toml` schema). Every other crate depends on this. The `SecretSource` trait is the central abstraction — connectors implement it, the scanner consumes it, and the trait boundary is what enforces that Rotary never touches secret values.

- **rotary-connectors** — Implementations of `SecretSource` for each vault. Currently: `DotEnvSource` (parses `.env` files, derives rotation timestamps from filesystem metadata), `DopplerSource` (Doppler REST API via `reqwest`). Each connector lives in `src/connectors/<name>.rs`.

- **rotary-scanner** — Takes any `&dyn SecretSource`, runs three health check rules (rotation age, missing owner, unreferenced in codebase), computes a 0–100 health score. Supports an optional `OwnersConfig` via `scanner.with_owners()` to resolve missing owners before checks run. The `usage` module walks the project tree using the `ignore` crate (respects `.gitignore`, skips binary files) to find keys not referenced anywhere. Rules are configured via `ScanConfig` (default: 90-day max age, 75-day warning threshold).

- **rotary-cli** — The user-facing `rotary` binary. Uses `clap` for arg parsing, `colored` for terminal output. Subcommands live in `src/commands/`. Commands: `scan`, `details <KEY>`, `check`, `init`. Both `scan` and `details` support `--source` for ad-hoc mode or read from `rotary.toml`. `check` is the CI command — exits non-zero if health score < threshold. Shared source construction logic is in `commands/sources.rs`.

Non-Rust:
- `playbooks/` — TOML files describing step-by-step rotation procedures for specific secret types. Each playbook has a `patterns` field (case-insensitive substrings) that matches against secret key names. The `details` command walks up from the current directory to find `playbooks/` and matches automatically.

## Config Files

**`rotary.toml`** — Source and scan configuration. Walks up parent directories (like `.gitignore`).
- `[scan]` — `max_age_days`, `warning_threshold_days`, `project_root` (for unused-secret detection)
- `[[sources]]` — each entry has `name`, `type`, `path`, `environment`, plus arbitrary connector-specific keys via `#[serde(flatten)]` into a `settings: HashMap<String, String>`. For example a Doppler source has `token`, `project`, `config` as extra keys on the same TOML table.

**`.rotary-owners.toml`** — Owner mapping. Maps glob patterns to owner names. Rules are evaluated in order, first match wins. Applied by the scanner before health checks run, so the "no owner" warning is suppressed for matched keys.

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
- **Owner resolution**: The scanner applies owner overrides from `.rotary-owners.toml` before running health checks. Owners already set by the source are not overridden.
