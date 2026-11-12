use std::sync::Arc;

use async_trait::async_trait;
use lspz::agent_sdk::AgentHandle;
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
pub struct LspPluginConfig {
    pub backend: String,
    pub language: String,
    pub workspace_root: Option<String>,
    pub compression: bool,
}

impl Default for LspPluginConfig {
    fn default() -> Self {
        Self {
            backend: "basedpyright-langserver --stdio".into(),
            language: "python".into(),
            workspace_root: None,
            compression: true,
        }
    }
}

pub struct LspPlugin {
    config: LspPluginConfig,
}

impl LspPlugin {
    pub fn new(config: &LspPluginConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for LspPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "lsp"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(registries) = ctx.get::<HostRegistries>() else {
            log::warn!("lsp plugin: HostRegistries missing; not assembling tools");
            return Ok(());
        };
        let state = Arc::new(LspState {
            config: self.config.clone(),
            agent: Mutex::new(AgentPhase::NotStarted),
        });
        let tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> = vec![
            Arc::new(LspDiagnosticsTool {
                state: Arc::clone(&state),
            }),
            Arc::new(LspSymbolsTool {
                state: Arc::clone(&state),
            }),
            Arc::new(LspHoverTool {
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
            if let AgentPhase::Running(agent) =
                std::mem::replace(&mut *state.agent.lock().await, AgentPhase::NotStarted)
            {
                let _ = agent.shutdown().await;
            }
            Ok(())
        })?;
        Ok(())
    }
}

enum AgentPhase {
    NotStarted,
    Running(AgentHandle),
    Dead { reason: String },
}

struct LspState {
    config: LspPluginConfig,
    agent: Mutex<AgentPhase>,
}

fn is_transport_error(error: &anyhow::Error) -> bool {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        return true;
    }
    let text = error.to_string().to_ascii_lowercase();
    [
        "broken pipe",
        "channel closed",
        "connection closed",
        "eof",
        "not connected",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn mark_dead_on_transport(guard: &mut AgentPhase, error: &anyhow::Error) {
    if is_transport_error(error) {
        *guard = AgentPhase::Dead {
            reason: error.to_string(),
        };
    }
}

async fn ensure_agent<'a>(
    state: &'a LspState,
    guard: &'a mut AgentPhase,
    workspace: &str,
) -> Result<&'a mut AgentHandle, XyToolError> {
    if let AgentPhase::Running(handle) = guard {
        return Ok(handle);
    }
    if let AgentPhase::Dead { reason } = guard {
        log::warn!("lsp agent was dead ({reason}); restarting on next request");
    }
    let workspace_root = state
        .config
        .workspace_root
        .clone()
        .unwrap_or_else(|| workspace.to_string());
    let handle = AgentHandle::builder()
        .backend(&state.config.backend)
        .language(&state.config.language)
        .enable_compression(state.config.compression)
        .workspace_root(workspace_root)
        .start()
        .await
        .map_err(XyToolError::ExecutionFailed)?;
    *guard = AgentPhase::Running(handle);
    Ok(match guard {
        AgentPhase::Running(handle) => handle,
        _ => unreachable!("agent initialized above"),
    })
}

fn resolve_uri(
    uri: Option<&str>,
    path: Option<&str>,
    ctx: &XyToolCtx,
) -> Result<String, XyToolError> {
    if let Some(uri) = uri {
        return Ok(uri.to_string());
    }
    let Some(path) = path else {
        return Err(XyToolError::InvalidArgs(
            "`path` or `uri` is required".into(),
        ));
    };
    let candidate = std::path::Path::new(path);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        ctx.workspace.join(candidate)
    };
    url::Url::from_file_path(&absolute)
        .map(|url| url.to_string())
        .map_err(|_| XyToolError::InvalidArgs(format!("not a file path: {}", absolute.display())))
}

fn schema_for<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or_else(|_| serde_json::json!({ "type": "object", "properties": {} }))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PathArgs {
    path: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PositionArgs {
    path: Option<String>,
    uri: Option<String>,
    line: u32,
    character: u32,
}

struct LspDiagnosticsTool {
    state: Arc<LspState>,
}

#[async_trait]
impl TypedTool for LspDiagnosticsTool {
    type Args = PathArgs;

    fn name(&self) -> &str {
        "lsp_diagnostics"
    }

    fn description(&self) -> &str {
        "Get LSP diagnostics for a file. Pass `path` (workspace-relative or absolute) or `uri`."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<PathArgs>()
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: PathArgs) -> Result<String, XyToolError> {
        let uri = resolve_uri(args.uri.as_deref(), args.path.as_deref(), ctx)?;
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard, &ctx.workspace.to_string_lossy()).await?;
        let result = agent.get_diagnostics(&uri).await;
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                mark_dead_on_transport(&mut guard, &error);
                Err(XyToolError::ExecutionFailed(error))
            }
        }
    }
}

