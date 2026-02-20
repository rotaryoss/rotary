use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata about a secret — never the secret value itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// The key name (e.g. "STRIPE_SECRET_KEY").
    pub key: String,

    /// When the secret was last rotated, if known.
    pub last_rotated: Option<DateTime<Utc>>,

    /// When the secret was last accessed, if known.
    pub last_accessed: Option<DateTime<Utc>>,

    /// The environment this secret belongs to (e.g. "production", "staging").
    pub environment: String,

    /// The team member or service account that owns this secret.
    pub owner: Option<String>,

    /// When the secret was first created.
    pub created_at: DateTime<Utc>,
}

/// A single entry in a secret's audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub key: String,
    pub action: AuditAction,
    pub actor: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Rotated,
    Accessed,
    Deleted,
    OwnerChanged,
}
