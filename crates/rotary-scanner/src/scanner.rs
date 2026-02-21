use std::collections::HashSet;

use chrono::Utc;
use rotaryoss_core::{
    HealthCheck, HealthReport, HealthScore, OwnersConfig, RotaryError, SecretSource, Severity,
};

use crate::rules::ScanConfig;
use crate::usage;

pub struct Scanner {
    config: ScanConfig,
    owners: Option<OwnersConfig>,
}

impl Scanner {
    pub fn new(config: ScanConfig) -> Self {
        Self {
            config,
            owners: None,
        }
    }

    /// Attach an owner mapping that overrides missing owners before health checks.
    pub fn with_owners(mut self, owners: OwnersConfig) -> Self {
        self.owners = Some(owners);
        self
    }

    /// Run all health checks against a secret source and produce a report.
    pub async fn scan(
        &self,
        source: &dyn SecretSource,
        environment: &str,
    ) -> Result<HealthReport, RotaryError> {
        let mut secrets = source.list_secrets().await?;

        // Apply owner overrides from .rotary-owners.toml.
        if let Some(ref owners) = self.owners {
            for secret in &mut secrets {
                if secret.owner.is_none() {
                    if let Some(owner) = owners.resolve_owner(&secret.key) {
                        secret.owner = Some(owner.to_string());
                    }
                }
            }
        }

        let total_secrets = secrets.len();
        let now = Utc::now();

        // Rule 3 (usage check) — run upfront so we can reference results per-key.
        let unreferenced: HashSet<String> = if let Some(root) = &self.config.project_root {
            let keys: Vec<String> = secrets.iter().map(|s| s.key.clone()).collect();
            usage::find_unreferenced_keys(&keys, root)
        } else {
            HashSet::new()
        };

        let mut checks = Vec::with_capacity(total_secrets);

        for secret in &secrets {
            // Rule 1: Rotation age
            let age_check = if let Some(rotated) = secret.last_rotated {
                let age_days = (now - rotated).num_days();
                if age_days < 0 {
                    Severity::Ok
                } else if age_days as u64 > self.config.max_age_days {
                    Severity::Critical
                } else if age_days as u64 > self.config.warning_threshold_days {
                    Severity::Warning
                } else {
                    Severity::Ok
                }
            } else {
                Severity::Warning
            };

            // Rule 2: Missing owner
            let owner_check = if secret.owner.is_none() {
                Severity::Warning
            } else {
                Severity::Ok
            };

            // Rule 3: Unreferenced in codebase
            let usage_check = if unreferenced.contains(&secret.key) {
                Severity::Warning
            } else {
                Severity::Ok
            };

            // Take the worst severity across all rules.
            let severity = age_check.max(owner_check).max(usage_check);

            let reason = build_reason(
                &secret.key,
                severity,
                secret.last_rotated,
                secret.owner.as_deref(),
                &self.config,
                &unreferenced,
                now,
            );

            checks.push(HealthCheck {
                key: secret.key.clone(),
                severity,
                reason,
            });
        }

        // Sort: Critical first, then Warning, then Ok.
        checks.sort_by(|a, b| b.severity.cmp(&a.severity));

        let score = HealthScore::compute(&checks);

        Ok(HealthReport {
            source_name: source.source_name().to_string(),
            environment: environment.to_string(),
            total_secrets,
            checks,
            score,
        })
    }
}

