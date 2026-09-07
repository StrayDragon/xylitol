//! Model vocabulary: config, streaming chunks, thinking, metadata.

pub mod chunk;
pub mod config;
pub mod meta;
pub mod thinking;

pub use chunk::{XyChunk, XyToolSchema};
pub use config::{ResolvedProfile, XyModelConfig, XyModelKind, default_context_window_for};
pub use meta::{ContextTokenEstimate, TokenProvenance, XyModelMeta};
pub use thinking::{
    BuiltinThinkingLevels, THINKING_OFF, ThinkingBudgets, ThinkingConfigError, ThinkingLevelMap,
    last_declared_thinking_level, resolve_configured_levels, thinking_levels_are_adjustable,
    validate_thinking_level_map,
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

    #[test]
    fn configured_levels_are_freeform_ordered_and_default_to_off() {
        assert_eq!(
            resolve_configured_levels(true, None).unwrap(),
            vec!["off".to_string()]
        );
        assert_eq!(
            resolve_configured_levels(true, Some(&[])).unwrap(),
            vec!["off".to_string()]
        );
        let declared = resolve_configured_levels(
            true,
            Some(&["off".into(), "bogon-level".into(), "max".into()]),
        )
        .unwrap();
        assert_eq!(declared, vec!["off", "bogon-level", "max"]);
        assert!(thinking_levels_are_adjustable(&declared));
        assert!(thinking_levels_are_adjustable(&["OFF".into()]));
        assert!(!thinking_levels_are_adjustable(&["off".into()]));
        assert!(thinking_levels_are_adjustable(&[
            "off".into(),
            "bogon-level".into()
        ]));
        assert_eq!(last_declared_thinking_level(&declared), "max");
        assert!(resolve_configured_levels(true, Some(&[" \t".into()])).is_err());
        assert_eq!(
            resolve_configured_levels(false, Some(&["high".into()])).unwrap(),
            vec!["off".to_string()]
        );
        assert!(resolve_configured_levels(false, Some(&["".into()])).is_err());
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
            compat: None,
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

    #[test]
    fn validate_thinking_level_map_requires_declared_key() {
        let mut map = ThinkingLevelMap::new();
        map.insert("bogon".into(), Some("x".into()));
        assert!(
            validate_thinking_level_map(&map, &["off".into(), "high".into()])
                .unwrap_err()
                .to_string()
                .contains("bogon")
        );
    }
}
