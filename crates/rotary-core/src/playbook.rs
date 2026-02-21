use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::RotaryError;

/// A rotation playbook loaded from a TOML file in `playbooks/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub playbook: PlaybookMeta,
    pub steps: Vec<PlaybookStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookMeta {
    pub name: String,
    pub description: String,

    /// Case-insensitive substrings to match against secret key names.
    /// e.g. `["stripe", "stripe_secret"]` matches "STRIPE_SECRET_KEY".
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookStep {
    pub action: String,
    pub description: String,
}

impl Playbook {
    /// Check if this playbook matches a given secret key name.
    /// Match is case-insensitive substring.
    pub fn matches_key(&self, key: &str) -> bool {
        let key_lower = key.to_lowercase();
        self.playbook
            .patterns
            .iter()
            .any(|p| key_lower.contains(&p.to_lowercase()))
    }
}

/// Load all playbook TOML files from a directory.
pub fn load_playbooks(dir: &Path) -> Result<Vec<Playbook>, RotaryError> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }

    let mut playbooks = Vec::new();

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let contents = std::fs::read_to_string(&path)?;
        let playbook: Playbook = toml::from_str(&contents)
            .map_err(|e| RotaryError::Config(format!("failed to parse {}: {e}", path.display())))?;
        playbooks.push(playbook);
    }

    Ok(playbooks)
}

/// Find the first playbook that matches a secret key.
pub fn find_matching_playbook<'a>(playbooks: &'a [Playbook], key: &str) -> Option<&'a Playbook> {
    playbooks.iter().find(|p| p.matches_key(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playbook_matches_key() {
        let pb = Playbook {
            playbook: PlaybookMeta {
                name: "stripe".into(),
                description: "Rotate Stripe key".into(),
                patterns: vec!["stripe".into()],
            },
            steps: vec![],
        };

        assert!(pb.matches_key("STRIPE_SECRET_KEY"));
        assert!(pb.matches_key("stripe_api_key"));
        assert!(!pb.matches_key("SENDGRID_KEY"));
    }

    #[test]
    fn playbook_no_patterns_matches_nothing() {
        let pb = Playbook {
            playbook: PlaybookMeta {
                name: "generic".into(),
                description: "Generic playbook".into(),
                patterns: vec![],
            },
            steps: vec![],
        };

        assert!(!pb.matches_key("ANYTHING"));
    }

    #[test]
    fn parse_playbook_toml() {
        let toml = r#"
[playbook]
name = "stripe-api-key"
description = "Rotate Stripe secret API key"
patterns = ["stripe"]

[[steps]]
action = "generate"
description = "Generate a new key."

[[steps]]
action = "deploy"
description = "Deploy the new key."
"#;
        let pb: Playbook = toml::from_str(toml).unwrap();
        assert_eq!(pb.playbook.name, "stripe-api-key");
        assert_eq!(pb.playbook.patterns, vec!["stripe"]);
        assert_eq!(pb.steps.len(), 2);
    }

    #[test]
    fn parse_playbook_without_patterns() {
        let toml = r#"
[playbook]
name = "generic"
description = "A generic playbook"

[[steps]]
action = "rotate"
description = "Do the thing."
"#;
        let pb: Playbook = toml::from_str(toml).unwrap();
        assert!(pb.playbook.patterns.is_empty());
    }

    #[test]
    fn find_matching() {
        let playbooks = vec![
            Playbook {
                playbook: PlaybookMeta {
                    name: "stripe".into(),
                    description: String::new(),
                    patterns: vec!["stripe".into()],
                },
                steps: vec![],
            },
            Playbook {
                playbook: PlaybookMeta {
                    name: "sendgrid".into(),
                    description: String::new(),
                    patterns: vec!["sendgrid".into()],
                },
                steps: vec![],
            },
        ];

        let m = find_matching_playbook(&playbooks, "STRIPE_SECRET_KEY");
        assert_eq!(m.unwrap().playbook.name, "stripe");

        let m = find_matching_playbook(&playbooks, "SENDGRID_API_KEY");
        assert_eq!(m.unwrap().playbook.name, "sendgrid");

        let m = find_matching_playbook(&playbooks, "DATABASE_URL");
        assert!(m.is_none());
    }
}
