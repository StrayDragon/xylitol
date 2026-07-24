//! Tool / executor wall-clock timeout with explicit unlimited semantics.

use std::time::Duration;

/// Maximum allowed explicit tool timeout (seconds). Orthogonal to the default
/// of [`ToolTimeout::Unlimited`].
pub const MAX_TOOL_TIMEOUT_SECS: u64 = 120;

/// Wall-clock limit for a tool or hook invocation.
///
/// Omitted / [`Self::Unlimited`] means no timer is armed (cancel tokens still apply).
/// Never encode unlimited as `0` — zero is invalid when a limit is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolTimeout {
    /// No wall-clock limit.
    #[default]
    Unlimited,
    /// Kill / fail after this duration.
    After(Duration),
}

impl ToolTimeout {
    /// Parse an optional seconds value from tool args or config.
    ///
    /// * `None` → [`Self::Unlimited`]
    /// * `Some(0)` / negative → error (not unlimited)
    /// * above [`MAX_TOOL_TIMEOUT_SECS`] → error
    pub fn from_secs_opt(secs: Option<u64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(0) => Err(ToolTimeoutError::ZeroOrNegative),
            Some(s) if s > MAX_TOOL_TIMEOUT_SECS => Err(ToolTimeoutError::AboveMax {
                got: s,
                max: MAX_TOOL_TIMEOUT_SECS,
            }),
            Some(s) => Ok(Self::After(Duration::from_secs(s))),
        }
    }

    /// Parse optional signed seconds (LLM JSON may use negative integers).
    pub fn from_i64_opt(secs: Option<i64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(n) if n <= 0 => Err(ToolTimeoutError::ZeroOrNegative),
            Some(n) => Self::from_secs_opt(Some(n as u64)),
        }
    }

    /// Duration when limited; `None` when unlimited.
    pub fn duration(self) -> Option<Duration> {
        match self {
            Self::Unlimited => None,
            Self::After(d) => Some(d),
        }
    }

    pub fn is_unlimited(self) -> bool {
        matches!(self, Self::Unlimited)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolTimeoutError {
    ZeroOrNegative,
    AboveMax { got: u64, max: u64 },
}

impl std::fmt::Display for ToolTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroOrNegative => {
                write!(
                    f,
                    "invalid timeout: must be a positive number of seconds (omit for unlimited; 0 is not unlimited)"
                )
            }
            Self::AboveMax { got, max } => {
                write!(f, "invalid timeout: {got}s exceeds maximum of {max}s")
            }
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
            Err(ToolTimeoutError::AboveMax { .. })
        ));
    }
}
