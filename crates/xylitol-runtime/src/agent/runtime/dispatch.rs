use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::{
    AgentMessage, AgentPart, LoopChain, ObsSessionContext, XyBatchMode, XyEvent, XyHookBus,
    XyLlmMessage, XyToolCallResult, XyToolCatalog, XyToolDispatch, XyToolError,
};

use super::hooks::AgentHooks;

use super::tool_exec::{ToolExecEnv, prepare_call, run_one};
use crate::agent::tools::{ToolCatalog, ToolSet};

type PermissionCheck = dyn Fn(&str, &str) -> Option<String> + Send + Sync;

pub(crate) struct DispatchHandle {
    tools: ToolSet,
    hooks: AgentHooks,
    hook_bus: Option<Arc<dyn XyHookBus>>,
    permission_check: Option<Arc<PermissionCheck>>,
    cancel: CancellationToken,
    turn_id: Option<String>,
    batch_mode: XyBatchMode,
    workspace: String,
    workspaces: Vec<String>,
    obs_session: ObsSessionContext,
    loop_chain: LoopChain,
    turn_index: u32,
    parent_call_id: String,
    catalog: Arc<ToolCatalog>,
    max_nested: usize,
    nested: AtomicUsize,
    sequence: AtomicUsize,
    progress_tx: Option<mpsc::Sender<String>>,
}

impl DispatchHandle {
    pub(crate) fn new(
        env: &ToolExecEnv<'_>,
        parent_call_id: &str,
        max_nested: usize,
        progress_tx: Option<mpsc::Sender<String>>,
    ) -> Self {
        let catalog = Arc::new(ToolCatalog::from_toolset(env.tools, None));
        Self {
            tools: env.tools.clone(),
            hooks: env.hooks.clone(),
            hook_bus: env.hook_bus.clone(),
            permission_check: env.permission_check.clone(),
            cancel: env.cancel.clone(),
            turn_id: env.turn_id.map(str::to_string),
            batch_mode: env.batch_mode,
            workspace: env.workspace.to_string(),
            workspaces: env.workspaces.to_vec(),
            obs_session: env.obs_session.clone(),
            loop_chain: env.loop_chain.clone(),
            turn_index: env.turn_index,
            parent_call_id: parent_call_id.to_string(),
            catalog,
            max_nested,
            nested: AtomicUsize::new(0),
            sequence: AtomicUsize::new(0),
            progress_tx,
        }
    }

    fn env(&self) -> ToolExecEnv<'_> {
        ToolExecEnv {
            tools: &self.tools,
            hooks: &self.hooks,
            hook_bus: &self.hook_bus,
            permission_check: &self.permission_check,
            cancel: &self.cancel,
            turn_id: self.turn_id.as_deref(),
            batch_mode: self.batch_mode,
            workspace: &self.workspace,
            workspaces: &self.workspaces,
            obs_session: &self.obs_session,
            loop_chain: &self.loop_chain,
            turn_index: self.turn_index,
        }
    }

    fn next_id(&self) -> String {
        let n = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        format!("{}:code:{n}", self.parent_call_id)
    }
}

