use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use xylitol_plugin_sdk::TypedTool;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::session::{
    Workflow, WorkflowStep, latest_workflow, runnable_workflow_step, validate_workflow_replan,
    validate_workflow_with_limits,
};
use xylitol_plugin_sdk::{WorkflowGateway, XySessionStore, XyToolCtx, XyToolExecutionMode};

pub struct SessionWorkflowGateway {
    store: Arc<dyn XySessionStore>,
    session_id: RwLock<Option<String>>,
}

impl SessionWorkflowGateway {
    pub fn new(store: Arc<dyn XySessionStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            session_id: RwLock::new(None),
        })
    }

    pub async fn bind_session(&self, session_id: Option<String>) {
        *self.session_id.write().await = session_id;
    }

    async fn load_latest(&self) -> Result<Option<Workflow>, XyToolError> {
        let Some(sid) = self.session_id.read().await.clone() else {
            return Ok(None);
        };
        let branch = self
            .store
            .load_leaf_branch(&sid)
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        Ok(latest_workflow(&branch))
    }

    async fn store_workflow(&self, workflow: &Workflow) -> Result<(), XyToolError> {
        let Some(sid) = self.session_id.read().await.clone() else {
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                "workflow write requires a bound session"
            )));
        };
        self.store
            .append_session_entry(&sid, &workflow.to_custom_entry_shell())
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        Ok(())
    }
}

#[async_trait]
impl WorkflowGateway for SessionWorkflowGateway {
    async fn latest(&self) -> Result<Option<Workflow>, XyToolError> {
        self.load_latest().await
    }

    async fn persist(&self, workflow: &Workflow) -> Result<(), XyToolError> {
        self.store_workflow(workflow).await
    }

    async fn bind_session(&self, session_id: Option<String>) {
        SessionWorkflowGateway::bind_session(self, session_id).await;
    }
}

pub struct WorkflowWriteTool {
    gateway: Arc<dyn WorkflowGateway>,
    max_depth: u32,
    max_total_steps: usize,
}

impl WorkflowWriteTool {
    pub fn new(gateway: Arc<dyn WorkflowGateway>, max_depth: u32, max_total_steps: usize) -> Self {
        Self {
            gateway,
            max_depth,
            max_total_steps,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkflowWriteArgs {
    pub goal: String,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub resume_cursor: Option<String>,
}

#[async_trait]
impl TypedTool for WorkflowWriteTool {
    type Args = WorkflowWriteArgs;

    fn name(&self) -> &str {
        "workflow_write"
    }

    fn description(&self) -> &str {
        "Create or replace the workflow DAG. Validates dependencies and replan constraints before persisting."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "$ref": "#/$defs/workflow",
            "$defs": {
                "workflow": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["goal", "steps"],
                    "properties": {
                        "goal": { "type": "string" },
                        "steps": {
                            "type": "array",
                            "minItems": 1,
                            "items": { "$ref": "#/$defs/workflow_step" }
                        },
                        "resume_cursor": {
                            "type": "string",
                            "description": "Slash-separated step path, e.g. s2/s2.1"
                        }
                    }
                },
                "workflow_step": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["id", "description"],
                    "properties": {
                        "id": { "type": "string" },
                        "description": { "type": "string" },
                        "depends_on": { "type": "array", "items": { "type": "string" } },
                        "inputs": { "type": "array", "items": { "type": "string" } },
                        "outputs": { "type": "array", "items": { "type": "string" } },
                        "risk": { "type": "string", "enum": ["low", "medium", "high"] },
                        "verification": { "type": "string" },
                        "rollback": { "type": "string" },
                        "requires_human": { "type": "boolean" },
                        "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "failed", "blocked"] },
                        "output_ref": { "type": "string" },
                        "sub_workflow": { "$ref": "#/$defs/workflow" }
                    }
                }
            }
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: WorkflowWriteArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let workflow = Workflow {
            goal: args.goal,
            steps: args.steps,
            resume_cursor: args.resume_cursor,
            updated_at: now_ms(),
        };
        validate_workflow_with_limits(&workflow, self.max_depth, self.max_total_steps)
            .map_err(|e| XyToolError::InvalidArgs(e.0))?;
        let previous = self.gateway.latest().await?;
        validate_workflow_replan(previous.as_ref(), &workflow)
            .map_err(|e| XyToolError::InvalidArgs(e.0))?;
        self.gateway.persist(&workflow).await?;
        ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::WorkflowUpdated {
            workflow: workflow.clone(),
        });
        let runnable = runnable_workflow_step(&workflow, &HashSet::new())
            .map(|step| step.id.clone())
            .unwrap_or_default();
        Ok(json!({
            "goal": workflow.goal,
            "steps": workflow.steps.len(),
            "completed": workflow.completed_count(),
            "runnable": runnable,
        })
        .to_string())
    }
}

pub struct WorkflowReadTool {
    gateway: Arc<dyn WorkflowGateway>,
}

impl WorkflowReadTool {
    pub fn new(gateway: Arc<dyn WorkflowGateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TypedTool for WorkflowReadTool {
    type Args = Value;

    fn name(&self) -> &str {
        "workflow_read"
    }

    fn description(&self) -> &str {
        "Read the latest workflow DAG as JSON."
    }

    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "additionalProperties": false })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, _args: Value) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        match self.gateway.latest().await? {
            Some(workflow) => Ok(workflow.to_data_value().to_string()),
            None => Err(XyToolError::InvalidArgs("workflow not found".into())),
        }
    }
}

