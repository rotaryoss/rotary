use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the health check rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Maximum age in days before a secret is flagged as stale.
    pub max_age_days: u64,

    /// Days before max_age to start warning.
    pub warning_threshold_days: u64,

    /// Project root to scan for secret key references.
    /// When set, enables the "unreferenced secret" check.
    #[serde(default)]
    pub project_root: Option<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_age_days: 90,
            warning_threshold_days: 75,
            project_root: None,
        }
    }
}
