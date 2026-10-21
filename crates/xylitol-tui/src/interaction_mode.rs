#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    #[default]
    Inline,
    ApplicationOwned,
}

impl InteractionMode {
    pub const fn is_application_owned(self) -> bool {
        matches!(self, Self::ApplicationOwned)
    }

    pub const fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_inline() {
        assert_eq!(InteractionMode::default(), InteractionMode::Inline);
        assert!(InteractionMode::Inline.is_inline());
        assert!(!InteractionMode::ApplicationOwned.is_inline());
    }
}
