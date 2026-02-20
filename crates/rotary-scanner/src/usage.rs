use std::collections::HashSet;
use std::path::Path;

use ignore::WalkBuilder;

/// Directories to always skip, even if not in .gitignore.
const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", ".next", "dist", "build"];

/// Check which secret keys are referenced somewhere in the project tree.
/// Returns the set of keys that were *not* found in any file.
pub fn find_unreferenced_keys(keys: &[String], project_root: &Path) -> HashSet<String> {
    let mut remaining: HashSet<String> = keys.iter().cloned().collect();

    if remaining.is_empty() {
        return remaining;
    }

    let walker = WalkBuilder::new(project_root)
        .hidden(false) // search dotfiles like .env.example
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .filter_entry(|entry| {
            if let Some(name) = entry.file_name().to_str() {
                // Skip known heavy directories.
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    return !SKIP_DIRS.contains(&name);
                }
            }
            true
        })
        .build();

    for result in walker {
        if remaining.is_empty() {
            break;
        }

        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Only check files.
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        // Skip the rotary.toml itself and .env files (the source files).
        if let Some(name) = entry.file_name().to_str() {
            if name == "rotary.toml" || name.starts_with(".env") {
                continue;
            }
        }

        // Read file contents — skip files we can't read (binary, permissions).
        let contents = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        remaining.retain(|key| !contents.contains(key.as_str()));
    }

    remaining
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn finds_unreferenced_keys() {
        let dir = tempfile::tempdir().unwrap();

        // Create a source file that references some keys.
        let src = dir.path().join("app.rs");
        fs::write(&src, r#"let url = std::env::var("DATABASE_URL").unwrap();"#).unwrap();

        let keys = vec![
            "DATABASE_URL".to_string(),
            "GHOST_KEY".to_string(),
        ];

        let unreferenced = find_unreferenced_keys(&keys, dir.path());
        assert!(unreferenced.contains("GHOST_KEY"));
        assert!(!unreferenced.contains("DATABASE_URL"));
    }

    #[test]
    fn empty_keys_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let unreferenced = find_unreferenced_keys(&[], dir.path());
        assert!(unreferenced.is_empty());
    }

    #[test]
    fn skips_env_files() {
        let dir = tempfile::tempdir().unwrap();

        // .env files should be skipped — we don't want to find a key
        // "referenced" only in the .env file itself.
        let env_file = dir.path().join(".env");
        fs::write(&env_file, "ONLY_IN_ENV=secret").unwrap();

        let keys = vec!["ONLY_IN_ENV".to_string()];
        let unreferenced = find_unreferenced_keys(&keys, dir.path());
        assert!(unreferenced.contains("ONLY_IN_ENV"));
    }
}
