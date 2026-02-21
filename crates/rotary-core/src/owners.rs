use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::RotaryError;

const OWNERS_FILENAME: &str = ".rotary-owners.toml";

/// Maps secret key patterns to owners/teams.
///
/// ```toml
/// [[owners]]
/// pattern = "STRIPE_*"
/// owner = "payments-team"
///
/// [[owners]]
/// pattern = "DATABASE_*"
/// owner = "infra-team"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnersConfig {
    #[serde(default)]
    pub owners: Vec<OwnerRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerRule {
    /// Glob pattern matched against the secret key name (e.g. "STRIPE_*", "DB_*").
    pub pattern: String,

    /// The owner name (person or team) assigned to matching secrets.
    pub owner: String,
}

impl OwnersConfig {
    /// Search for `.rotary-owners.toml` starting from `start_dir` and walking
    /// up to parent directories. Returns the parsed config if found.
    pub fn find_and_load(start_dir: &Path) -> Result<Option<Self>, RotaryError> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join(OWNERS_FILENAME);
            if candidate.is_file() {
                return Self::load(&candidate).map(Some);
            }
            if !dir.pop() {
                return Ok(None);
            }
        }
    }

    /// Load from a specific path.
    pub fn load(path: &PathBuf) -> Result<Self, RotaryError> {
        let contents = std::fs::read_to_string(path)?;
        let config: OwnersConfig = toml::from_str(&contents)
            .map_err(|e| RotaryError::Config(format!("failed to parse {}: {e}", path.display())))?;
        Ok(config)
    }

    /// Find the owner for a given secret key by matching against rules in order.
    /// First match wins.
    pub fn resolve_owner(&self, key: &str) -> Option<&str> {
        for rule in &self.owners {
            let pattern = glob::Pattern::new(&rule.pattern).ok()?;
            if pattern.matches(key) {
                return Some(&rule.owner);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_owner_basic() {
        let config = OwnersConfig {
            owners: vec![
                OwnerRule {
                    pattern: "STRIPE_*".into(),
                    owner: "payments-team".into(),
                },
                OwnerRule {
                    pattern: "DB_*".into(),
                    owner: "infra-team".into(),
                },
                OwnerRule {
                    pattern: "DATABASE_*".into(),
                    owner: "infra-team".into(),
                },
            ],
        };

        assert_eq!(
            config.resolve_owner("STRIPE_SECRET_KEY"),
            Some("payments-team")
        );
        assert_eq!(
            config.resolve_owner("STRIPE_PUBLISHABLE_KEY"),
            Some("payments-team")
        );
        assert_eq!(config.resolve_owner("DB_URL"), Some("infra-team"));
        assert_eq!(config.resolve_owner("DATABASE_URL"), Some("infra-team"));
        assert_eq!(config.resolve_owner("REDIS_URL"), None);
    }

    #[test]
    fn resolve_owner_first_match_wins() {
        let config = OwnersConfig {
            owners: vec![
                OwnerRule {
                    pattern: "STRIPE_SECRET_*".into(),
                    owner: "security-team".into(),
                },
                OwnerRule {
                    pattern: "STRIPE_*".into(),
                    owner: "payments-team".into(),
                },
            ],
        };

        assert_eq!(
            config.resolve_owner("STRIPE_SECRET_KEY"),
            Some("security-team")
        );
        assert_eq!(
            config.resolve_owner("STRIPE_PUBLISHABLE_KEY"),
            Some("payments-team")
        );
    }

    #[test]
    fn resolve_owner_no_rules() {
        let config = OwnersConfig { owners: vec![] };
        assert_eq!(config.resolve_owner("ANYTHING"), None);
    }

    #[test]
    fn parse_owners_toml() {
        let toml = r#"
[[owners]]
pattern = "STRIPE_*"
owner = "payments-team"

[[owners]]
pattern = "DB_*"
owner = "infra"
"#;
        let config: OwnersConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.owners.len(), 2);
        assert_eq!(config.owners[0].owner, "payments-team");
    }
}
