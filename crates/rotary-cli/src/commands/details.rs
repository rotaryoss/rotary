use std::path::PathBuf;

use rotary_core::{
    find_matching_playbook, load_playbooks, RotaryConfig, RotaryError, SecretSource,
};

use super::sources::{build_source, build_source_adhoc};
use crate::output;

pub async fn run(
    key: &str,
    source_flag: Option<&str>,
    path_flag: Option<&str>,
    env_flag: Option<&str>,
) -> Result<(), RotaryError> {
    let cwd = std::env::current_dir().map_err(|e| RotaryError::Other(e.to_string()))?;

    // Resolve sources to search through.
    let sources_and_envs: Vec<(Box<dyn SecretSource>, String)> = if let Some(source_name) =
        source_flag
    {
        let env = env_flag.unwrap_or("default").to_string();
        let source = build_source_adhoc(source_name, path_flag, &env)?;
        vec![(source, env)]
    } else {
        let (config, config_dir) = RotaryConfig::find_and_load(&cwd)?.ok_or_else(|| {
            RotaryError::Config(
                "no rotary.toml found. Run `rotary init` or use --source for ad-hoc lookup."
                    .into(),
            )
        })?;

        if config.sources.is_empty() {
            return Err(RotaryError::Config(
                "rotary.toml has no [[sources]] entries.".into(),
            ));
        }

        config
            .sources
            .iter()
            .filter_map(|entry| {
                let mut resolved = entry.clone();
                if let Some(ref p) = resolved.path {
                    let pb = PathBuf::from(p);
                    if pb.is_relative() {
                        resolved.path =
                            Some(config_dir.join(pb).to_string_lossy().into_owned());
                    }
                }
                if let Some(env_override) = env_flag {
                    resolved.environment = env_override.to_string();
                }
                let env = resolved.environment.clone();
                build_source(&resolved).ok().map(|s| (s, env))
            })
            .collect()
    };

    // Search all sources for the key.
    let mut found_metadata = None;
    let mut found_env = String::new();
    let mut found_source_name = String::new();

    for (source, env) in &sources_and_envs {
        let secrets = source.list_secrets().await?;
        if let Some(meta) = secrets.into_iter().find(|s| s.key == key) {
            found_env = env.clone();
            found_source_name = source.source_name().to_string();
            found_metadata = Some(meta);
            break;
        }
    }

    let metadata = found_metadata.ok_or_else(|| {
        RotaryError::Other(format!(
            "secret \"{key}\" not found in any configured source"
        ))
    })?;

    // Load playbooks.
    let playbooks_dir = find_playbooks_dir(&cwd);
    let playbooks = playbooks_dir
        .map(|d| load_playbooks(&d))
        .transpose()?
        .unwrap_or_default();

    let matching_playbook = find_matching_playbook(&playbooks, key);

    output::print_details(&metadata, &found_source_name, &found_env, matching_playbook);

    Ok(())
}

/// Walk up from `start` to find a `playbooks/` directory.
fn find_playbooks_dir(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join("playbooks");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}
