//! Typed agent message types — session SSOT with LLM / Env composition (c1210).
//!
//! [`AgentMessage`] = `Llm(`[ `LlmMessage` ]`) | Env(`[ `EnvMessage` ]`)`.
//! [`LlmMessage`] is a `pub use` alias of bridge `AiBridgeMessage` (wire-compatible).

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Bridge LLM leaf aliases (minimize call-site churn) ──────────────

pub use xylitol_ai_bridge::dto::{
    AiBridgeImageContent as ImageContent, AiBridgeMessage as LlmMessage, AiBridgePart as AgentPart,
    AiBridgeStopReason as XyStopReason, AiBridgeUsage as XyUsage, now_ms,
};
#[cfg(test)]
pub use xylitol_ai_bridge::dto::{AiBridgeUsageCost as XyUsageCost, Diagnostic};

// ── EnvMessage / AgentMessage (domain composition) ─────────────────

/// Lifecycle of an interactive bang run (c2760).
///
/// `Running` rows are start markers (command only); `Done` rows carry the
/// finished result. Old JSONL rows without the field default to `Done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BashExecutionStatus {
    Running,
    #[default]
    Done,
}

/// Environment / session meta roles (not sent to the model as-is).
///
/// Wire fields are camelCase (`excludeFromContext`, `tokensBefore`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum EnvMessage {
    #[serde(rename = "bashExecution")]
    #[serde(rename_all = "camelCase")]
    BashExecutionMessage {
        /// Correlation id linking the running start row to its done row (c2760).
        #[serde(default)]
        bash_id: String,
        command: String,
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(default)]
        cancelled: bool,
        #[serde(default)]
        truncated: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full_output_path: Option<String>,
        #[serde(default)]
        exclude_from_context: bool,
        /// Lifecycle status (c2760); missing = Done for old rows.
        #[serde(default)]
        status: BashExecutionStatus,
    },
    #[serde(rename = "custom")]
    #[serde(rename_all = "camelCase")]
    CustomMessage {
        custom_type: String,
        content: Value,
        #[serde(default)]
        display: Value,
        #[serde(default)]
        details: Value,
    },
    #[serde(rename = "compactionSummary")]
    #[serde(rename_all = "camelCase")]
    CompactionSummaryMessage {
        summary: String,
        tokens_before: u64,
        tokens_after: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        read_files: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_files: Option<Vec<String>>,
    },
    #[serde(rename = "branchSummary")]
    #[serde(rename_all = "camelCase")]
    BranchSummaryMessage { summary: String, from_id: String },
}

/// Structured LLM-visible projection of an [`EnvMessage`] row (c2725).
///
/// `None` from [`EnvMessage::llm_projection`] means the row must not reach the
/// model (excluded bash / empty custom). Adding an `EnvMessage` variant forces
/// updating this enum's consumers — the fold shape is the prompt-cache prefix
/// contract (c1930 / as48), so text templates stay in `agent::llm_project`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvLlmProjection {
    Bash { command: String, output: String },
    ContextSummary { summary: String },
    CustomText { text: String },
}

impl EnvMessage {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::BashExecutionMessage { .. } => "bashExecution",
            Self::CustomMessage { .. } => "custom",
            Self::CompactionSummaryMessage { .. } => "compactionSummary",
            Self::BranchSummaryMessage { .. } => "branchSummary",
        }
    }

    /// LLM-visible projection of this row; `None` = do not send to model.
    ///
    /// Single source of truth for the *whether / what* of env→LLM folding
    /// (c2725): bash exclusion, compaction/branch summary, custom text.
    pub fn llm_projection(&self) -> Option<EnvLlmProjection> {
        match self {
            Self::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                status,
                ..
            } => {
                // c2760: a running start row never projects to the model —
                // only the finished (done) row is part of the stable prefix.
                if *exclude_from_context || *status == BashExecutionStatus::Running {
                    None
                } else {
                    Some(EnvLlmProjection::Bash {
                        command: command.clone(),
                        output: output.clone(),
                    })
                }
            }
            Self::CompactionSummaryMessage { summary, .. }
            | Self::BranchSummaryMessage { summary, .. } => {
                Some(EnvLlmProjection::ContextSummary {
                    summary: summary.clone(),
                })
            }
            Self::CustomMessage { content, .. } => {
                custom_projection_text(content).map(|text| EnvLlmProjection::CustomText { text })
            }
        }
    }

    /// Whether this row must be excluded from context windows / token math.
    ///
    /// Only bash carries a per-row flag; other variants are never excluded.
    pub fn exclude_from_context(&self) -> bool {
        match self {
            Self::BashExecutionMessage {
                exclude_from_context,
                ..
            } => *exclude_from_context,
            _ => false,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::BashExecutionMessage {
                command, output, ..
            } => format!("$ {command}\n{output}"),
            Self::CustomMessage { content, .. } => content.as_str().unwrap_or("").to_string(),
            Self::CompactionSummaryMessage { summary, .. }
            | Self::BranchSummaryMessage { summary, .. } => summary.clone(),
        }
    }
}

