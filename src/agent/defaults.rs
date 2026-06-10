//! Centralized default values for the agent runtime.
//!
//! Aligns with pi's defaults.ts. Provides canonical defaults for
//! thinking level, max iterations, compaction threshold, and model defaults.

use crate::agent::session::ThinkingLevel;

/// Default thinking level when none is explicitly configured.
pub const DEFAULT_THINKING_LEVEL: ThinkingLevel = ThinkingLevel::Medium;

/// Default maximum ReAct loop iterations per turn.
pub const DEFAULT_MAX_ITERATIONS: u32 = 50;

/// Default compaction threshold (0.0-1.0).
///
/// When estimated token usage exceeds this fraction of the context window,
/// auto-compaction is triggered.
pub const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

/// Default model ID per known provider.
pub const DEFAULT_MODEL_OPENAI: &str = "gpt-4o";
pub const DEFAULT_MODEL_ANTHROPIC: &str = "claude-sonnet-4-20250514";

/// Get the default model ID for a provider name.
pub fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "openai" => DEFAULT_MODEL_OPENAI,
        "anthropic" => DEFAULT_MODEL_ANTHROPIC,
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_thinking_level() {
        assert_eq!(DEFAULT_THINKING_LEVEL, ThinkingLevel::Medium);
    }

    #[test]
    fn test_default_max_iterations() {
        assert_eq!(DEFAULT_MAX_ITERATIONS, 50);
    }

    #[test]
    fn test_default_compaction_threshold() {
        assert!((DEFAULT_COMPACTION_THRESHOLD - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_model_for_provider() {
        assert_eq!(default_model_for_provider("openai"), "gpt-4o");
        assert_eq!(
            default_model_for_provider("anthropic"),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(default_model_for_provider("unknown"), "unknown");
    }
}
