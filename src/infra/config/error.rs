//! Config load / validate / template failures (crate-private; not `Xy*`).

/// Errors from config loading and field validation.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("YAML parse error in {path}: {source}")]
    Yaml {
        path: String,
        source: yaml_serde::Error,
    },
    #[error("{0}")]
    Template(String),

    #[error("deserialize: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("{0}")]
    Validation(String),
}

impl LoadError {
    pub(crate) fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub(crate) fn template(message: impl Into<String>) -> Self {
        Self::Template(message.into())
    }
}

impl From<crate::protocol::model::ThinkingConfigError> for LoadError {
    fn from(err: crate::protocol::model::ThinkingConfigError) -> Self {
        Self::validation(err.to_string())
    }
}
