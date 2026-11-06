use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde_json::{Value, json};
use xylitol_plugin_sdk::{
    AgentPart, CodemodeConfig, XyTool, XyToolCtx, XyToolError, XyToolExecutionMode, XyToolExposure,
    XyToolOutcome,
};

use crate::runner::{CodemodeError, CodemodeErrorKind, CodemodeRunner};

pub struct RunCodeTool {
    config: CodemodeConfig,
    runner: OnceLock<Arc<CodemodeRunner>>,
}

impl RunCodeTool {
    pub fn new(config: CodemodeConfig) -> Self {
        Self {
            config,
            runner: OnceLock::new(),
        }
    }

    fn runner(&self) -> Result<&Arc<CodemodeRunner>, XyToolError> {
        if let Some(runner) = self.runner.get() {
            return Ok(runner);
        }
        let runner =
            Arc::new(CodemodeRunner::new(self.config.clone()).map_err(|error| {
                XyToolError::ExecutionFailed(anyhow::anyhow!(error.to_string()))
            })?);
        let _ = self.runner.set(runner);
        Ok(self.runner.get().expect("runner initialized"))
    }

    pub fn runner_initialized(&self) -> bool {
        self.runner.get().is_some()
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
            .runner()?
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
