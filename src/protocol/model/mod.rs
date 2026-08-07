//! Model vocabulary: config, streaming chunks, thinking, metadata.

pub mod chunk;
pub mod config;
pub mod meta;
pub mod thinking;

pub use chunk::{XyChunk, XyToolSchema};
pub use config::{ResolvedProfile, XyModelConfig, XyModelKind, default_context_window_for};
pub use meta::{ContextTokenEstimate, TokenProvenance, XyModelMeta};
pub use thinking::{
    ResolvedThinking, ThinkingAdapterKind, ThinkingBudgets, ThinkingLevel, ThinkingLevelMap,
    resolve_thinking_for_request, validate_thinking_level_map,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::{XyStopReason, XyUsage};

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
    fn xy_chunk_tool_call_end() {
        let chunk = XyChunk::ToolCallEnd {
            name: "read_file".into(),
            args: serde_json::json!({}),
            id: "call-1".into(),
        };
        match chunk {
            XyChunk::ToolCallEnd { name, id, .. } => {
                assert_eq!(name, "read_file");
                assert_eq!(id, "call-1");
            }
            _ => panic!("expected ToolCallEnd"),
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
            total_tokens: 150,
            ..Default::default()
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
        let config = crate::protocol::model::config::XyModelConfig {
            kind: crate::protocol::model::config::XyModelKind::Anthropic,
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
            thinking_level_map: std::collections::HashMap::new(),
        };
        assert_eq!(meta.id, "claude-3");
        assert!(meta.thinking);
        assert_eq!(meta.context_window, 200_000);
        assert_eq!(meta.max_tokens, 8192);
    }

    // ── resolve_thinking_for_request ─────────────────────────────────

    #[test]
    fn resolve_openai_absent_key_uses_identity() {
        let r = resolve_thinking_for_request(
            ThinkingLevel::Medium,
            None,
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::OpenAiEffort("medium".into()));
    }

    #[test]
    fn resolve_openai_off_omits() {
        let r = resolve_thinking_for_request(
            ThinkingLevel::Off,
            None,
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::Omit);
    }

    #[test]
    fn resolve_null_omits() {
        let mut map = ThinkingLevelMap::new();
        map.insert("high".into(), None);
        let r = resolve_thinking_for_request(
            ThinkingLevel::High,
            Some(&map),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::Omit);
    }

    #[test]
    fn resolve_map_overrides_default() {
        let mut map = ThinkingLevelMap::new();
        map.insert("high".into(), Some("max".into()));
        let r = resolve_thinking_for_request(
            ThinkingLevel::High,
            Some(&map),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::OpenAiEffort("max".into()));
    }

    #[test]
    fn resolve_anthropic_budget_defaults_and_settings() {
        let r = resolve_thinking_for_request(
            ThinkingLevel::Low,
            None,
            None,
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r, ResolvedThinking::AnthropicBudget(2048));

        let budgets = ThinkingBudgets {
            low: Some(4096),
            ..Default::default()
        };
        let r2 = resolve_thinking_for_request(
            ThinkingLevel::Low,
            None,
            Some(&budgets),
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r2, ResolvedThinking::AnthropicBudget(4096));
    }

    #[test]
    fn validate_thinking_level_map_rejects_unknown() {
        let mut map = ThinkingLevelMap::new();
        map.insert("bogon".into(), Some("x".into()));
        assert!(
            validate_thinking_level_map(&map)
                .unwrap_err()
                .contains("bogon")
        );
    }
}
