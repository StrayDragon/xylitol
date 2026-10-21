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

impl From<xylitol_plugin_sdk::model::ThinkingConfigError> for LoadError {
    fn from(err: xylitol_plugin_sdk::model::ThinkingConfigError) -> Self {
        Self::validation(err.to_string())
    }
}
