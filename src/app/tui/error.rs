//! Product-TUI surface failures (crate-private; not `Xy*`).

use crate::app::core::driver::XyDriverError;

/// Keybindings / theme / terminal / external-editor failures.
///
/// `InvalidInput` vs `Io` is the stable split; payloads stay the original body
/// so Driver Display is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TuiSurfaceError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    Io(String),
}

impl TuiSurfaceError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }
}

impl From<TuiSurfaceError> for XyDriverError {
    fn from(err: TuiSurfaceError) -> Self {
        match err {
            TuiSurfaceError::InvalidInput(message) => Self::invalid_input(message),
            TuiSurfaceError::Io(message) => Self::io(message),
        }
    }
}
