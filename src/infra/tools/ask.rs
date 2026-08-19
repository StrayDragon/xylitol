//! TUI-only builtin `ask` tool — structured clarify / decision questionnaire.
//!
//! Observability: inherits the generic `tool_exec::run_one` ToolExecuteSpan
//! (fastrace / langfuse). No per-tool ask span is required.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::typed::TypedTool;
use crate::protocol::BuiltinToolName;
use crate::protocol::error::XyToolError;
use crate::protocol::ports::ask::{AskArgs, AskUserGateway};
use crate::protocol::ports::{XyToolCtx, XyToolExecutionMode};

/// Builtin `ask` — Barrier / Sequential; not in [`super::default_tools`].
pub struct AskTool {
    gateway: Arc<dyn AskUserGateway>,
}

impl AskTool {
    pub fn new(gateway: Arc<dyn AskUserGateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TypedTool for AskTool {
    type Args = AskArgs;

    fn name(&self) -> &str {
        BuiltinToolName::Ask.as_str()
    }

    fn description(&self) -> &str {
        "Ask the user structured clarifying or decision questions (single/multi choice). Use when requirements are unclear or a fork must be chosen. Skip is a successful structured outcome."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "One or more questions (1×single / 1×multi / ≥2 tabs+review)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Stable question id (returned in answers)"
                            },
                            "prompt": {
                                "type": "string",
                                "description": "Question text shown to the user"
                            },
                            "label": {
                                "type": "string",
                                "description": "Short tab label (multi-question); defaults to id"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["single", "multi"],
                                "description": "single = one choice; multi = checkboxes"
                            },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "value": { "type": "string" },
                                        "label": { "type": "string" },
                                        "description": {
                                            "type": "string",
                                            "description": "Optional human explanation; MAY omit"
                                        },
                                        "recommended": {
                                            "type": "boolean",
                                            "description": "Optional recommended badge"
                                        }
                                    },
                                    "required": ["value", "label"]
                                }
                            },
                            "allow_other": {
                                "type": "boolean",
                                "description": "Allow free-text Other (default true)"
                            }
                        },
                        "required": ["id", "prompt", "mode", "options"]
                    },
                    "minItems": 1
                }
            },
            "required": ["questions"]
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        // Barrier — never enter a parallel tool window while waiting on the user.
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: AskArgs) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        if args.questions.is_empty() {
            return Err(XyToolError::InvalidArgs(
                "ask requires at least one question".into(),
            ));
        }
        let result = self.gateway.prompt(args).await?;
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        Ok(result)
    }
}

/// `default_tools()` plus an `ask` tool bound to `gateway`.
///
/// Used by TUI composition / MCP reload. Print MUST keep plain [`super::default_tools`].
/// Prefer [`default_tools_with_ask_and_todo`] when a session-bound Todo gateway exists.
pub fn default_tools_with_ask(
    gateway: Arc<dyn AskUserGateway>,
) -> Vec<Arc<dyn crate::protocol::ports::XyTool>> {
    let mut tools = super::default_tools();
    tools.push(Arc::new(AskTool::new(gateway)));
    tools
}

/// Session-bound Todo builtins + TUI-only `ask`.
pub fn default_tools_with_ask_and_todo(
    ask: Arc<dyn AskUserGateway>,
    todo: Arc<dyn crate::protocol::ports::AgentTodoGateway>,
) -> Vec<Arc<dyn crate::protocol::ports::XyTool>> {
    let mut tools = super::default_tools_with_todo(todo);
    tools.push(Arc::new(AskTool::new(ask)));
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::tools::default_tools;
    use crate::protocol::ports::ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg};

    struct MockGateway {
        payload: String,
    }

    #[async_trait]
    impl AskUserGateway for MockGateway {
        async fn prompt(&self, _args: AskArgs) -> Result<String, XyToolError> {
            Ok(self.payload.clone())
        }
    }

    #[test]
    fn ask_not_in_default_tools() {
        assert!(
            default_tools().iter().all(|t| t.name() != "ask"),
            "ask MUST NOT be in default_tools"
        );
    }

    #[test]
    fn ask_execution_mode_is_sequential() {
        let gateway: Arc<dyn AskUserGateway> = Arc::new(MockGateway {
            payload: r#"{"status":"skipped","answers":[]}"#.into(),
        });
        let tool = AskTool::new(gateway);
        assert_eq!(
            TypedTool::execution_mode(&tool),
            XyToolExecutionMode::Sequential
        );
    }

    #[test]
    fn ask_schema_has_questions() {
        let gateway: Arc<dyn AskUserGateway> = Arc::new(MockGateway {
            payload: r#"{"status":"skipped","answers":[]}"#.into(),
        });
        let tool = AskTool::new(gateway);
        let schema = TypedTool::parameters_schema(&tool);
        assert!(
            schema
                .get("properties")
                .and_then(|p| p.get("questions"))
                .is_some()
        );
    }

    #[tokio::test]
    async fn ask_gateway_returns_skipped_json() {
        let payload = r#"{"status":"skipped","answers":[]}"#;
        let gateway: Arc<dyn AskUserGateway> = Arc::new(MockGateway {
            payload: payload.into(),
        });
        let tool = AskTool::new(gateway);
        let ctx = XyToolCtx::new("t1");
        let args = AskArgs {
            questions: vec![AskQuestionArg {
                id: "q1".into(),
                prompt: "Pick one".into(),
                label: None,
                mode: AskModeArg::Single,
                options: vec![AskOptionArg {
                    value: "a".into(),
                    label: "A".into(),
                    description: None,
                    recommended: false,
                }],
                allow_other: true,
            }],
        };
        let out = tool.execute_typed(&ctx, args).await.expect("ok");
        assert_eq!(out, payload);
    }

    #[tokio::test]
    async fn ask_abort_before_prompt() {
        let gateway: Arc<dyn AskUserGateway> = Arc::new(MockGateway {
            payload: r#"{"status":"skipped","answers":[]}"#.into(),
        });
        let tool = AskTool::new(gateway);
        let ctx = XyToolCtx::new("t1");
        ctx.cancel.cancel();
        let args = AskArgs {
            questions: vec![AskQuestionArg {
                id: "q1".into(),
                prompt: "x".into(),
                label: None,
                mode: AskModeArg::Single,
                options: vec![AskOptionArg {
                    value: "a".into(),
                    label: "A".into(),
                    description: None,
                    recommended: false,
                }],
                allow_other: false,
            }],
        };
        let err = tool.execute_typed(&ctx, args).await.expect_err("aborted");
        assert!(matches!(err, XyToolError::Aborted));
    }

    #[test]
    fn default_tools_with_ask_includes_ask() {
        let gateway: Arc<dyn AskUserGateway> = Arc::new(MockGateway {
            payload: r#"{"status":"skipped","answers":[]}"#.into(),
        });
        let names: Vec<_> = default_tools_with_ask(gateway)
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "ask"));
        assert!(names.iter().any(|n| n == "read"));
    }
}
