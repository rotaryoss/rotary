mod connectors;

pub use connectors::doppler::DopplerSource;
pub use connectors::dotenv::DotEnvSource;

// Re-export the trait so callers don't need to depend on rotary-core directly.
pub use rotaryoss_core::SecretSource;
