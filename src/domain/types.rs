//! Core data types — streaming chunks, thinking levels, model metadata, tool schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::message::{XyStopReason, XyUsage};
use crate::domain::model::XyModelConfig;

// ── Streaming Chunk ──────────────────────────────────────────────

/// A single chunk from a streaming LLM response.
#[derive(Debug, Clone)]
pub enum XyChunk {
    TextDelta(String),
    ThinkingDelta(String),
    FunctionCall {
        name: String,
        args: Value,
        id: String,
    },
    Done {
        finish_reason: XyStopReason,
        usage: Option<XyUsage>,
    },
}

/// JSON schema describing a tool's parameters.
#[derive(Debug, Clone, Serialize)]
pub struct XyToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Where a context-token estimate came from (c1030 accounting provenance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProvenance {
    Api,
    RemoteCount,
    LocalTokenizer,
    Heuristic,
    Unknown,
}

/// Context occupancy estimate with provenance (Driver / compaction seam).
#[derive(Debug, Clone)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

// ── Thinking Level ──────────────────────────────────────────────────

/// How much "thinking" / chain-of-thought the model should expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
    Max,
}

impl ThinkingLevel {
    /// Default levels when `thinking: true` and no explicit list is configured.
    pub const STANDARD: &'static [ThinkingLevel] = &[
        ThinkingLevel::Off,
        ThinkingLevel::Minimal,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::Xhigh => "xhigh",
            ThinkingLevel::Max => "max",
        }
    }

    /// Parse a level name (case-insensitive). Unknown → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Some(ThinkingLevel::Off),
            "minimal" => Some(ThinkingLevel::Minimal),
            "low" => Some(ThinkingLevel::Low),
            "medium" => Some(ThinkingLevel::Medium),
            "high" => Some(ThinkingLevel::High),
            "xhigh" => Some(ThinkingLevel::Xhigh),
            "max" => Some(ThinkingLevel::Max),
            _ => None,
        }
    }

    /// Clamp to what the model supports (boolean capability only).
    pub fn clamp(self, model_supports_thinking: bool) -> Self {
        if !model_supports_thinking {
            return ThinkingLevel::Off;
        }
        self
    }

    /// Resolve configured level names into a support list.
    ///
    /// - `thinking == false` → `[Off]`
    /// - `thinking == true` and empty/missing list → [`Self::STANDARD`]
    /// - explicit list → parsed in order (holes allowed); unknown names → `Err`
    pub fn resolve_configured_levels(
        thinking: bool,
        configured: Option<&[String]>,
    ) -> Result<Vec<ThinkingLevel>, String> {
        if !thinking {
            return Ok(vec![ThinkingLevel::Off]);
        }
        let Some(raw) = configured.filter(|v| !v.is_empty()) else {
            return Ok(Self::STANDARD.to_vec());
        };
        let mut out = Vec::with_capacity(raw.len());
        for name in raw {
            let Some(level) = Self::parse(name) else {
                return Err(format!("unknown thinking level: {name}"));
            };
            out.push(level);
        }
        Ok(out)
    }

    /// Pick a legal level when the current one is unsupported.
    pub fn clamp_to_supported(
        current: ThinkingLevel,
        supported: &[ThinkingLevel],
        preferred_default: Option<ThinkingLevel>,
    ) -> ThinkingLevel {
        if supported.is_empty() {
            return ThinkingLevel::Off;
        }
        if supported.contains(&current) {
            return current;
        }
        if let Some(d) = preferred_default
            && supported.contains(&d)
        {
            return d;
        }
        if supported.contains(&ThinkingLevel::Medium) {
            return ThinkingLevel::Medium;
        }
        supported
            .iter()
            .copied()
            .rev()
            .find(|l| *l != ThinkingLevel::Off)
            .unwrap_or(supported[0])
    }
}

// ── Model Metadata ──────────────────────────────────────────────────

/// Metadata describing a model variant available from a provider.
#[derive(Debug, Clone)]
pub struct XyModelMeta {
    pub id: String,
    pub config: XyModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
    pub api: String,
    pub provider: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub cost_cache_read: f64,
    pub cost_cache_write: f64,
    pub max_tokens: u64,
    pub thinking_levels: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::XyUsage;

    // ── XyChunk ─────────────────────────────────────────────────────

    #[test]
    fn xy_chunk_text_delta() {
        let chunk = XyChunk::TextDelta("hello".into());
        match chunk {
            XyChunk::TextDelta(t) => assert_eq!(t, "hello"),
            _ => panic!("expected TextDelta"),
        }
    }

    #[test]
    fn xy_chunk_thinking_delta() {
        let chunk = XyChunk::ThinkingDelta("reasoning...".into());
        match chunk {
            XyChunk::ThinkingDelta(t) => assert_eq!(t, "reasoning..."),
            _ => panic!("expected ThinkingDelta"),
        }
    }

    #[test]
    fn xy_chunk_function_call() {
        let chunk = XyChunk::FunctionCall {
            name: "read_file".into(),
            args: serde_json::json!({}),
            id: "call-1".into(),
        };
        match chunk {
            XyChunk::FunctionCall { name, id, .. } => {
                assert_eq!(name, "read_file");
                assert_eq!(id, "call-1");
            }
            _ => panic!("expected FunctionCall"),
        }
    }

    #[test]
    fn xy_chunk_done_stop() {
        let chunk = XyChunk::Done {
            finish_reason: XyStopReason::Stop,
            usage: None,
        };
        match chunk {
            XyChunk::Done {
                finish_reason,
                usage,
                ..
            } => {
                assert_eq!(finish_reason, XyStopReason::Stop);
                assert!(usage.is_none());
            }
            _ => panic!("expected Done"),
        }
    }