struct LspSymbolsTool {
    state: Arc<LspState>,
}

#[async_trait]
impl TypedTool for LspSymbolsTool {
    type Args = PathArgs;

    fn name(&self) -> &str {
        "lsp_symbols"
    }

    fn description(&self) -> &str {
        "Get document symbols for a file. Pass `path` (workspace-relative or absolute) or `uri`."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<PathArgs>()
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: PathArgs) -> Result<String, XyToolError> {
        let uri = resolve_uri(args.uri.as_deref(), args.path.as_deref(), ctx)?;
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard, &ctx.workspace.to_string_lossy()).await?;
        let result = agent.get_symbols(&uri).await;
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                mark_dead_on_transport(&mut guard, &error);
                Err(XyToolError::ExecutionFailed(error))
            }
        }
    }
}

struct LspHoverTool {
    state: Arc<LspState>,
}

#[async_trait]
impl TypedTool for LspHoverTool {
    type Args = PositionArgs;

    fn name(&self) -> &str {
        "lsp_hover"
    }

    fn description(&self) -> &str {
        "Get LSP hover text at a 0-based line/character position. Pass `path` or `uri`."
    }

    fn parameters_schema(&self) -> Value {
        schema_for::<PositionArgs>()
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: PositionArgs,
    ) -> Result<String, XyToolError> {
        let uri = resolve_uri(args.uri.as_deref(), args.path.as_deref(), ctx)?;
        let mut guard = self.state.agent.lock().await;
        let agent = ensure_agent(&self.state, &mut guard, &ctx.workspace.to_string_lossy()).await?;
        let result = agent.get_hover(&uri, args.line, args.character).await;
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                mark_dead_on_transport(&mut guard, &error);
                Err(XyToolError::ExecutionFailed(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_are_zero_cost_when_absent() {
        let config = LspPluginConfig::default();
        assert_eq!(config.backend, "basedpyright-langserver --stdio");
        assert_eq!(config.language, "python");
        assert!(config.compression);
    }

    #[test]
    fn resolve_uri_prefers_explicit_uri() {
        let ctx = XyToolCtx::new("test");
        assert_eq!(
            resolve_uri(Some("file:///tmp/a.py"), Some("ignored.py"), &ctx).unwrap(),
            "file:///tmp/a.py"
        );
    }

    #[test]
    fn resolve_uri_rejects_missing_path_and_uri() {
        let ctx = XyToolCtx::new("test");
        assert!(resolve_uri(None, None, &ctx).is_err());
    }

    #[test]
    fn resolve_uri_builds_file_url() {
        let ctx = XyToolCtx::new("test");
        let uri = resolve_uri(None, Some("a.py"), &ctx).expect("uri");
        assert!(uri.starts_with("file://"));
        assert!(uri.ends_with("/a.py"));
    }

    #[tokio::test]
    #[ignore = "requires basedpyright-langserver on PATH; run locally with --ignored"]
    async fn basedpyright_diagnostics_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("bad.py"),
            "def f():\n    return undefined_name\n",
        )
        .expect("write python file");
        let config = LspPluginConfig {
            backend: "basedpyright-langserver --stdio".into(),
            language: "python".into(),
            workspace_root: Some(dir.path().to_string_lossy().into_owned()),
            compression: true,
        };
        let state = Arc::new(LspState {
            config,
            agent: Mutex::new(AgentPhase::NotStarted),
        });
        let tool = LspDiagnosticsTool { state };
        let ctx = XyToolCtx::new("test").with_workspace(dir.path());
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            tool.execute_typed(
                &ctx,
                PathArgs {
                    path: Some("bad.py".into()),
                    uri: None,
                },
            ),
        )
        .await
        .expect("diagnostics timeout")
        .expect("diagnostics");
        assert!(
            result.contains("undefined_name") || result.contains("reportUndefinedVariable"),
            "{result}"
        );
    }

    #[test]
    fn transport_errors_mark_the_agent_dead() {
        assert!(is_transport_error(&anyhow::anyhow!(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe"
        ))));
        assert!(is_transport_error(&anyhow::anyhow!("channel closed")));
        assert!(!is_transport_error(&anyhow::anyhow!("file not found")));
        let mut phase = AgentPhase::NotStarted;
        mark_dead_on_transport(&mut phase, &anyhow::anyhow!("broken pipe"));
        assert!(matches!(phase, AgentPhase::Dead { .. }));
    }
}
