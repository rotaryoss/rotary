use thiserror::Error;

#[derive(Debug, Error)]
pub enum RotaryError {
    #[error("connector error ({source_name}): {message}")]
    Connector {
        source_name: String,
        message: String,
    },

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