    #[test]
    fn xy_chunk_done_max_tokens() {
        let usage = XyUsage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 150,
            cost: None,
        };
        let chunk = XyChunk::Done {
            finish_reason: XyStopReason::MaxTokens,
            usage: Some(usage),
        };
        match chunk {
            XyChunk::Done {
                finish_reason,
                usage: Some(u),
                ..
            } => {
                assert_eq!(finish_reason, XyStopReason::MaxTokens);
                assert_eq!(u.total_tokens, 150);
            }
            _ => panic!("expected Done with usage"),
        }
    }

    // ── ThinkingLevel ───────────────────────────────────────────────

    #[test]
    fn thinking_level_default_is_medium() {
        assert_eq!(ThinkingLevel::default(), ThinkingLevel::Medium);
    }

    #[test]
    fn thinking_level_as_str() {
        assert_eq!(ThinkingLevel::Off.as_str(), "off");
        assert_eq!(ThinkingLevel::Minimal.as_str(), "minimal");
        assert_eq!(ThinkingLevel::Low.as_str(), "low");
        assert_eq!(ThinkingLevel::Medium.as_str(), "medium");
        assert_eq!(ThinkingLevel::High.as_str(), "high");
        assert_eq!(ThinkingLevel::Xhigh.as_str(), "xhigh");
        assert_eq!(ThinkingLevel::Max.as_str(), "max");
    }

    #[test]
    fn thinking_level_parse_covers_all_and_rejects_unknown() {
        assert_eq!(ThinkingLevel::parse("xhigh"), Some(ThinkingLevel::Xhigh));
        assert_eq!(ThinkingLevel::parse("MAX"), Some(ThinkingLevel::Max));
        assert_eq!(ThinkingLevel::parse("bogon"), None);
    }

    #[test]
    fn thinking_level_serde_round_trip() {
        let levels = [
            ThinkingLevel::Off,
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::Xhigh,
            ThinkingLevel::Max,
        ];
        for level in &levels {
            let json = serde_json::to_string(level).unwrap();
            let deserialized: ThinkingLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(*level, deserialized);
        }
    }

    #[test]
    fn thinking_level_resolve_configured_defaults_and_holes() {
        let std = ThinkingLevel::resolve_configured_levels(true, None).unwrap();
        assert_eq!(std, ThinkingLevel::STANDARD.to_vec());
        assert!(!std.contains(&ThinkingLevel::Xhigh));

        let hole =
            ThinkingLevel::resolve_configured_levels(true, Some(&["high".into(), "max".into()]))
                .unwrap();
        assert_eq!(hole, vec![ThinkingLevel::High, ThinkingLevel::Max]);

        assert!(
            ThinkingLevel::resolve_configured_levels(true, Some(&["bogon".into()]))
                .unwrap_err()
                .contains("unknown")
        );

        assert_eq!(
            ThinkingLevel::resolve_configured_levels(false, Some(&["high".into()])).unwrap(),
            vec![ThinkingLevel::Off]
        );
    }

    #[test]
    fn thinking_level_clamp_off_when_no_thinking() {
        assert_eq!(ThinkingLevel::High.clamp(false), ThinkingLevel::Off);
        assert_eq!(ThinkingLevel::Medium.clamp(false), ThinkingLevel::Off);
        assert_eq!(ThinkingLevel::Off.clamp(false), ThinkingLevel::Off);
    }

    #[test]
    fn thinking_level_passthrough_when_supported() {
        assert_eq!(ThinkingLevel::High.clamp(true), ThinkingLevel::High);
        assert_eq!(ThinkingLevel::Medium.clamp(true), ThinkingLevel::Medium);
        assert_eq!(ThinkingLevel::Off.clamp(true), ThinkingLevel::Off);
    }

    #[test]
    fn thinking_level_serde_lowercase() {
        let json = serde_json::to_string(&ThinkingLevel::Medium).unwrap();
        assert_eq!(json, "\"medium\"");
        let deserialized: ThinkingLevel = serde_json::from_str("\"high\"").unwrap();
        assert_eq!(deserialized, ThinkingLevel::High);
    }

    // ── XyToolSchema ────────────────────────────────────────────────

    #[test]
    fn xy_tool_schema_construct() {
        let schema = XyToolSchema {
            name: "read_file".into(),
            description: "Read a file".into(),
            parameters: serde_json::json!({"type": "object"}),
        };
        assert_eq!(schema.name, "read_file");
        assert_eq!(schema.description, "Read a file");
        assert_eq!(schema.parameters["type"], "object");
    }

    // ── XyModelMeta ───────────────────────────────────────────────────

    #[test]
    fn model_meta_construct() {
        let config = crate::domain::model::XyModelConfig {
            kind: crate::domain::model::XyModelKind::Anthropic,
            api_key: "sk-test".into(),
            model: "claude-3".into(),
            base_url: None,
            api: None,
        };
        let meta = XyModelMeta {
            id: "claude-3".into(),
            config,
            display_name: "Claude 3".into(),
            thinking: true,
            context_window: 200_000,
            api: "anthropic".into(),
            provider: "anthropic".into(),
            cost_input: 3.0,
            cost_output: 15.0,
            cost_cache_read: 0.3,
            cost_cache_write: 3.75,
            max_tokens: 8192,
            thinking_levels: vec!["low".into(), "medium".into(), "high".into()],
        };
        assert_eq!(meta.id, "claude-3");
        assert!(meta.thinking);
        assert_eq!(meta.context_window, 200_000);
        assert_eq!(meta.max_tokens, 8192);
    }
}
