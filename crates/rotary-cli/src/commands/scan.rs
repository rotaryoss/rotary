use rotary_core::{RotaryConfig, RotaryError};
use rotary_scanner::{ScanConfig, Scanner};

use super::sources::build_source;
use crate::output;

pub async fn run(
    source_flag: Option<&str>,
    path_flag: Option<&str>,
    env_flag: Option<&str>,
    max_age_flag: Option<u64>,
    json: bool,
) -> Result<(), RotaryError> {
    if let Some(source_name) = source_flag {
        // Explicit --source flag: single ad-hoc scan (original behavior).
        let env = env_flag.unwrap_or("default");
        let max_age = max_age_flag.unwrap_or(90);
        let source = build_source(source_name, path_flag, env)?;
        let cwd = std::env::current_dir().ok();
        let config = ScanConfig {
            max_age_days: max_age,
            warning_threshold_days: max_age.saturating_sub(15),
            project_root: cwd,
        };
        let scanner = Scanner::new(config);
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
    let cwd = std::env::current_dir().map_err(|e| RotaryError::Other(e.to_string()))?;
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
    let scanner = Scanner::new(scan_config);

    for entry in &config.sources {
        // Resolve relative paths against the config file's directory.
        let resolved_path = entry.path.as_ref().map(|p| {
            let pb = std::path::PathBuf::from(p);
            if pb.is_relative() {
                config_dir.join(pb).to_string_lossy().into_owned()
            } else {
                p.clone()
            }
        });

        let env = env_flag.unwrap_or(&entry.environment);
        let source = build_source(
            &entry.source_type,
            resolved_path.as_deref(),
            env,
        )?;

        let report = scanner.scan(source.as_ref(), env).await?;

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

