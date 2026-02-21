use rotaryoss_connectors::{DopplerSource, DotEnvSource};
use rotaryoss_core::{RotaryError, SecretSource, SourceEntry};

/// Build a `SecretSource` from a config entry.
///
/// Each connector reads what it needs from `entry.path`, `entry.environment`,
/// and `entry.settings`. This is the single place to wire up new connectors.
pub fn build_source(entry: &SourceEntry) -> Result<Box<dyn SecretSource>, RotaryError> {
    match entry.source_type.as_str() {
        "dotenv" => {
            let path = entry.path.as_deref().ok_or_else(|| {
                RotaryError::Config("'path' is required for the dotenv source".into())
            })?;
            Ok(Box::new(DotEnvSource::new(path, &entry.environment)))
        }
        "doppler" => {
            let token = entry
                .settings
                .get("token")
                .ok_or_else(|| {
                    RotaryError::Config("'token' is required for the doppler source".into())
                })?
                .clone();
            let project = entry
                .settings
                .get("project")
                .ok_or_else(|| {
                    RotaryError::Config("'project' is required for the doppler source".into())
                })?
                .clone();
            let config = entry
                .settings
                .get("config")
                .ok_or_else(|| {
                    RotaryError::Config(
                        "'config' is required for the doppler source (e.g. \"prd\", \"stg\")"
                            .into(),
                    )
                })?
                .clone();
            Ok(Box::new(DopplerSource::new(
                token,
                project,
                config,
                &entry.environment,
            )))
        }
        other => Err(RotaryError::Config(format!(
            "unknown source type: {other}. Supported: dotenv, doppler"
        ))),
    }
}

/// Convenience constructor for ad-hoc CLI usage (--source flag).
/// Builds a minimal `SourceEntry` from individual arguments.
pub fn build_source_adhoc(
    source_type: &str,
    path: Option<&str>,
    environment: &str,
) -> Result<Box<dyn SecretSource>, RotaryError> {
    let entry = SourceEntry {
        name: "adhoc".into(),
        source_type: source_type.into(),
        path: path.map(String::from),
        environment: environment.into(),
        settings: Default::default(),
    };
    build_source(&entry)
}
