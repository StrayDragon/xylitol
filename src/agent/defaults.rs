//! Centralized default values for the agent runtime.
//!
//! Aligns with pi's defaults.ts. Provides canonical defaults for
//! thinking level, max iterations, compaction threshold, and model defaults.

#![allow(dead_code)]
use crate::agent::types::ThinkingLevel;

/// Default thinking level when none is explicitly configured.
pub(crate) const DEFAULT_THINKING_LEVEL: ThinkingLevel = ThinkingLevel::Medium;

/// Default maximum ReAct loop iterations per turn.
pub(crate) const DEFAULT_MAX_ITERATIONS: u32 = 50;

/// Default compaction threshold (0.0-1.0).
///
/// When estimated token usage exceeds this fraction of the context window,
/// auto-compaction is triggered.
pub(crate) const DEFAULT_COMPACTION_THRESHOLD: f64 = 0.8;

/// Default model ID for each known provider (mirrors pi's defaultModelPerProvider).
pub(crate) fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-5.4",
        "anthropic" => "claude-opus-4-8",
        "amazon-bedrock" => "us.anthropic.claude-opus-4-6-v1",
        "ant-ling" => "Ring-2.6-1T",
        "azure-openai-responses" => "gpt-5.4",
        "openai-codex" => "gpt-5.5",
        "nvidia" => "nvidia/nemotron-3-super-120b-a12b",
        "deepseek" => "deepseek-v4-pro",
        "google" => "gemini-3.1-pro-preview",
        "google-vertex" => "gemini-3.1-pro-preview",
        "github-copilot" => "gpt-5.4",
        "openrouter" => "moonshotai/kimi-k2.6",
        "vercel-ai-gateway" => "zai/glm-5.1",
        "xai" => "grok-4.20-0309-reasoning",
        "groq" => "openai/gpt-oss-120b",
        "cerebras" => "zai-glm-4.7",
        "zai" => "glm-5.1",
        "zai-coding-cn" => "glm-5.1",
        "mistral" => "devstral-medium-latest",
        "minimax" => "MiniMax-M2.7",
        "minimax-cn" => "MiniMax-M2.7",
        "moonshotai" => "kimi-k2.6",
        "moonshotai-cn" => "kimi-k2.6",
        "huggingface" => "moonshotai/Kimi-K2.6",
        "fireworks" => "accounts/fireworks/models/kimi-k2p6",
        "together" => "moonshotai/Kimi-K2.6",
        "opencode" => "kimi-k2.6",
        "opencode-go" => "kimi-k2.6",
        "kimi-coding" => "kimi-for-coding",
        "cloudflare-workers-ai" => "@cf/moonshotai/kimi-k2.6",
        "cloudflare-ai-gateway" => "workers-ai/@cf/moonshotai/kimi-k2.6",
        "xiaomi" => "mimo-v2.5-pro",
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
        assert_eq!(default_model_for_provider("openai"), "gpt-5.4");
        assert_eq!(default_model_for_provider("anthropic"), "claude-opus-4-8");
        assert_eq!(default_model_for_provider("deepseek"), "deepseek-v4-pro");
        assert_eq!(default_model_for_provider("xiaomi"), "mimo-v2.5-pro");
        assert_eq!(default_model_for_provider("unknown"), "unknown");
    }
}
