use rotaryoss_core::{OwnersConfig, RotaryConfig, RotaryError};
use rotaryoss_scanner::{ScanConfig, Scanner};

use super::sources::{build_source, build_source_adhoc};
use crate::output;

pub async fn run(
    source_flag: Option<&str>,
    path_flag: Option<&str>,
    env_flag: Option<&str>,
    max_age_flag: Option<u64>,
    json: bool,
) -> Result<(), RotaryError> {
    let cwd = std::env::current_dir().map_err(|e| RotaryError::Other(e.to_string()))?;
    let owners = OwnersConfig::find_and_load(&cwd)?;

    if let Some(source_name) = source_flag {
        // Explicit --source flag: single ad-hoc scan.
        let env = env_flag.unwrap_or("default");
        let max_age = max_age_flag.unwrap_or(90);
        let source = build_source_adhoc(source_name, path_flag, env)?;
        let config = ScanConfig {
            max_age_days: max_age,
            warning_threshold_days: max_age.saturating_sub(15),
            project_root: Some(cwd),
        };
        let mut scanner = Scanner::new(config);
        if let Some(owners) = owners {
            scanner = scanner.with_owners(owners);
        }
        let report = scanner.scan(source.as_ref(), env).await?;

        if json {
            let out = serde_json::to_string_pretty(&report)
                .map_err(|e| RotaryError::Other(e.to_string()))?;
            println!("{out}");
        } else {
            output::print_report(&report);
        }
        return Ok(());
    }

    // No --source flag: load from rotary.toml.
    let (config, config_dir) = RotaryConfig::find_and_load(&cwd)?
        .ok_or_else(|| {
            RotaryError::Config(
                "no rotary.toml found. Run `rotary init` to create one, or use --source for an ad-hoc scan.".into(),
            )
        })?;

    if config.sources.is_empty() {
        return Err(RotaryError::Config(
            "rotary.toml has no [[sources]] entries. Add at least one source to scan.".into(),
        ));
    }

    let max_age = max_age_flag.unwrap_or(config.scan.max_age_days);
    let project_root = {
        let root = std::path::PathBuf::from(&config.scan.project_root);
        if root.is_relative() {
            config_dir.join(root)
        } else {
            root
        }
    };
    let scan_config = ScanConfig {
        max_age_days: max_age,
        warning_threshold_days: max_age_flag
            .map(|m| m.saturating_sub(15))
            .unwrap_or(config.scan.warning_threshold_days),
        project_root: Some(project_root),
    };
    let mut scanner = Scanner::new(scan_config);
    if let Some(owners) = owners {
        scanner = scanner.with_owners(owners);
    }

    for entry in &config.sources {
        // Resolve relative paths against the config file's directory.
        let mut resolved = entry.clone();
        if let Some(ref p) = resolved.path {
            let pb = std::path::PathBuf::from(p);
            if pb.is_relative() {
                resolved.path = Some(config_dir.join(pb).to_string_lossy().into_owned());
            }
        }
        if let Some(env_override) = env_flag {
            resolved.environment = env_override.to_string();
        }

        let source = build_source(&resolved)?;
        let report = scanner.scan(source.as_ref(), &resolved.environment).await?;

        if json {
            let out = serde_json::to_string_pretty(&report)
                .map_err(|e| RotaryError::Other(e.to_string()))?;
            println!("{out}");
        } else {
            output::print_report(&report);
        }
    }

    Ok(())
}