fn build_reason(
    key: &str,
    severity: Severity,
    last_rotated: Option<chrono::DateTime<Utc>>,
    owner: Option<&str>,
    config: &ScanConfig,
    unreferenced: &HashSet<String>,
    now: chrono::DateTime<Utc>,
) -> String {
    match severity {
        Severity::Critical => {
            let days = last_rotated.map(|r| (now - r).num_days()).unwrap_or(0);
            let mut parts = vec![format!("last rotated {days} days ago")];
            if owner.is_none() {
                parts.push("no owner assigned".into());
            }
            if unreferenced.contains(key) {
                parts.push("not found in codebase".into());
            }
            parts.join("    ")
        }
        Severity::Warning => {
            let mut parts = Vec::new();
            if let Some(rotated) = last_rotated {
                let days = (now - rotated).num_days();
                if days as u64 > config.warning_threshold_days {
                    parts.push(format!("last rotated {days} days ago"));
                }
            } else {
                parts.push("no rotation data".into());
            }
            if owner.is_none() {
                parts.push("no owner assigned".into());
            }
            if unreferenced.contains(key) {
                parts.push("not found in codebase".into());
            }
            parts.join("    ")
        }
        Severity::Ok => {
            if let Some(rotated) = last_rotated {
                let days = (now - rotated).num_days();
                format!("rotated {days} days ago")
            } else {
                "ok".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{Duration, Utc};
    use rotaryoss_core::{AuditEntry, RotaryError, SecretMetadata, SecretSource, Severity};

    use super::*;
    use crate::rules::ScanConfig;

    struct MockSource {
        secrets: Vec<SecretMetadata>,
    }

    #[async_trait::async_trait]
    impl SecretSource for MockSource {
        async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError> {
            Ok(self.secrets.clone())
        }
        async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError> {
            Ok(vec![])
        }
        fn source_name(&self) -> &str {
            "mock"
        }
    }

    fn make_secret(key: &str, days_ago: Option<i64>, owner: Option<&str>) -> SecretMetadata {
        SecretMetadata {
            key: key.to_string(),
            last_rotated: days_ago.map(|d| Utc::now() - Duration::days(d)),
            last_accessed: None,
            environment: "test".into(),
            owner: owner.map(String::from),
            created_at: Utc::now() - Duration::days(days_ago.unwrap_or(0)),
        }
    }

    #[tokio::test]
    async fn scan_healthy_secrets() {
        let source = MockSource {
            secrets: vec![
                make_secret("DB_URL", Some(10), Some("alice")),
                make_secret("API_KEY", Some(30), Some("bob")),
            ],
        };
        let scanner = Scanner::new(ScanConfig::default());
        let report = scanner.scan(&source, "production").await.unwrap();
        assert_eq!(report.score.0, 100);
        assert!(report.checks.iter().all(|c| c.severity == Severity::Ok));
    }

    #[tokio::test]
    async fn scan_stale_secret() {
        let source = MockSource {
            secrets: vec![
                make_secret("STALE_KEY", Some(100), Some("alice")),
                make_secret("OK_KEY", Some(10), Some("bob")),
            ],
        };
        let scanner = Scanner::new(ScanConfig::default());
        let report = scanner.scan(&source, "production").await.unwrap();
        assert!(report.score.0 < 100);
        assert_eq!(report.checks[0].severity, Severity::Critical);
        assert_eq!(report.checks[0].key, "STALE_KEY");
    }

    #[tokio::test]
    async fn scan_missing_owner() {
        let source = MockSource {
            secrets: vec![make_secret("ORPHAN", Some(10), None)],
        };
        let scanner = Scanner::new(ScanConfig::default());
        let report = scanner.scan(&source, "staging").await.unwrap();
        assert_eq!(report.checks[0].severity, Severity::Warning);
    }

    #[tokio::test]
    async fn scan_unreferenced_secret() {
        let dir = tempfile::tempdir().unwrap();

        // Create a source file that only references DB_URL.
        let src = dir.path().join("config.rs");
        fs::write(&src, r#"let db = env("DB_URL");"#).unwrap();

        let source = MockSource {
            secrets: vec![
                make_secret("DB_URL", Some(10), Some("alice")),
                make_secret("GHOST_KEY", Some(10), Some("bob")),
            ],
        };

        let config = ScanConfig {
            project_root: Some(dir.path().to_path_buf()),
            ..Default::default()
        };
        let scanner = Scanner::new(config);
        let report = scanner.scan(&source, "production").await.unwrap();

        let ghost = report.checks.iter().find(|c| c.key == "GHOST_KEY").unwrap();
        assert_eq!(ghost.severity, Severity::Warning);
        assert!(ghost.reason.contains("not found in codebase"));

        let db = report.checks.iter().find(|c| c.key == "DB_URL").unwrap();
        assert_eq!(db.severity, Severity::Ok);
    }
}
