use rotaryoss_core::{OwnersConfig, RotaryConfig, RotaryError};
use rotaryoss_scanner::{ScanConfig, Scanner};

use super::sources::build_source;
use crate::output;

/// Exit code returned when the health score is below the threshold.
pub const EXIT_CHECK_FAILED: i32 = 2;

pub async fn run(threshold: u8, json: bool) -> Result<i32, RotaryError> {
    let cwd = std::env::current_dir().map_err(|e| RotaryError::Other(e.to_string()))?;

    let (config, config_dir) = RotaryConfig::find_and_load(&cwd)?.ok_or_else(|| {
        RotaryError::Config("no rotary.toml found. Run `rotary init` to create one.".into())
    })?;

    if config.sources.is_empty() {
        return Err(RotaryError::Config(
            "rotary.toml has no [[sources]] entries.".into(),
        ));
    }

    let owners = OwnersConfig::find_and_load(&cwd)?;

    let project_root = {
        let root = std::path::PathBuf::from(&config.scan.project_root);
        if root.is_relative() {
            config_dir.join(root)
        } else {
            root
        }
    };
    let scan_config = ScanConfig {
        max_age_days: config.scan.max_age_days,
        warning_threshold_days: config.scan.warning_threshold_days,
        project_root: Some(project_root),
    };
    let mut scanner = Scanner::new(scan_config);
    if let Some(owners) = owners {
        scanner = scanner.with_owners(owners);
    }

    let mut worst_score: u8 = 100;

    for entry in &config.sources {
        let mut resolved = entry.clone();
        if let Some(ref p) = resolved.path {
            let pb = std::path::PathBuf::from(p);
            if pb.is_relative() {
                resolved.path = Some(config_dir.join(pb).to_string_lossy().into_owned());
            }
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

        worst_score = worst_score.min(report.score.0);
    }

    if worst_score < threshold {
        if !json {
            eprintln!("check failed: health score {worst_score} is below threshold {threshold}");
        }
        Ok(EXIT_CHECK_FAILED)
    } else {
        if !json {
            eprintln!("check passed: health score {worst_score} >= threshold {threshold}");
        }
        Ok(0)
    }
}
