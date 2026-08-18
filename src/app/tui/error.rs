//! Product-TUI surface failures (crate-private; not `Xy*`).

use crate::app::core::driver::XyDriverError;

/// Keybindings / theme / terminal / external-editor failures.
///
/// `InvalidInput` vs `Io` is the stable split; payloads keep the original
/// body so Driver Display is unchanged. Real io sources are preserved.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TuiSurfaceError {
    /// Local io failure with a static context label.
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// External-editor process spawn failure.
    #[error("spawn {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    InvalidInput(String),
}

impl TuiSurfaceError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub(crate) fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    pub(crate) fn spawn(program: impl Into<String>, source: std::io::Error) -> Self {
        Self::Spawn {
            program: program.into(),
            source,
        }
    }
}

impl From<TuiSurfaceError> for XyDriverError {
    fn from(err: TuiSurfaceError) -> Self {
        match err {
            TuiSurfaceError::InvalidInput(message) => Self::invalid_input(message),
            TuiSurfaceError::Io { context, source } => Self::io(format!("{context}: {source}")),
            TuiSurfaceError::Spawn { program, source } => {
                Self::io(format!("spawn {program}: {source}"))
            }
        }
    }
}