#[async_trait]
impl XyToolDispatch for DispatchHandle {
    async fn call(
        &self,
        _parent_call_id: &str,
        name: &str,
        args: serde_json::Value,
    ) -> Result<XyToolCallResult, XyToolError> {
        if name == "run_code" {
            return Err(XyToolError::InvalidArgs(
                "run_code cannot be called recursively".into(),
            ));
        }
        let Some(tool) = self.tools.get(name) else {
            return Err(XyToolError::InvalidArgs(format!("unknown tool: {name}")));
        };
        let canonical = tool.name();
        if self.catalog.describe(canonical).is_none() {
            return Err(XyToolError::InvalidArgs(format!(
                "tool is not available to codemode: {canonical}"
            )));
        }
        let previous = self.nested.fetch_add(1, Ordering::Relaxed);
        if previous >= self.max_nested {
            self.nested.fetch_sub(1, Ordering::Relaxed);
            return Err(XyToolError::InvalidArgs(format!(
                "nested tool call limit exceeded ({})",
                self.max_nested
            )));
        }
        let id = self.next_id();
        let env = self.env();
        let call = prepare_call(&env, &id, name, &args, None).await;
        if call.stop_run {
            self.nested.fetch_sub(1, Ordering::Relaxed);
            return Err(XyToolError::InvalidArgs(
                "nested call stopped by middleware".into(),
            ));
        }
        if let Some(fault) = call.fault {
            self.nested.fetch_sub(1, Ordering::Relaxed);
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                fault.message()
            )));
        }
        if let Some(reason) = call.denied_reason {
            self.nested.fetch_sub(1, Ordering::Relaxed);
            return Err(XyToolError::PermissionDenied(reason));
        }
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        let outcome = run_one(&env, &call, tx, None, 0).await;
        while let Ok(event) = rx.try_recv() {
            if let XyEvent::ToolExecutionUpdate { output, .. } = event
                && let Some(progress) = &self.progress_tx
            {
                let _ = progress.send(output).await;
            }
        }
        self.nested.fetch_sub(1, Ordering::Relaxed);
        if outcome.stop_run {
            return Err(XyToolError::InvalidArgs(
                "nested call stopped by middleware".into(),
            ));
        }
        if let Some(fault) = outcome.fault {
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                fault.message()
            )));
        }
        let (parts, details, is_error) = match outcome.message {
            AgentMessage::Llm(XyLlmMessage::ToolResultMessage {
                content,
                details,
                is_error,
                ..
            }) => (content, details, is_error),
            other => (vec![AgentPart::text(other.text())], None, false),
        };
        Ok(XyToolCallResult {
            parts,
            details,
            is_error,
        })
    }

    fn catalog(&self) -> Arc<dyn XyToolCatalog> {
        self.catalog.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::{Value, json};

    use super::*;
    use xylitol_plugin_sdk::{XyTool, XyToolCtx, XyToolExposure};

    struct Stub(&'static str, XyToolExposure);

    #[async_trait]
    impl XyTool for Stub {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn parameters_schema(&self) -> Value {
            json!({"type": "object"})
        }
        fn exposure(&self) -> XyToolExposure {
            self.1
        }
        async fn execute(&self, _: &XyToolCtx, _: Value) -> Result<String, XyToolError> {
            Ok("ok".into())
        }
    }

    fn env<'a>(tools: &'a ToolSet) -> ToolExecEnv<'a> {
        static HOOKS: std::sync::OnceLock<AgentHooks> = std::sync::OnceLock::new();
        static CHAIN: std::sync::OnceLock<LoopChain> = std::sync::OnceLock::new();
        static OBS: std::sync::OnceLock<ObsSessionContext> = std::sync::OnceLock::new();
        static CANCEL: std::sync::OnceLock<CancellationToken> = std::sync::OnceLock::new();
        ToolExecEnv {
            tools,
            hooks: HOOKS.get_or_init(AgentHooks::empty),
            hook_bus: &None,
            permission_check: &None,
            cancel: CANCEL.get_or_init(CancellationToken::new),
            turn_id: None,
            batch_mode: XyBatchMode::Sequential,
            workspace: "/tmp",
            workspaces: &[],
            obs_session: OBS.get_or_init(ObsSessionContext::default),
            loop_chain: CHAIN.get_or_init(LoopChain::default),
            turn_index: 0,
        }
    }

    #[tokio::test]
    async fn dispatch_rejects_recursion_and_hidden_tools() {
        let set = ToolSet::from_iter(vec![
            Arc::new(Stub("read", XyToolExposure::Direct)) as Arc<dyn XyTool>,
            Arc::new(Stub("run_code", XyToolExposure::ModelOnly)) as Arc<dyn XyTool>,
            Arc::new(Stub("secret", XyToolExposure::Hidden)) as Arc<dyn XyTool>,
        ]);
        let handle = DispatchHandle::new(&env(&set), "outer", 4, None);
        assert!(handle.call("outer", "run_code", json!({})).await.is_err());
        assert!(handle.call("outer", "secret", json!({})).await.is_err());
    }

    #[tokio::test]
    async fn dispatch_calls_visible_tool() {
        let set = ToolSet::from_iter(vec![
            Arc::new(Stub("read", XyToolExposure::Direct)) as Arc<dyn XyTool>
        ]);
        let handle = DispatchHandle::new(&env(&set), "outer", 4, None);
        let result = handle.call("outer", "read", json!({})).await.expect("call");
        assert!(!result.is_error);
    }
}
