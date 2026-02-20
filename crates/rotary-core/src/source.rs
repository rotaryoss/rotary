use crate::error::RotaryError;
use crate::types::{AuditEntry, SecretMetadata};

/// The core abstraction for vault integrations.
///
/// Every connector (`.env`, Doppler, AWS Secrets Manager, etc.) implements
/// this trait. The scanner operates exclusively through this interface,
/// which means it never sees secret values — only metadata.
#[async_trait::async_trait]
pub trait SecretSource: Send + Sync {
    /// List all secrets and their metadata from this source.
    async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError>;

    /// Retrieve the audit log for secrets in this source.
    /// Returns an empty vec if the source doesn't support audit logs.
    async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError>;

    /// Human-readable name for this source (e.g. "doppler", "aws-secrets-manager").
    fn source_name(&self) -> &str;
}
