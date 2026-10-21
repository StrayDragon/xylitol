use crate::driver::XyDriverError;

#[derive(Debug, thiserror::Error)]
pub enum TuiSurfaceError {
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
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
