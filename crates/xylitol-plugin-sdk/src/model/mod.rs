pub mod chunk;
pub mod config;
pub mod meta;
pub mod thinking;

pub use chunk::{XyChunk, XyToolSchema};
pub use config::{
    ResolvedProfile, XyModality, XyModelConfig, XyModelKind, XyPromptCacheConfig,
    XyPromptCacheRetention, XyPromptCacheTtl, default_context_window_for, default_input_modalities,
};
pub use meta::{ContextTokenEstimate, TokenProvenance, XyModelMeta};
pub use thinking::{
    CANONICAL_THINKING_LEVELS, THINKING_OFF, ThinkingBudgets, ThinkingConfigError,
    ThinkingLevelMap, last_declared_thinking_level, resolve_levels_from_map,
    thinking_levels_are_adjustable, validate_thinking_level_map,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{XyStopReason, XyUsage};

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
    fn map_keys_are_levels_in_canonical_order() {
        let mut map = ThinkingLevelMap::new();
        map.insert("bogon-level".into(), Some("high".into()));
        map.insert("max".into(), Some("max".into()));
        map.insert("off".into(), None);
        let levels = resolve_levels_from_map(true, &map).unwrap();
        assert_eq!(levels, vec!["off", "max", "bogon-level"]);
        assert!(thinking_levels_are_adjustable(&levels));
        assert!(thinking_levels_are_adjustable(&["OFF".into()]));
        assert!(!thinking_levels_are_adjustable(&["off".into()]));
        assert_eq!(last_declared_thinking_level(&levels), "bogon-level");

        assert_eq!(
            resolve_levels_from_map(true, &ThinkingLevelMap::new()).unwrap(),
            vec!["off".to_string()]
        );

        let mut off_only = ThinkingLevelMap::new();
        off_only.insert("off".into(), None);
        assert_eq!(
            resolve_levels_from_map(false, &off_only).unwrap(),
            vec!["off".to_string()]
        );
        assert_eq!(
            resolve_levels_from_map(false, &map).unwrap(),
            vec!["off".to_string()]
        );

        let mut blank_key = ThinkingLevelMap::new();
        blank_key.insert(" \t".into(), None);
        assert!(resolve_levels_from_map(true, &blank_key).is_err());

        let mut blank_wire = ThinkingLevelMap::new();
        blank_wire.insert("high".into(), Some(" ".into()));
        assert!(resolve_levels_from_map(true, &blank_wire).is_err());
    }

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

    #[test]
    fn model_meta_construct() {
        let config = crate::model::config::XyModelConfig {
            kind: crate::model::config::XyModelKind::Anthropic,
            api_key: "sk-test".into(),
            model: "claude-3".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
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
    fn validate_thinking_level_map_rejects_blank_entries() {
        let mut blank_key = ThinkingLevelMap::new();
        blank_key.insert(String::new(), Some("x".into()));
        assert!(validate_thinking_level_map(&blank_key).is_err());

        let mut blank_wire = ThinkingLevelMap::new();
        blank_wire.insert("high".into(), Some(String::new()));
        assert!(validate_thinking_level_map(&blank_wire).is_err());

        let mut null_wire = ThinkingLevelMap::new();
        null_wire.insert("off".into(), None);
        assert!(validate_thinking_level_map(&null_wire).is_ok());
    }
}
