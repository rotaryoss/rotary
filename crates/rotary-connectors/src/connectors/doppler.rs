use chrono::{DateTime, Utc};
use rotaryoss_core::{AuditEntry, RotaryError, SecretMetadata, SecretSource};
use serde::Deserialize;

/// Reads secret metadata from the Doppler API.
///
/// Uses the `v3/configs/config/secrets` endpoint with `include_dynamic_secrets`
/// and `include_managed_secrets` enabled. Only reads metadata fields —
/// secret values are never stored or logged.
pub struct DopplerSource {
    token: String,
    project: String,
    config: String,
    environment: String,
    base_url: String,
}

impl DopplerSource {
    pub fn new(
        token: impl Into<String>,
        project: impl Into<String>,
        config: impl Into<String>,
        environment: impl Into<String>,
    ) -> Self {
        Self {
            token: token.into(),
            project: project.into(),
            config: config.into(),
            environment: environment.into(),
            base_url: "https://api.doppler.com".into(),
        }
    }
}

/// Doppler activity log entry.
#[derive(Debug, Deserialize)]
struct DopplerActivityLog {
    logs: Vec<DopplerActivity>,
}

#[derive(Debug, Deserialize)]
struct DopplerActivity {
    text: Option<String>,
    user: Option<DopplerUser>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DopplerUser {
    name: Option<String>,
}

#[async_trait::async_trait]
impl SecretSource for DopplerSource {
    async fn list_secrets(&self) -> Result<Vec<SecretMetadata>, RotaryError> {
        let client = reqwest::Client::new();

        let url = format!("{}/v3/configs/config/secrets", self.base_url);

        let resp = client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("project", &self.project), ("config", &self.config)])
            .send()
            .await
            .map_err(|e| RotaryError::Connector {
                source_name: self.source_name().to_string(),
                message: format!("failed to call Doppler API: {e}"),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(RotaryError::Connector {
                source_name: self.source_name().to_string(),
                message: format!("Doppler API returned {status}: {body}"),
            });
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| RotaryError::Connector {
            source_name: self.source_name().to_string(),
            message: format!("failed to parse Doppler response: {e}"),
        })?;

        let secrets_obj = body
            .get("secrets")
            .and_then(|v| v.as_object())
            .ok_or_else(|| RotaryError::Connector {
                source_name: self.source_name().to_string(),
                message: "unexpected Doppler response: missing 'secrets' object".into(),
            })?;

        let now = Utc::now();
        let secrets = secrets_obj
            .keys()
            .filter(|k| {
                // Skip Doppler's computed keys.
                !k.starts_with("DOPPLER_")
            })
            .map(|key| SecretMetadata {
                key: key.clone(),
                // Doppler doesn't expose per-key rotation timestamps in this endpoint.
                // The activity log (fetched separately in get_audit_log) has this info.
                last_rotated: None,
                last_accessed: None,
                environment: self.environment.clone(),
                owner: None,
                created_at: now,
            })
            .collect();

        Ok(secrets)
    }

    async fn get_audit_log(&self) -> Result<Vec<AuditEntry>, RotaryError> {
        let client = reqwest::Client::new();

        let url = format!("{}/v3/logs", self.base_url);

        let resp = client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[
                ("project", &self.project),
                ("config", &self.config),
                ("per_page", &"100".to_string()),
            ])
            .send()
            .await
            .map_err(|e| RotaryError::Connector {
                source_name: self.source_name().to_string(),
                message: format!("failed to fetch Doppler activity log: {e}"),
            })?;

        if !resp.status().is_success() {
            // Activity log is optional — return empty on failure.
            return Ok(vec![]);
        }

        let log: DopplerActivityLog = resp.json().await.map_err(|e| RotaryError::Connector {
            source_name: self.source_name().to_string(),
            message: format!("failed to parse Doppler activity log: {e}"),
        })?;

        let entries = log
            .logs
            .into_iter()
            .filter_map(|activity| {
                let timestamp: DateTime<Utc> =
                    activity.created_at.as_ref().and_then(|s| s.parse().ok())?;
                let actor = activity.user.and_then(|u| u.name);
                let text = activity.text.unwrap_or_default();
                Some(AuditEntry {
                    key: text,
                    action: rotaryoss_core::AuditAction::Rotated,
                    actor,
                    timestamp,
                })
            })
            .collect();

        Ok(entries)
    }

    fn source_name(&self) -> &str {
        "doppler"
    }
}