/// Custom content → model-visible text; `None` for null / empty (c2725).
fn custom_projection_text(content: &Value) -> Option<String> {
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Null => return None,
        other => other.to_string(),
    };
    (!text.is_empty()).then_some(text)
}

/// Session/agent transcript entry: LLM turn **or** environment meta.
///
/// Wire format stays flat `{ "role": … }` via `untagged` + inner tagged enums.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentMessage {
    Llm(LlmMessage),
    Env(EnvMessage),
}

impl From<LlmMessage> for AgentMessage {
    fn from(value: LlmMessage) -> Self {
        Self::Llm(value)
    }
}

impl From<EnvMessage> for AgentMessage {
    fn from(value: EnvMessage) -> Self {
        Self::Env(value)
    }
}

impl AgentMessage {
    pub fn as_llm(&self) -> Option<&LlmMessage> {
        match self {
            Self::Llm(m) => Some(m),
            Self::Env(_) => None,
        }
    }

    pub fn role_name(&self) -> &'static str {
        match self {
            Self::Llm(m) => m.role_name(),
            Self::Env(m) => m.role_name(),
        }
    }

    pub fn content(&self) -> &[AgentPart] {
        match self {
            Self::Llm(m) => m.content(),
            Self::Env(_) => &[],
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::Llm(m) => m.text(),
            Self::Env(m) => m.text(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.as_llm().is_some_and(LlmMessage::is_error)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::Llm(LlmMessage::user(text))
    }

    /// User message with arbitrary parts (text + images, c1155 / dm7).
    pub fn user_parts(content: Vec<AgentPart>) -> Self {
        Self::Llm(LlmMessage::user_parts(content))
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::Llm(LlmMessage::assistant(text))
    }

    pub fn tool_result(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        content: Vec<AgentPart>,
        is_error: bool,
    ) -> Self {
        Self::tool_result_with_details(id, tool_name, content, None, is_error)
    }

    pub fn tool_result_with_details(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        content: Vec<AgentPart>,
        details: Option<Value>,
        is_error: bool,
    ) -> Self {
        Self::Llm(LlmMessage::ToolResultMessage {
            tool_use_id: id.into(),
            tool_name: tool_name.into(),
            content,
            details,
            is_error,
            timestamp: now_ms(),
        })
    }

    pub fn bash(
        command: impl Into<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self::Env(EnvMessage::BashExecutionMessage {
            bash_id: String::new(),
            command: command.into(),
            output: output.into(),
            exit_code,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
            status: BashExecutionStatus::Done,
        })
    }

    /// Interactive bang start row (c2760): command only, `status = Running`.
    pub fn bash_running(
        bash_id: impl Into<String>,
        command: impl Into<String>,
        exclude_from_context: bool,
    ) -> Self {
        Self::Env(EnvMessage::BashExecutionMessage {
            bash_id: bash_id.into(),
            command: command.into(),
            output: String::new(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context,
            status: BashExecutionStatus::Running,
        })
    }
}

// ── Display ─────────────────────────────────────────────────────────

impl std::fmt::Display for AgentMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.role_name(), self.text())
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_message_has_new_fields() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::text("response")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(XyUsage {
                input: 100,
                output: 50,
                total_tokens: 150,
                cost: Some(XyUsageCost {
                    input: 0.001,
                    output: 0.002,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.003,
                }),
                ..Default::default()
            }),
            api: "openai".into(),
            provider: "openai".into(),
            model: "gpt-4".into(),
            response_id: Some("resp-123".into()),
            error_message: None,
            timestamp: 1000,
            diagnostics: vec![],
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role_name(), "assistant");
        assert_eq!(deserialized.text(), "response");
        assert!(matches!(deserialized, AgentMessage::Llm(_)));
    }

    #[test]
    fn assistant_error_message() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![],
            stop_reason: Some(XyStopReason::Error),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: Some("Rate limit exceeded".into()),
            timestamp: 1000,
            diagnostics: vec![Diagnostic {
                message: "Retried 3 times".into(),
                source: Some("openai".into()),
            }],
        });
        assert!(msg.is_error());
        assert_eq!(msg.text(), "");
    }

    #[test]
    fn tool_result_message_has_tool_name() {
        let msg = AgentMessage::Llm(LlmMessage::ToolResultMessage {
            tool_use_id: "call-1".into(),
            tool_name: "read_file".into(),
            content: vec![AgentPart::text("file contents")],
            details: Some(serde_json::json!({"path": "src/main.rs", "lines": 42})),
            is_error: false,
            timestamp: 1000,
        });
        assert_eq!(msg.role_name(), "toolResult");
    }

    #[test]
    fn usage_with_cost() {
        let mut usage = XyUsage {
            input: 1000,
            output: 500,
            cache_read: 200,
            cache_write: 100,
            cache_write_1h: 50,
            total_tokens: 0,
            ..Default::default()
        }
        .with_prompt_cache_read(xylitol_ai_bridge::dto::PromptCacheRead::Tokens(200));
        usage.compute_total();
        assert_eq!(usage.total_tokens, 1500);

        usage.compute_cost(10.0, 30.0, 1.0, 5.0);
        let cost = usage.cost.unwrap();
        assert!((cost.total - (0.010 + 0.015 + 0.0002 + 0.0005)).abs() < 1e-6);
    }

    #[test]
    fn thinking_part_with_metadata() {
        let part = AgentPart::Thinking {
            thinking: "Let me reason...".into(),
            redacted: false,
            thinking_signature: Some("sig-abc".into()),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(
            json.contains(r#""type":"thinking""#) && json.contains(r#""thinking":"#),
            "tagged thinking wire: {json}"
        );
        assert!(
            json.contains("thinkingSignature"),
            "signature key must be thinkingSignature: {json}"
        );
        let deserialized: AgentPart = serde_json::from_str(&json).unwrap();
        match deserialized {
            AgentPart::Thinking {
                thinking,
                redacted,
                thinking_signature,
            } => {
                assert_eq!(thinking, "Let me reason...");
                assert!(!redacted);
                assert_eq!(thinking_signature, Some("sig-abc".into()));
            }
            _ => panic!("expected Thinking"),
        }
        assert!(serde_json::from_str::<AgentPart>(r#""bare""#).is_err());
        assert!(serde_json::from_str::<AgentPart>(r#"{"redacted":false,"text":"x"}"#).is_err());
    }

    #[test]
    fn text_part_is_tagged_not_bare_string() {
        let json = serde_json::to_string(&AgentPart::text("hi")).unwrap();
        assert_eq!(json, r#"{"type":"text","text":"hi"}"#);
    }

    #[test]
    fn tool_result_message_uses_tool_call_id_key() {
        let msg = AgentMessage::tool_result("c1", "read", vec![AgentPart::text("ok")], false);
        let v = serde_json::to_value(&msg).unwrap();
        assert!(v.get("toolCallId").is_some(), "{v}");
        assert!(v.get("toolUseId").is_none(), "{v}");
        assert_eq!(v.get("toolName").and_then(|x| x.as_str()), Some("read"));
        assert_eq!(v.get("isError").and_then(|x| x.as_bool()), Some(false));
        assert!(v.get("tool_name").is_none(), "{v}");
        assert!(v.get("is_error").is_none(), "{v}");
    }

    #[test]
    fn assistant_wire_uses_camel_case_stop_reason() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::text("")],
            stop_reason: Some(XyStopReason::Error),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: Some("overflow".into()),
            timestamp: 1,
            diagnostics: Vec::new(),
        });
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v.get("stopReason").and_then(|x| x.as_str()), Some("error"));
        assert_eq!(
            v.get("errorMessage").and_then(|x| x.as_str()),
            Some("overflow")
        );
        assert!(v.get("stop_reason").is_none(), "{v}");
    }

    #[test]
    fn all_seven_roles_serialize_and_deserialize() {
        let messages = vec![
            AgentMessage::user("hi"),
            AgentMessage::assistant("hello"),
            AgentMessage::tool_result("t1", "read_file", vec![AgentPart::text("done")], false),
            AgentMessage::bash("pwd", "/home", Some(0)),
            AgentMessage::Env(EnvMessage::CustomMessage {
                custom_type: "x".into(),
                content: serde_json::json!({}),
                display: serde_json::json!({}),
                details: serde_json::json!({}),
            }),
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "s".into(),
                tokens_before: 10,
                tokens_after: 2,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                summary: "s".into(),
                from_id: "e-1".into(),
            }),
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.role_name(), msg.role_name());
        }
    }

    #[test]
    fn wire_user_deserializes_as_llm_variant() {
        let json = r#"{"role":"user","content":[{"type":"text","text":"hi"}],"timestamp":1}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(
            msg,
            AgentMessage::Llm(LlmMessage::UserMessage { .. })
        ));
    }

    #[test]
    fn wire_bash_deserializes_as_env_variant() {
        let json = r#"{"role":"bashExecution","command":"ls","output":"a","cancelled":false,"truncated":false,"excludeFromContext":false}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(
            msg,
            AgentMessage::Env(EnvMessage::BashExecutionMessage { .. })
        ));

        let camel = r#"{"role":"bashExecution","command":"ls","output":"a","cancelled":false,"truncated":false,"excludeFromContext":true}"#;
        let msg: AgentMessage = serde_json::from_str(camel).unwrap();
        match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                exclude_from_context: true,
                ..
            }) => {}
            other => panic!("expected excluded bash, got {other:?}"),
        }

        // v6: snake aliases are removed (s18 / c2260). Unknown snake keys are
        // ignored by serde, so a snake `exclude_from_context` no longer flips the
        // flag — the row deserializes with the default value.
        let snake = r#"{"role":"bashExecution","command":"ls","output":"a","cancelled":false,"truncated":false,"exclude_from_context":true}"#;
        let msg: AgentMessage = serde_json::from_str(snake).unwrap();
        match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                exclude_from_context: false,
                ..
            }) => {}
            other => panic!("snake alias must be ignored, got {other:?}"),
        }

        let v = serde_json::to_value(AgentMessage::bash("pwd", "/tmp", Some(0))).unwrap();
        assert_eq!(v.get("excludeFromContext"), Some(&serde_json::json!(false)));
        assert!(v.get("exclude_from_context").is_none(), "{v}");
        assert_eq!(v.get("exitCode"), Some(&serde_json::json!(0)));
    }

    #[test]
    fn llm_arm_is_bridge_dto_alias() {
        // dm6: Llm leaf is bridge AiBridgeMessage (type alias), not a parallel enum.
        let llm: LlmMessage = LlmMessage::user("hi");
        let bridge: xylitol_ai_bridge::dto::AiBridgeMessage = llm;
        assert_eq!(bridge.role_name(), "user");
    }

    // ── c2725: env projection / exclusion helpers ────────────────────

    #[test]
    fn env_llm_projection_covers_every_variant() {
        // Exhaustive: adding an EnvMessage variant must update this match (c2725).
        let bash = AgentMessage::bash("ls", "a.txt", Some(0));
        let excluded = AgentMessage::Env(EnvMessage::BashExecutionMessage {
            bash_id: String::new(),
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
            status: crate::protocol::message::BashExecutionStatus::Done,
        });
        let summary = AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
            summary: "compressed".into(),
            tokens_before: 100,
            tokens_after: 10,
            read_files: None,
            modified_files: None,
        });
        let branch = AgentMessage::Env(EnvMessage::BranchSummaryMessage {
            summary: "forked".into(),
            from_id: "a".into(),
        });
        let custom = AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: "note".into(),
            content: serde_json::json!("pinned"),
            display: serde_json::Value::Null,
            details: serde_json::Value::Null,
        });
        let custom_obj = AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: "obj".into(),
            content: serde_json::json!({ "k": "v" }),
            display: serde_json::Value::Null,
            details: serde_json::Value::Null,
        });
        let custom_empty = AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: "empty".into(),
            content: serde_json::Value::String(String::new()),
            display: serde_json::Value::Null,
            details: serde_json::Value::Null,
        });

        for msg in [
            &bash,
            &excluded,
            &summary,
            &branch,
            &custom,
            &custom_obj,
            &custom_empty,
        ] {
            let AgentMessage::Env(env) = msg else {
                panic!("expected env, got {msg:?}")
            };
            match env.llm_projection() {
                Some(EnvLlmProjection::Bash { command, output }) => {
                    assert_eq!(command, "ls");
                    assert_eq!(output, "a.txt");
                }
                Some(EnvLlmProjection::ContextSummary { summary }) => {
                    assert!(summary == "compressed" || summary == "forked");
                }
                Some(EnvLlmProjection::CustomText { text }) => {
                    assert!(text == "pinned" || text == r#"{"k":"v"}"#);
                }
                None => assert!(
                    matches!(env, EnvMessage::BashExecutionMessage { .. })
                        || matches!(env, EnvMessage::CustomMessage { .. })
                ),
            }
        }
    }

    #[test]
    fn env_exclude_from_context_only_bash_flag() {
        let bash = AgentMessage::bash("ls", "a", Some(0));
        let AgentMessage::Env(env) = &bash else {
            panic!("expected env")
        };
        assert!(!env.exclude_from_context());

        let excluded = AgentMessage::Env(EnvMessage::BashExecutionMessage {
            bash_id: String::new(),
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
            status: crate::protocol::message::BashExecutionStatus::Done,
        });
        let AgentMessage::Env(env) = &excluded else {
            panic!("expected env")
        };
        assert!(env.exclude_from_context());

        // Every non-bash variant is never excluded.
        let summary = EnvMessage::CompactionSummaryMessage {
            summary: "s".into(),
            tokens_before: 1,
            tokens_after: 1,
            read_files: None,
            modified_files: None,
        };
        let branch = EnvMessage::BranchSummaryMessage {
            summary: "s".into(),
            from_id: "a".into(),
        };
        let custom = EnvMessage::CustomMessage {
            custom_type: "n".into(),
            content: serde_json::json!("x"),
            display: serde_json::Value::Null,
            details: serde_json::Value::Null,
        };
        for env in [&summary, &branch, &custom] {
            assert!(!env.exclude_from_context());
        }
    }

    #[test]
    fn bash_execution_status_defaults_to_done_for_old_rows() {
        // c2760: JSONL rows written before the lifecycle fields exist must
        // deserialize as finished (Done) and keep projecting to the model.
        let json = r#"{"role":"bashExecution","command":"ls","output":"a","cancelled":false,"truncated":false,"excludeFromContext":false}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        let AgentMessage::Env(EnvMessage::BashExecutionMessage {
            bash_id, status, ..
        }) = &msg
        else {
            panic!("expected env bash");
        };
        assert!(bash_id.is_empty());
        assert_eq!(*status, BashExecutionStatus::Done);
        // running rows never project to the model (stable prefix invariant).
        let running = AgentMessage::bash_running("b1", "ls", false);
        let AgentMessage::Env(env) = &running else {
            panic!("expected env");
        };
        assert_eq!(env.llm_projection(), None);
    }

    #[test]
    fn env_excluded_bash_projects_to_none() {
        let msg = AgentMessage::Env(EnvMessage::BashExecutionMessage {
            bash_id: String::new(),
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
            status: crate::protocol::message::BashExecutionStatus::Done,
        });
        let AgentMessage::Env(env) = &msg else {
            panic!("expected env")
        };
        assert_eq!(env.llm_projection(), None);
    }
}
