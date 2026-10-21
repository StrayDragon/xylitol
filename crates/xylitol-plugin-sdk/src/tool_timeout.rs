use std::time::Duration;

pub const MAX_TOOL_TIMEOUT_SECS: u64 = 600;

pub const MAX_EXTERNAL_WAIT_SECS: u64 = 600;

pub const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolTimeout {
    #[default]
    Unlimited,
    After(Duration),
}

impl ToolTimeout {
    pub fn from_secs_opt(secs: Option<u64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(0) => Err(ToolTimeoutError::ZeroOrNegative),
            Some(s) => Ok(Self::After(Duration::from_secs(s))),
        }
    }

    pub fn from_i64_opt(secs: Option<i64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(n) if n <= 0 => Err(ToolTimeoutError::ZeroOrNegative),
            Some(n) => Ok(Self::After(Duration::from_secs(n as u64))),
        }
    }

    pub fn duration(self) -> Option<Duration> {
        match self {
            Self::Unlimited => None,
            Self::After(d) => Some(d),
        }
    }

    pub fn is_unlimited(self) -> bool {
        matches!(self, Self::Unlimited)
    }

    pub fn or_default(self, default_secs: u64) -> Self {
        if self.is_unlimited() {
            Self::After(Duration::from_secs(default_secs))
        } else {
            self
        }
    }

    pub fn clamped(self) -> Self {
        match self {
            Self::After(d) if d > Duration::from_secs(MAX_TOOL_TIMEOUT_SECS) => {
                Self::After(Duration::from_secs(MAX_TOOL_TIMEOUT_SECS))
            }
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolTimeoutError {
    ZeroOrNegative,
}

impl std::fmt::Display for ToolTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroOrNegative => write!(
                f,
                "invalid timeout: must be a positive number of seconds \
                 (values above the {MAX_TOOL_TIMEOUT_SECS}s ceiling are clamped)"
            ),
        }
    }
}

impl std::error::Error for ToolTimeoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_unlimited() {
        assert_eq!(
            ToolTimeout::from_secs_opt(None).unwrap(),
            ToolTimeout::Unlimited
        );
        assert!(ToolTimeout::Unlimited.is_unlimited());
        assert_eq!(ToolTimeout::Unlimited.duration(), None);
    }

    #[test]
    fn zero_and_negative_rejected() {
        assert_eq!(
            ToolTimeout::from_secs_opt(Some(0)).unwrap_err(),
            ToolTimeoutError::ZeroOrNegative
        );
        assert_eq!(
            ToolTimeout::from_i64_opt(Some(-1)).unwrap_err(),
            ToolTimeoutError::ZeroOrNegative
        );
        assert_eq!(
            ToolTimeout::from_i64_opt(Some(0)).unwrap_err(),
            ToolTimeoutError::ZeroOrNegative
        );
    }

    #[test]
    fn positive_ok_and_above_max_err() {
        assert_eq!(
            ToolTimeout::from_secs_opt(Some(1)).unwrap(),
            ToolTimeout::After(Duration::from_secs(1))
        );
        assert!(matches!(
            ToolTimeout::from_secs_opt(Some(MAX_TOOL_TIMEOUT_SECS + 1)),
            Ok(ToolTimeout::After(_))
        ));
    }

    #[test]
    fn or_default_bounds_only_unlimited() {
        assert_eq!(
            ToolTimeout::Unlimited.or_default(DEFAULT_TOOL_TIMEOUT_SECS),
            ToolTimeout::After(Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS))
        );
        let explicit = ToolTimeout::After(Duration::from_secs(1));
        assert_eq!(explicit.or_default(DEFAULT_TOOL_TIMEOUT_SECS), explicit);
    }

    #[test]
    fn i64_accepts_any_positive_then_clamps_to_ceiling() {
        assert_eq!(
            ToolTimeout::from_i64_opt(Some(600)).unwrap(),
            ToolTimeout::After(Duration::from_secs(600))
        );
        assert_eq!(
            ToolTimeout::from_i64_opt(Some(3_600)).unwrap().clamped(),
            ToolTimeout::After(Duration::from_secs(MAX_TOOL_TIMEOUT_SECS))
        );
        assert_eq!(
            ToolTimeout::Unlimited.or_default(120).clamped(),
            ToolTimeout::After(Duration::from_secs(120))
        );
    }
}
