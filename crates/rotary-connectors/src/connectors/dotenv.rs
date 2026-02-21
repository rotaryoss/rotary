use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rotaryoss_core::{AuditEntry, RotaryError, SecretMetadata, SecretSource};

/// Reads secrets from a `.env` file on disk.
///
/// Since `.env` files have no rotation metadata, `last_rotated` and
/// `last_accessed` are derived from the file's filesystem timestamps.
/// The owner field is always `None`.
pub struct DotEnvSource {
    path: PathBuf,
    environment: String,
}

impl DotEnvSource {
    pub fn new(path: impl Into<PathBuf>, environment: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            environment: environment.into(),
        }
    }

    fn file_modified_time(path: &Path) -> Option<DateTime<Utc>> {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(DateTime::<Utc>::from)
    }

    /// Parse a `.env` file and extract key names.
    /// Skips blank lines and comments. Does not evaluate values.
    fn parse_keys(contents: &str) -> Vec<String> {
        contents
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }
                let key = trimmed.split('=').next()?.trim();
                if key.is_empty() {
                    return None;
                }
                Some(key.to_string())
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl SecretSource for DotEnvSource {
    async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError> {
        let contents =
            tokio::fs::read_to_string(&self.path)
                .await
                .map_err(|e| RotaryError::Connector {
                    source_name: self.source_name().to_string(),
                    message: format!("failed to read {}: {e}", self.path.display()),
                })?;

        let modified = Self::file_modified_time(&self.path);
        let keys = Self::parse_keys(&contents);

        let secrets = keys
            .into_iter()
            .map(|key| SecretMetadata {
                key,
                last_rotated: modified,
                last_accessed: None,
                environment: self.environment.clone(),
                owner: None,
                created_at: modified.unwrap_or_else(Utc::now),
            })
            .collect();

        Ok(secrets)
    }

    async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError> {
        // .env files have no audit log.
        Ok(vec![])
    }

    fn source_name(&self) -> &str {
        "dotenv"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keys_basic() {
        let input = r#"
# Database config
DATABASE_URL=postgres://localhost/mydb
REDIS_URL=redis://localhost

# Empty line above
API_KEY=secret123
MALFORMED_LINE
"#;
        let keys = DotEnvSource::parse_keys(input);
        assert_eq!(
            keys,
            vec!["DATABASE_URL", "REDIS_URL", "API_KEY", "MALFORMED_LINE"]
        );
    }

    #[test]
    fn parse_keys_empty() {
        assert!(DotEnvSource::parse_keys("").is_empty());
        assert!(DotEnvSource::parse_keys("# only comments\n# here").is_empty());
    }

    #[test]
    fn parse_keys_with_spaces() {
        let input = "  KEY_WITH_SPACES  = value\nANOTHER=val";
        let keys = DotEnvSource::parse_keys(input);
        assert_eq!(keys, vec!["KEY_WITH_SPACES", "ANOTHER"]);
    }
}
