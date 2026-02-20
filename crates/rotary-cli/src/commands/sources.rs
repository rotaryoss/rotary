use rotary_connectors::DotEnvSource;
use rotary_core::{RotaryError, SecretSource};

pub fn build_source(
    source_type: &str,
    path: Option<&str>,
    environment: &str,
) -> Result<Box<dyn SecretSource>, RotaryError> {
    match source_type {
        "dotenv" => {
            let path = path.ok_or_else(|| {
                RotaryError::Config("path is required for the dotenv source".into())
            })?;
            Ok(Box::new(DotEnvSource::new(path, environment)))
        }
        other => Err(RotaryError::Config(format!(
            "unknown source type: {other}. Supported: dotenv"
        ))),
    }
}
