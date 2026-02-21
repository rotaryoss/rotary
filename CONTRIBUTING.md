# Contributing to Rotary

## Getting Started

```bash
git clone https://github.com/rotaryoss/rotary.git
cd rotary
cargo test --workspace
```

Requires Rust stable (latest).

## What to Contribute

**New connectors** — the most impactful contribution. See [Adding a Connector](#adding-a-connector) below.

**Rotation playbooks** — TOML files in `playbooks/` describing how to rotate secrets for specific services. Low effort, high value.

**Bug fixes and improvements** — always welcome. Open an issue first for larger changes so we can discuss the approach.

## Adding a Connector

Each vault integration implements the `SecretSource` trait. Rotary only reads metadata — never secret values.

1. Create `crates/rotary-connectors/src/connectors/<name>.rs`
2. Implement `SecretSource`:

```rust
use rotary_core::{SecretSource, SecretMetadata, AuditEntry, RotaryError};

pub struct MyVaultSource {
    // Your connector's config fields (token, region, etc.)
}

impl MyVaultSource {
    pub fn new(/* ... */) -> Self { /* ... */ }
}

#[async_trait::async_trait]
impl SecretSource for MyVaultSource {
    async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError> {
        // Fetch metadata from the vault API.
        // Return key names, rotation dates, owners — never secret values.
        todo!()
    }

    async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError> {
        // Return empty vec if the vault doesn't support audit logs.
        Ok(vec![])
    }

    fn source_name(&self) -> &str {
        "my-vault"
    }
}
```

3. Add `pub mod <name>;` to `crates/rotary-connectors/src/connectors/mod.rs`
4. Re-export from `crates/rotary-connectors/src/lib.rs`
5. Add a match arm in `crates/rotary-cli/src/commands/sources.rs`:

```rust
"my-vault" => {
    let token = entry.settings.get("token").ok_or_else(|| {
        RotaryError::Config("'token' is required for my-vault source".into())
    })?;
    Ok(Box::new(MyVaultSource::new(token, &entry.environment)))
}
```

Connector-specific settings come from `entry.settings` — a `HashMap<String, String>` populated by extra keys on the `[[sources]]` TOML table.

6. Add tests and open a PR.

## Adding a Playbook

1. Create `playbooks/<service>.toml`:

```toml
[playbook]
name = "my-service-api-key"
description = "Rotate My Service API key"
patterns = ["my_service", "myservice"]

[[steps]]
action = "generate"
description = "Create a new API key in the My Service dashboard."

[[steps]]
action = "deploy"
description = "Update the secret in your vault and deploy."

[[steps]]
action = "verify"
description = "Confirm the new key works."

[[steps]]
action = "revoke"
description = "Delete the old key."
```

2. `patterns` are case-insensitive substrings matched against secret key names.
3. Open a PR.

## Development

```bash
cargo check                        # Fast type-check
cargo test --workspace             # All tests
cargo test -p rotary-core          # Single crate
cargo clippy --workspace           # Lint
cargo fmt --all                    # Format
```

All PRs must pass CI: tests on Linux/macOS/Windows, clippy with no warnings, and `cargo fmt` check.

## Pull Requests

- Keep PRs focused — one connector or one feature per PR.
- Add tests for new functionality.
- Run `cargo fmt --all` before pushing.
- Update `README.md` if adding a new connector (add it to the connector table).
