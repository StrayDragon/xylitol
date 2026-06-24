//! Focus context — which area of the TUI has keyboard focus.

/// Current focus area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusCtx {
    /// The composer (input area) has focus.
    Composer,
    /// The transcript (message list) has focus.
    Transcript,
}

impl FocusCtx {
    pub(crate) fn cycle(self) -> Self {
        match self {
            Self::Composer => Self::Transcript,
            Self::Transcript => Self::Composer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle() {
        assert_eq!(FocusCtx::Composer.cycle(), FocusCtx::Transcript);
        assert_eq!(FocusCtx::Transcript.cycle(), FocusCtx::Composer);
    }
}
