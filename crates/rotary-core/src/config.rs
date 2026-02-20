use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::RotaryError;

const CONFIG_FILENAME: &str = "rotary.toml";

/// Top-level project configuration loaded from `rotary.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotaryConfig {
    /// Default scan settings.
    #[serde(default)]
    pub scan: ScanDefaults,

    /// Secret sources to scan.
    #[serde(default)]
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDefaults {
    /// Maximum secret age in days before flagging as critical.
    #[serde(default = "default_max_age")]
    pub max_age_days: u64,

    /// Days before max_age to start warning.
    #[serde(default = "default_warning_threshold")]
    pub warning_threshold_days: u64,

    /// Path to scan for unused secret references (defaults to ".").
    #[serde(default = "default_project_root")]
    pub project_root: String,
}

impl Default for ScanDefaults {
    fn default() -> Self {
        Self {
            max_age_days: default_max_age(),
            warning_threshold_days: default_warning_threshold(),
            project_root: default_project_root(),
        }
    }
}

fn default_max_age() -> u64 {
    90
}
fn default_warning_threshold() -> u64 {
    75
}
fn default_project_root() -> String {
    ".".into()
}

/// A single secret source entry in the config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    /// Unique name for this source (e.g. "production-env", "doppler-prod").
    pub name: String,

    /// Source type: "dotenv", "doppler", "aws".
    #[serde(rename = "type")]
    pub source_type: String,

    /// File path (for dotenv) or API URL (for remote sources).
    pub path: Option<String>,

    /// Environment label.
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "default".into()
}

impl RotaryConfig {
    /// Search for `rotary.toml` starting from `start_dir` and walking up
    /// to parent directories. Returns the parsed config and the directory
    /// it was found in.
    pub fn find_and_load(start_dir: &Path) -> Result<Option<(Self, PathBuf)>, RotaryError> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join(CONFIG_FILENAME);
            if candidate.is_file() {
                let contents = std::fs::read_to_string(&candidate)?;
                let config: RotaryConfig = toml::from_str(&contents).map_err(|e| {
                    RotaryError::Config(format!("failed to parse {}: {e}", candidate.display()))
                })?;
                return Ok(Some((config, dir)));
            }
            if !dir.pop() {
                return Ok(None);
            }
        }
    }

    /// Generate a starter config file.
    pub fn starter() -> String {
        r#"# Rotary configuration
# Docs: https://github.com/rotary-dev/rotary

[scan]
max_age_days = 90
warning_threshold_days = 75
project_root = "."

# Add your secret sources below.
# Each [[sources]] block defines one source to scan.

# Example: scan a local .env file
# [[sources]]
# name = "local-env"
# type = "dotenv"
# path = ".env"
# environment = "development"

# Example: scan a production .env file
# [[sources]]
# name = "prod-env"
# type = "dotenv"
# path = ".env.production"
# environment = "production"
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let toml = r#"
[scan]
max_age_days = 60
warning_threshold_days = 45
project_root = "./src"

[[sources]]
name = "local"
type = "dotenv"
path = ".env"
environment = "development"

[[sources]]
name = "prod"
type = "dotenv"
path = ".env.production"
environment = "production"
"#;
        let config: RotaryConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.scan.max_age_days, 60);
        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.sources[0].source_type, "dotenv");
        assert_eq!(config.sources[1].environment, "production");
    }

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
[[sources]]
name = "dev"
type = "dotenv"
path = ".env"
"#;
        let config: RotaryConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.scan.max_age_days, 90); // default
        assert_eq!(config.sources[0].environment, "default"); // default
    }

    #[test]
    fn starter_is_valid_toml() {
        // The starter has all sources commented out, so it should parse
        // with empty sources vec.
        let config: RotaryConfig = toml::from_str(&RotaryConfig::starter()).unwrap();
        assert!(config.sources.is_empty());
        assert_eq!(config.scan.max_age_days, 90);
    }
}
