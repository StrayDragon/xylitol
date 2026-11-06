use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use xylitol_plugin_sdk::{
    AgentPart, XyTool, XyToolCtx, XyToolError, XyToolExecutionMode, XyToolExposure, XyToolOutcome,
};

use crate::runner::{CodemodeError, CodemodeErrorKind, CodemodeRunner};

pub struct RunCodeTool {
    runner: Arc<CodemodeRunner>,
}

impl RunCodeTool {
    pub fn new(runner: Arc<CodemodeRunner>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl XyTool for RunCodeTool {
    fn name(&self) -> &str {
        "run_code"
    }

    fn description(&self) -> &str {
        "Run JavaScript in an isolated Wasmtime sandbox. Call tools through tools.<name>({...}); use ALL_TOOLS, searchTools(query), describeTool(name), and text(value). Return JSON-compatible values."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "JavaScript source. Call tools through tools.<name>({...})."
                },
                "description": {
                    "type": "string",
                    "description": "Short human-readable summary of the script."
                },
                "timeoutMs": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional per-run timeout in milliseconds."
                }
            },
            "required": ["code"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> XyToolExposure {
        XyToolExposure::ModelOnly
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_outcome(
        &self,
        ctx: &XyToolCtx,
        args: Value,
    ) -> Result<XyToolOutcome, XyToolError> {
        let code = args
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(|| XyToolError::InvalidArgs("missing code".into()))?;
        let timeout_ms = args.get("timeoutMs").and_then(Value::as_u64);
        let (text, details) = self
            .runner
            .run_with_details(ctx, code, timeout_ms)
            .await
            .map_err(map_error)?;
        Ok(XyToolOutcome {
            parts: vec![AgentPart::text(text)],
            details: Some(details),
            is_error: false,
            error_kind: None,
        })
    }
}

fn map_error(error: CodemodeError) -> XyToolError {
    match error.kind {
        CodemodeErrorKind::InvalidArgs => XyToolError::InvalidArgs(error.message),
        CodemodeErrorKind::Cancelled => XyToolError::Aborted,
        CodemodeErrorKind::Timeout => XyToolError::Timeout(std::time::Duration::from_millis(0)),
        _ => XyToolError::ExecutionFailed(anyhow::anyhow!(error.message)),
    }
}