pub fn workflow_tools(
    gateway: Arc<dyn WorkflowGateway>,
    max_depth: u32,
    max_total_steps: usize,
) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
    vec![
        Arc::new(WorkflowWriteTool::new(
            gateway.clone(),
            max_depth,
            max_total_steps,
        )),
        Arc::new(WorkflowReadTool::new(gateway)),
    ]
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use xylitol_plugin_sdk::XyTool;

    async fn gateway() -> (Arc<SessionWorkflowGateway>, Arc<dyn XySessionStore>) {
        let store: Arc<dyn XySessionStore> =
            Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        store.create("s1", Some("."), None).await.expect("create");
        let gateway = SessionWorkflowGateway::new(store.clone());
        gateway.bind_session(Some("s1".into())).await;
        (gateway, store)
    }

    fn step(id: &str, status: xylitol_plugin_sdk::session::WorkflowStepStatus) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            risk: xylitol_plugin_sdk::session::WorkflowRisk::Low,
            verification: None,
            rollback: None,
            requires_human: false,
            status,
            output_ref: None,
            sub_workflow: None,
        }
    }

    #[tokio::test]
    async fn write_persists_and_publishes() {
        let (gateway, store) = gateway().await;
        let tool = WorkflowWriteTool::new(
            gateway.clone(),
            xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
        );
        let (tx, mut rx) = mpsc::unbounded_channel();
        let ctx = XyToolCtx::new("c1").with_state_event_tx(tx);
        let result = tool
            .execute(
                &ctx,
                serde_json::json!({
                    "goal": "ship",
                    "steps": [{"id": "s1", "description": "do it"}]
                }),
            )
            .await
            .expect("write");
        assert!(result.contains("\"runnable\":\"s1\""));
        let event = rx.recv().await.expect("event");
        assert!(matches!(
            event,
            xylitol_plugin_sdk::lifecycle::XyEvent::WorkflowUpdated { .. }
        ));
        let entries = store.load_leaf_branch("s1").await.expect("entries");
        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                xylitol_plugin_sdk::session::SessionEntry::Custom(custom)
                    if custom.custom_type == xylitol_plugin_sdk::session::CUSTOM_TYPE_WORKFLOW
            )),
            "workflow_write must persist a workflow entry"
        );
    }

    #[tokio::test]
    async fn invalid_dag_is_rejected_without_write() {
        let (gateway, store) = gateway().await;
        let tool = WorkflowWriteTool::new(
            gateway,
            xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
        );
        let ctx = XyToolCtx::new("c1");
        let err = tool
            .execute(
                &ctx,
                serde_json::json!({
                    "goal": "bad",
                    "steps": [
                        {"id": "s1", "description": "one", "depends_on": ["missing"]}
                    ]
                }),
            )
            .await
            .expect_err("invalid dag");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err:?}");
        let entries = store.load_leaf_branch("s1").await.expect("entries");
        assert!(!entries.iter().any(|entry| matches!(
            entry,
            xylitol_plugin_sdk::session::SessionEntry::Custom(custom)
                if custom.custom_type == xylitol_plugin_sdk::session::CUSTOM_TYPE_WORKFLOW
        )));
    }

    #[tokio::test]
    async fn replan_cannot_rewrite_completed_steps() {
        let (gateway, store) = gateway().await;
        let first = Workflow {
            goal: "ship".into(),
            steps: vec![step(
                "s1",
                xylitol_plugin_sdk::session::WorkflowStepStatus::Completed,
            )],
            resume_cursor: None,
            updated_at: 1,
        };
        store
            .append_session_entry("s1", &first.to_custom_entry_shell())
            .await
            .expect("seed");
        let tool = WorkflowWriteTool::new(
            gateway,
            xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
        );
        let err = tool
            .execute(
                &XyToolCtx::new("c1"),
                serde_json::json!({
                    "goal": "ship",
                    "steps": [{"id": "s1", "description": "rewritten"}]
                }),
            )
            .await
            .expect_err("replan must reject");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err:?}");
    }

    #[tokio::test]
    async fn read_returns_latest_or_errors() {
        let (gateway, store) = gateway().await;
        let tool = WorkflowReadTool::new(gateway.clone());
        let err = tool
            .execute(&XyToolCtx::new("c1"), serde_json::json!({}))
            .await
            .expect_err("missing workflow");
        assert!(matches!(err, XyToolError::InvalidArgs(_)));
        let workflow = Workflow {
            goal: "ship".into(),
            steps: vec![step(
                "s1",
                xylitol_plugin_sdk::session::WorkflowStepStatus::Pending,
            )],
            resume_cursor: None,
            updated_at: 2,
        };
        store
            .append_session_entry("s1", &workflow.to_custom_entry_shell())
            .await
            .expect("seed");
        let result = tool
            .execute(&XyToolCtx::new("c1"), serde_json::json!({}))
            .await
            .expect("read");
        assert!(result.contains("\"goal\":\"ship\""));
    }
}
