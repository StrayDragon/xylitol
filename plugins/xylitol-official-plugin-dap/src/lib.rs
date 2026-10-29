use std::sync::Arc;

use async_trait::async_trait;
use dapz::agent_sdk::AgentHandle;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;
use xylitol_plugin_sdk::TypedTool;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::{HostRegistries, XyToolCtx};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DapPluginConfig {
    pub backend: String,
    pub cwd: Option<String>,
    pub compression: bool,
}

impl Default for DapPluginConfig {
    fn default() -> Self {
        Self {
            backend: "python3 -m debugpy.adapter".into(),
            cwd: None,
            compression: true,
        }
    }
}

pub struct DapPlugin {
    config: DapPluginConfig,
}

impl DapPlugin {
    pub fn new(config: &DapPluginConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for DapPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "dap"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(registries) = ctx.get::<HostRegistries>() else {
            log::warn!("dap plugin: HostRegistries missing; not assembling tools");
            return Ok(());
        };
        let state = Arc::new(DapState {
            config: self.config.clone(),
            agent: Mutex::new(None),
        });
        let tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> = vec![
            Arc::new(DapLaunchTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapStackTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapScopesTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapVariablesTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapEvaluateTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapContinueTool {
                state: Arc::clone(&state),
            }),
            Arc::new(DapDisconnectTool {
                state: Arc::clone(&state),
            }),
        ];
        let mut names = Vec::new();
        for tool in tools {
            let name = tool.name().to_string();
            registries.tools.insert(name.clone(), tool);
            names.push(name);
        }
        let cleanup_registries = Arc::clone(&registries);
        ctx.effect(move || async move {
            for name in names {
                cleanup_registries.tools.remove(&name);
            }
            if let Some(mut agent) = state.agent.lock().await.take() {
                let _ = agent.disconnect(Some(true)).await;
            }
            Ok(())
        })?;
        Ok(())
    }
}

struct DapState {
    config: DapPluginConfig,
    agent: Mutex<Option<AgentHandle>>,
}

async fn ensure_agent<'a>(
    state: &'a DapState,
    guard: &'a mut Option<AgentHandle>,
) -> Result<&'a mut AgentHandle, XyToolError> {
    if guard.is_none() {
        let mut builder = AgentHandle::builder()
            .backend(&state.config.backend)
            .enable_compression(state.config.compression);
        if let Some(cwd) = state.config.cwd.as_deref() {
            builder = builder.cwd(cwd);
        }
        let handle = builder
            .start()
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e.to_string())))?;
        *guard = Some(handle);
    }
    Ok(guard.as_mut().expect("agent initialized"))
}

fn schema_for<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or_else(|_| serde_json::json!({ "type": "object", "properties": {} }))
}

