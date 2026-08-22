//! Tool / executor wall-clock timeout with explicit unlimited semantics.

use std::time::Duration;

/// Hard ceiling for model-requested tool timeouts (seconds), aligned with
/// [`crate::MAX_EXTERNAL_WAIT_SECS`]. Requests above it are clamped, not
/// rejected (c2430 program authority).
pub const MAX_TOOL_TIMEOUT_SECS: u64 = 600;

/// Global fallback ceiling for ANY external wait the agent can trigger
/// (c2425, program authority). Even where configuration may raise a
/// per-tool/per-channel default, no armed wait may exceed this bound.
pub const MAX_EXTERNAL_WAIT_SECS: u64 = 600;

/// Wall-clock default applied when a tool invocation omits its timeout.
///
/// Product contract (c2425): omitting `timeout` no longer means unlimited —
/// the tool layer maps an omitted limit to a per-tool bound (see each tool's
/// constant) so every external wait is bounded. Long legitimate work must
/// raise the explicit value instead, up to the global 600s ceiling.
pub const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 120;

/// Wall-clock limit for a tool or hook invocation.
///
/// [`Self::Unlimited`] is the parser-level "omitted" marker; the product
/// tool layer maps it to a per-tool default via [`Self::or_default`] and
/// caps requests with [`Self::clamped`] — an armed timer is guaranteed.
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
    /// * positive values accepted at any magnitude — callers clamp with
    ///   [`Self::clamped`] to [`MAX_TOOL_TIMEOUT_SECS`] (c2430)
    pub fn from_secs_opt(secs: Option<u64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(0) => Err(ToolTimeoutError::ZeroOrNegative),
            Some(s) => Ok(Self::After(Duration::from_secs(s))),
        }
    }

    /// Parse optional signed seconds (LLM JSON may use negative integers).
    ///
    /// * `None` → [`Self::Unlimited`]
    /// * `Some(0)` / negative → error (not unlimited)
    /// * positive values are accepted at any magnitude — the tool layer
    ///   clamps them with [`Self::clamped`] to [`MAX_TOOL_TIMEOUT_SECS`]
    pub fn from_i64_opt(secs: Option<i64>) -> Result<Self, ToolTimeoutError> {
        match secs {
            None => Ok(Self::Unlimited),
            Some(n) if n <= 0 => Err(ToolTimeoutError::ZeroOrNegative),
            Some(n) => Ok(Self::After(Duration::from_secs(n as u64))),
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

    /// Replace [`Self::Unlimited`] with a concrete bound.
    ///
    /// Used at the product tool layer to guarantee bounded waits: an omitted
    /// timeout becomes [`DEFAULT_TOOL_TIMEOUT_SECS`]-style bound while an
    /// explicit value passes through unchanged.
    pub fn or_default(self, default_secs: u64) -> Self {
        if self.is_unlimited() {
            Self::After(Duration::from_secs(default_secs))
        } else {
            self
        }
    }

    /// Cap a concrete bound at [`MAX_TOOL_TIMEOUT_SECS`] (program authority,
    /// c2430): model-requested magnitudes are clamped, never rejected.
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
        // positive magnitudes beyond the ceiling are accepted and clamped later
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
        // c2430: model-requested magnitudes are clamped, never rejected.
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
