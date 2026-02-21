# Rotary

Audit secret health across your existing vaults — no migration, no storage, no trust problem.

Rotary is a CLI that connects to your secret managers, reads **only metadata** (key names, rotation dates, owners — never the secret values), and gives you a health report. It tells you which secrets are stale, unowned, or unused.

```
$ rotary scan

  ROTARY — Secret Health Report
  Production · 24 secrets · scanned just now

  ● CRITICAL   STRIPE_SECRET_KEY        last rotated 187 days ago    no owner assigned
  ● WARNING    SENDGRID_API_KEY         last rotated 94 days ago
  ✓ OK         DATABASE_URL             rotated 12 days ago
  ✓ OK         REDIS_URL                rotated 31 days ago

  Health score: 61/100
```

## Install

```bash
# From crates.io
cargo install rotaryoss-cli

# From source
git clone https://github.com/rotaryoss/rotary.git
cd rotary
cargo install --path crates/rotary-cli
```

## Quick Start

```bash
# Initialize a config file
rotary init

# Edit rotary.toml to add your sources, then:
rotary scan

# Or scan a .env file directly
rotary scan --source dotenv --path .env --env production

# Get details and rotation playbook for a specific key
rotary details STRIPE_SECRET_KEY

# Machine-readable output
rotary scan --json
```

## How It Works

Rotary runs three health checks against each secret:

1. **Rotation age** — flags secrets that haven't been rotated within a configurable threshold (default: 90 days critical, 75 days warning)
2. **Missing owner** — flags secrets with no assigned owner
3. **Unreferenced** — scans your codebase to find secrets that exist in the vault but aren't used anywhere

Each check produces a severity (OK, Warning, Critical) and the worst wins. The health score is computed across all secrets: Critical = 1.0 deduction, Warning = 0.5, normalized over total count.

## Connectors

Each vault integration implements the `SecretSource` trait from `rotary-core`. Rotary never sees your secret values — only metadata.

| Connector | Status |
|-----------|--------|
| `.env` files | Available |
| Doppler | Planned |
| AWS Secrets Manager | Planned |
| HashiCorp Vault | Planned |

### Writing a Connector

Implement the `SecretSource` trait:

```rust
use rotary_core::{SecretSource, SecretMetadata, AuditEntry, RotaryError};

#[async_trait::async_trait]
impl SecretSource for MyVault {
    async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError> {
        // Return metadata — never the secret values.
        todo!()
    }

    async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError> {
        Ok(vec![]) // Optional — return empty if unsupported.
    }

    fn source_name(&self) -> &str {
        "my-vault"
    }
}
```

See the [dotenv connector](crates/rotary-connectors/src/connectors/dotenv.rs) for a complete example.

## Rotation Playbooks

Playbooks are TOML files in `playbooks/` that describe step-by-step rotation procedures. The `details` command automatically matches a secret to the right playbook using pattern matching:

```toml
[playbook]
name = "stripe-api-key"
description = "Rotate Stripe secret API key"
patterns = ["stripe"]

[[steps]]
action = "generate"
description = "Generate a new restricted key in the Stripe Dashboard."

[[steps]]
action = "deploy"
description = "Update the secret in your vault and deploy."

[[steps]]
action = "verify"
description = "Confirm the new key works."

[[steps]]
action = "revoke"
description = "Revoke the old key."
```

## Configuration

`rotary.toml` configures sources and scan thresholds. Run `rotary init` to generate a starter file.

```toml
[scan]
max_age_days = 90
warning_threshold_days = 75
project_root = "."

[[sources]]
name = "production"
type = "dotenv"
path = ".env.production"
environment = "production"
```

## Project Structure

```
crates/
├── rotary-core/        # SecretSource trait, types, config, playbooks
├── rotary-connectors/  # Vault integrations
├── rotary-scanner/     # Health check engine
└── rotary-cli/         # The `rotary` binary
playbooks/              # Rotation playbook definitions
```

## Contributing

Contributions are welcome. The most impactful contributions right now are **new connectors** and **rotation playbooks**.

To add a connector:
1. Create `crates/rotary-connectors/src/connectors/<name>.rs`
2. Implement `SecretSource`
3. Add the module to `connectors/mod.rs` and re-export from `lib.rs`
4. Add a match arm in `crates/rotary-cli/src/commands/sources.rs`
5. Open a PR

To add a playbook:
1. Create `playbooks/<service>.toml`
2. Add `patterns` that match the key names for that service
3. Add `[[steps]]` with the rotation procedure
4. Open a PR

## License

MIT