fn map_dap_error(error: dapz::DapzError) -> XyToolError {
    XyToolError::ExecutionFailed(anyhow::anyhow!(error.to_string()))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LaunchArgs {
    program: String,
    cwd: Option<String>,
    args: Option<Vec<String>>,
    breakpoints: Option<Vec<BreakpointArg>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct BreakpointArg {
    path: String,
    lines: Vec<i64>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct StackArgs {
    thread_id: Option<i64>,
    levels: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FrameArgs {
    frame_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VariablesArgs {
    variables_reference: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EvaluateArgs {
    expression: String,
    frame_id: Option<i64>,
    context: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ThreadArgs {
    thread_id: Option<i64>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct DisconnectArgs {
    terminate_debuggee: Option<bool>,
}

struct DapLaunchTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapLaunchTool {
    type Args = LaunchArgs;

    fn name(&self) -> &str {
        "dap_launch"
    }

    fn description(&self) -> &str {
        "Launch a program under the debug adapter and wait for the first stop. Optional breakpoints are `{path, lines}` entries."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<LaunchArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: LaunchArgs,
    ) -> Result<String, XyToolError> {
        let breakpoints: Option<Vec<(String, Vec<i64>)>> = args
            .breakpoints
            .map(|bps| bps.into_iter().map(|bp| (bp.path, bp.lines)).collect());
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent
            .launch(
                &args.program,
                args.cwd.as_deref(),
                args.args.as_deref(),
                breakpoints.as_deref(),
            )
            .await
            .map_err(map_dap_error)
    }
}

struct DapStackTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapStackTool {
    type Args = StackArgs;

    fn name(&self) -> &str {
        "dap_stack"
    }

    fn description(&self) -> &str {
        "Get stack frames for a thread (compressed by dapz)."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<StackArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: StackArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent
            .get_stack(args.thread_id, args.levels)
            .await
            .map_err(map_dap_error)
    }
}

struct DapScopesTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapScopesTool {
    type Args = FrameArgs;

    fn name(&self) -> &str {
        "dap_scopes"
    }

    fn description(&self) -> &str {
        "Get scopes for a stack frame (compressed by dapz)."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<FrameArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: FrameArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent.get_scopes(args.frame_id).await.map_err(map_dap_error)
    }
}

struct DapVariablesTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapVariablesTool {
    type Args = VariablesArgs;

    fn name(&self) -> &str {
        "dap_variables"
    }

    fn description(&self) -> &str {
        "Get variables for a variables reference (compressed by dapz)."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<VariablesArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: VariablesArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent
            .get_variables(args.variables_reference)
            .await
            .map_err(map_dap_error)
    }
}

struct DapEvaluateTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapEvaluateTool {
    type Args = EvaluateArgs;

    fn name(&self) -> &str {
        "dap_evaluate"
    }

    fn description(&self) -> &str {
        "Evaluate an expression in the debuggee (compressed by dapz)."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<EvaluateArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: EvaluateArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent
            .evaluate(&args.expression, args.frame_id, args.context.as_deref())
            .await
            .map_err(map_dap_error)
    }
}

struct DapContinueTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapContinueTool {
    type Args = ThreadArgs;

    fn name(&self) -> &str {
        "dap_continue"
    }

    fn description(&self) -> &str {
        "Continue execution (compressed by dapz)."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<ThreadArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: ThreadArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard).await?;
        agent.continue_(args.thread_id).await.map_err(map_dap_error)
    }
}

struct DapDisconnectTool {
    state: Arc<DapState>,
}

#[async_trait]
impl TypedTool for DapDisconnectTool {
    type Args = DisconnectArgs;

    fn name(&self) -> &str {
        "dap_disconnect"
    }

    fn description(&self) -> &str {
        "Disconnect from the debuggee; optionally terminate it."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<DisconnectArgs>()
    }

    async fn execute_typed(
        &self,
        _ctx: &XyToolCtx,
        args: DisconnectArgs,
    ) -> Result<String, XyToolError> {
        let mut guard = self.state.agent.lock().await;
        let Some(mut agent) = guard.take() else {
            return Ok("not debugging".to_string());
        };
        agent
            .disconnect(args.terminate_debuggee)
            .await
            .map_err(map_dap_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = DapPluginConfig::default();
        assert_eq!(config.backend, "python3 -m debugpy.adapter");
        assert!(config.compression);
    }

    #[test]
    fn launch_args_accept_breakpoints() {
        let args: LaunchArgs = serde_json::from_value(serde_json::json!({
            "program": "script.py",
            "breakpoints": [{"path": "script.py", "lines": [10]}]
        }))
        .expect("launch args");
        assert_eq!(args.program, "script.py");
        assert_eq!(args.breakpoints.expect("breakpoints").len(), 1);
    }

    #[test]
    fn empty_disconnect_args_default() {
        let args: DisconnectArgs =
            serde_json::from_value(serde_json::json!({})).expect("disconnect args");
        assert!(args.terminate_debuggee.is_none());
    }

    #[tokio::test]
    #[ignore = "requires debugpy adapter; run locally with --ignored"]
    async fn debugpy_launch_stack_evaluate_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("debug_me.py"),
            "import time\nx = 41\nprint(x)\ntime.sleep(2)\n",
        )
        .expect("write python file");
        let backend = std::env::var("XYLITOL_DEBUGPY_BACKEND").unwrap_or_else(|_| {
            "/Users/l8ng/.local/share/uv/tools/debugpy/bin/python -m debugpy.adapter".into()
        });
        let config = DapPluginConfig {
            backend,
            cwd: Some(dir.path().to_string_lossy().into_owned()),
            compression: true,
        };
        let state = Arc::new(DapState {
            config,
            agent: Mutex::new(None),
        });
        let ctx = XyToolCtx::new("test").with_workspace(dir.path());
        let launch = DapLaunchTool {
            state: Arc::clone(&state),
        };
        let launched = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            launch.execute_typed(
                &ctx,
                LaunchArgs {
                    program: "debug_me.py".into(),
                    cwd: Some(dir.path().to_string_lossy().into_owned()),
                    args: None,
                    breakpoints: Some(vec![BreakpointArg {
                        path: "debug_me.py".into(),
                        lines: vec![3],
                    }]),
                },
            ),
        )
        .await
        .expect("launch timeout")
        .expect("launch");
        assert!(!launched.is_empty(), "{launched}");

        let stack = DapStackTool {
            state: Arc::clone(&state),
        };
        let frames = stack
            .execute_typed(
                &ctx,
                StackArgs {
                    thread_id: None,
                    levels: Some(5),
                },
            )
            .await
            .expect("stack");
        assert!(frames.contains("debug_me.py"), "{frames}");
        let frame_id = frames
            .lines()
            .find_map(|line| line.trim().strip_prefix("- id: "))
            .and_then(|value| value.parse::<i64>().ok())
            .expect("frame id in stack output");

        let evaluate = DapEvaluateTool {
            state: Arc::clone(&state),
        };
        let value = evaluate
            .execute_typed(
                &ctx,
                EvaluateArgs {
                    expression: "x".into(),
                    frame_id: Some(frame_id),
                    context: Some("repl".into()),
                },
            )
            .await
            .expect("evaluate");
        assert!(value.contains("41"), "{value}");

        let disconnect = DapDisconnectTool {
            state: Arc::clone(&state),
        };
        disconnect
            .execute_typed(
                &ctx,
                DisconnectArgs {
                    terminate_debuggee: Some(true),
                },
            )
            .await
            .expect("disconnect");
    }
}
