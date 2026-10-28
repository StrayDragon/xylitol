use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use super::typed::TypedTool;
use xylitol_plugin_sdk::ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway};
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::session::{
    PlanDoc, PlanDocStatus, PlanDocStep, PlanMode, PlanModeKind, TodoItemDraft, latest_plan_doc,
    latest_plan_doc_by_id, latest_plan_mode, plan_docs,
};
use xylitol_plugin_sdk::{
    AgentTodoGateway, PlanGateway, XySessionStore, XyToolCtx, XyToolExecutionMode,
};

pub struct SessionPlanGateway {
    store: Arc<dyn XySessionStore>,
    session_id: RwLock<Option<String>>,
    memory: RwLock<HashMap<String, PlanDoc>>,
    memory_mode: RwLock<PlanMode>,
}

impl SessionPlanGateway {
    pub fn new(store: Arc<dyn XySessionStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            session_id: RwLock::new(None),
            memory: RwLock::new(HashMap::new()),
            memory_mode: RwLock::new(PlanMode::execute()),
        })
    }

    pub async fn bind_session(&self, session_id: Option<String>) {
        *self.session_id.write().await = session_id;
    }

    async fn entries(&self) -> Option<Vec<xylitol_plugin_sdk::session::SessionEntry>> {
        let sid = self.session_id.read().await.clone()?;
        self.store.load_leaf_branch(&sid).await.ok()
    }

    async fn persist(&self, doc: &PlanDoc) -> Result<(), XyToolError> {
        let sid = self.session_id.read().await.clone();
        let Some(sid) = sid else {
            self.memory
                .write()
                .await
                .insert(doc.plan_id.clone(), doc.clone());
            return Ok(());
        };
        self.store
            .append_session_entry(&sid, &doc.to_custom_entry_shell())
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        self.memory
            .write()
            .await
            .insert(doc.plan_id.clone(), doc.clone());
        self.write_markdown(&sid, doc);
        Ok(())
    }

    fn write_markdown(&self, session_id: &str, doc: &PlanDoc) {
        let base = std::env::temp_dir().join("xylitol-plans").join(session_id);
        let dir = base.join("plans");
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let path = dir.join(format!("{}.md", doc.plan_id));
        let _ = std::fs::write(&path, doc.to_markdown());
    }

    async fn persist_mode(&self, mode: &PlanMode) -> Result<(), XyToolError> {
        let sid = self.session_id.read().await.clone();
        if let Some(sid) = sid {
            self.store
                .append_session_entry(&sid, &mode.to_custom_entry_shell())
                .await
                .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        }
        *self.memory_mode.write().await = mode.clone();
        Ok(())
    }
}

#[async_trait]
impl PlanGateway for SessionPlanGateway {
    async fn list(&self) -> Result<Vec<PlanDoc>, XyToolError> {
        let mut docs = self
            .entries()
            .await
            .map(|e| plan_docs(&e))
            .unwrap_or_default();
        let memory = self.memory.read().await;
        for doc in memory.values() {
            if !docs.iter().any(|d| d.plan_id == doc.plan_id) {
                docs.push(doc.clone());
            }
        }
        docs.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then(a.plan_id.cmp(&b.plan_id))
        });
        Ok(docs)
    }

    async fn read(&self, plan_id: Option<&str>) -> Result<Option<PlanDoc>, XyToolError> {
        let from_store = self.entries().await.and_then(|entries| match plan_id {
            Some(id) => latest_plan_doc_by_id(&entries, id),
            None => latest_plan_doc(&entries),
        });
        if from_store.is_some() {
            return Ok(from_store);
        }
        let memory = self.memory.read().await;
        Ok(match plan_id {
            Some(id) => memory.get(id).cloned(),
            None => memory.values().max_by_key(|d| d.updated_at).cloned(),
        })
    }

    async fn write(&self, doc: PlanDoc) -> Result<PlanDoc, XyToolError> {
        doc.validate().map_err(|e| XyToolError::InvalidArgs(e.0))?;
        self.persist(&doc).await?;
        Ok(doc)
    }

    async fn mode(&self) -> Result<PlanMode, XyToolError> {
        if let Some(entries) = self.entries().await {
            return Ok(latest_plan_mode(&entries));
        }
        Ok(self.memory_mode.read().await.clone())
    }

    async fn set_mode(&self, mode: PlanMode) -> Result<(), XyToolError> {
        self.persist_mode(&mode).await
    }

    async fn bind_session(&self, session_id: Option<String>) {
        SessionPlanGateway::bind_session(self, session_id).await;
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn plan_json(doc: &PlanDoc) -> String {
    json!({
        "plan": doc.to_data_value(),
        "markdown": doc.to_markdown(),
    })
    .to_string()
}

fn approved(answer: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(answer) else {
        return false;
    };
    fn scan(value: &Value) -> bool {
        match value {
            Value::String(s) => s == "approve",
            Value::Array(items) => items.iter().any(scan),
            Value::Object(map) => map.values().any(scan),
            _ => false,
        }
    }
    scan(&value)
}

fn approval_question(prompt: &str) -> AskArgs {
    AskArgs {
        questions: vec![AskQuestionArg {
            id: "plan_approval".into(),
            prompt: prompt.into(),
            label: Some("Plan".into()),
            mode: AskModeArg::Single,
            options: vec![
                AskOptionArg {
                    value: "approve".into(),
                    label: "Approve".into(),
                    description: Some("Switch to execute and seed the todo list.".into()),
                    recommended: true,
                },
                AskOptionArg {
                    value: "revise".into(),
                    label: "Revise".into(),
                    description: Some("Stay in plan mode and keep editing.".into()),
                    recommended: false,
                },
            ],
            allow_other: true,
        }],
    }
}

pub struct PlanWriteTool {
    gateway: Arc<dyn PlanGateway>,
}

impl PlanWriteTool {
    pub fn new(gateway: Arc<dyn PlanGateway>) -> Self {
        Self { gateway }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanWriteArgs {
    pub plan_id: String,
    pub title: String,
    pub steps: Vec<PlanDocStep>,
    #[serde(default)]
    pub status: Option<PlanDocStatus>,
}

#[async_trait]
impl TypedTool for PlanWriteTool {
    type Args = PlanWriteArgs;

    fn name(&self) -> &str {
        "plan_write"
    }

    fn description(&self) -> &str {
        "Create or replace a plan document. Steps may declare depends_on and verification."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["plan_id", "title", "steps"],
            "properties": {
                "plan_id": { "type": "string" },
                "title": { "type": "string" },
                "status": { "type": "string", "enum": ["draft", "awaiting_approval", "approved", "abandoned"] },
                "steps": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["id", "content"],
                        "properties": {
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "depends_on": { "type": "array", "items": { "type": "string" } },
                            "verification": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        }
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
        args: PlanWriteArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let previous = self.gateway.read(Some(&args.plan_id)).await?;
        let now = now_ms();
        let doc = PlanDoc {
            plan_id: args.plan_id,
            title: args.title,
            status: args.status.unwrap_or(PlanDocStatus::Draft),
            steps: args.steps,
            created_at: previous.as_ref().map(|d| d.created_at).unwrap_or(now),
            updated_at: now,
        };
        let doc = self.gateway.write(doc).await?;
        ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::PlanDocUpdated {
            doc: doc.clone(),
        });
        Ok(plan_json(&doc))
    }
}

pub struct PlanReadTool {
    gateway: Arc<dyn PlanGateway>,
}

impl PlanReadTool {
    pub fn new(gateway: Arc<dyn PlanGateway>) -> Self {
        Self { gateway }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PlanReadArgs {
    #[serde(default)]
    pub plan_id: Option<String>,
}

#[async_trait]
impl TypedTool for PlanReadTool {
    type Args = PlanReadArgs;

    fn name(&self) -> &str {
        "plan_read"
    }

    fn description(&self) -> &str {
        "Read a plan document by plan_id, or the latest plan when omitted."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "plan_id": { "type": "string" }
            }
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: PlanReadArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        match self.gateway.read(args.plan_id.as_deref()).await? {
            Some(doc) => Ok(plan_json(&doc)),
            None => Err(XyToolError::InvalidArgs("plan not found".into())),
        }
    }
}

pub struct PlanListTool {
    gateway: Arc<dyn PlanGateway>,
}

impl PlanListTool {
    pub fn new(gateway: Arc<dyn PlanGateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TypedTool for PlanListTool {
    type Args = Value;

    fn name(&self) -> &str {
        "plan_list"
    }

    fn description(&self) -> &str {
        "List plan documents with id, title, status and updated_at."
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
        let docs = self.gateway.list().await?;
        let items: Vec<Value> = docs
            .iter()
            .map(|doc| {
                json!({
                    "plan_id": doc.plan_id,
                    "title": doc.title,
                    "status": doc.status,
                    "updated_at": doc.updated_at,
                    "steps": doc.steps.len(),
                })
            })
            .collect();
        Ok(json!({ "plans": items }).to_string())
    }
}

pub struct PlanSubmitTool {
    gateway: Arc<dyn PlanGateway>,
    todo: Arc<dyn AgentTodoGateway>,
    ask: Option<Arc<dyn AskUserGateway>>,
}

impl PlanSubmitTool {
    pub fn new(
        gateway: Arc<dyn PlanGateway>,
        todo: Arc<dyn AgentTodoGateway>,
        ask: Option<Arc<dyn AskUserGateway>>,
    ) -> Self {
        Self { gateway, todo, ask }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanSubmitArgs {
    pub plan_id: String,
}

#[async_trait]
impl TypedTool for PlanSubmitTool {
    type Args = PlanSubmitArgs;

    fn name(&self) -> &str {
        "plan_submit"
    }

    fn description(&self) -> &str {
        "Submit a plan for user approval. Approval switches to execute and seeds the todo list."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["plan_id"],
            "properties": { "plan_id": { "type": "string" } }
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: PlanSubmitArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let Some(ask) = self.ask.as_ref() else {
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                "plan approval requires an ask gateway"
            )));
        };
        let Some(mut doc) = self.gateway.read(Some(&args.plan_id)).await? else {
            return Err(XyToolError::InvalidArgs(format!(
                "unknown plan_id: {}",
                args.plan_id
            )));
        };
        doc.validate().map_err(|e| XyToolError::InvalidArgs(e.0))?;
        doc.status = PlanDocStatus::AwaitingApproval;
        doc.updated_at = now_ms();
        let doc = self.gateway.write(doc).await?;
        let answer = ask
            .prompt(approval_question(&format!(
                "Approve plan {} — {}?",
                doc.plan_id, doc.title
            )))
            .await?;
        let approved = approved(&answer);
        let mut doc = doc;
        doc.updated_at = now_ms();
        if approved {
            doc.status = PlanDocStatus::Approved;
            let doc = self.gateway.write(doc).await?;
            self.gateway
                .set_mode(PlanMode {
                    mode: PlanModeKind::Execute,
                    plan_id: Some(doc.plan_id.clone()),
                    updated_at: now_ms(),
                })
                .await?;
            let items: Vec<TodoItemDraft> = doc
                .steps
                .iter()
                .map(|step| TodoItemDraft {
                    id: Some(step.id.clone()),
                    content: step.content.clone(),
                    status: step.status,
                })
                .collect();
            let todo_seeded = match self.todo.rewrite(items).await {
                Ok(_) => true,
                Err(err) => {
                    log::warn!(
                        target: "xylitol::tools",
                        "plan todo handoff failed: {err}"
                    );
                    false
                }
            };
            ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::PlanDocUpdated {
                doc: doc.clone(),
            });
            ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::PlanModeChanged {
                mode: PlanModeKind::Execute,
                plan_id: Some(doc.plan_id.clone()),
            });
            Ok(json!({
                "approved": true,
                "plan_id": doc.plan_id,
                "mode": "execute",
                "todo_seeded": todo_seeded,
            })
            .to_string())
        } else {
            doc.status = PlanDocStatus::Draft;
            let doc = self.gateway.write(doc).await?;
            self.gateway
                .set_mode(PlanMode {
                    mode: PlanModeKind::Plan,
                    plan_id: Some(doc.plan_id.clone()),
                    updated_at: now_ms(),
                })
                .await?;
            ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::PlanDocUpdated {
                doc: doc.clone(),
            });
            Ok(json!({
                "approved": false,
                "plan_id": doc.plan_id,
                "mode": "plan",
                "answer": answer,
            })
            .to_string())
        }
    }
}

pub struct PlanModeTool {
    gateway: Arc<dyn PlanGateway>,
    ask: Option<Arc<dyn AskUserGateway>>,
}

impl PlanModeTool {
    pub fn new(gateway: Arc<dyn PlanGateway>, ask: Option<Arc<dyn AskUserGateway>>) -> Self {
        Self { gateway, ask }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanModeArgs {
    pub mode: String,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[async_trait]
impl TypedTool for PlanModeTool {
    type Args = PlanModeArgs;

    fn name(&self) -> &str {
        "plan_mode"
    }

    fn description(&self) -> &str {
        "Request a plan-mode switch. Entering plan mode always asks the user."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["mode"],
            "properties": {
                "mode": { "type": "string", "enum": ["plan", "execute"] },
                "plan_id": { "type": "string" },
                "reason": { "type": "string" }
            }
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: PlanModeArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let mode = match args.mode.as_str() {
            "plan" => PlanModeKind::Plan,
            "execute" => PlanModeKind::Execute,
            other => {
                return Err(XyToolError::InvalidArgs(format!(
                    "unknown plan mode: {other}"
                )));
            }
        };
        let approved_plan = match args.plan_id.as_deref() {
            Some(id) => self
                .gateway
                .read(Some(id))
                .await?
                .is_some_and(|doc| doc.status == PlanDocStatus::Approved),
            None => false,
        };
        let needs_ask = mode == PlanModeKind::Plan || !approved_plan;
        if needs_ask {
            let Some(ask) = self.ask.as_ref() else {
                return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "plan mode switch requires an ask gateway"
                )));
            };
            let prompt = match mode {
                PlanModeKind::Plan => "Enter plan mode? The agent will plan before editing.",
                PlanModeKind::Execute => "Switch to execute mode and leave planning?",
            };
            let answer = ask
                .prompt(AskArgs {
                    questions: vec![AskQuestionArg {
                        id: "plan_mode".into(),
                        prompt: prompt.into(),
                        label: Some("Mode".into()),
                        mode: AskModeArg::Single,
                        options: vec![
                            AskOptionArg {
                                value: "approve".into(),
                                label: "Switch".into(),
                                description: args.reason.clone(),
                                recommended: true,
                            },
                            AskOptionArg {
                                value: "revise".into(),
                                label: "Stay".into(),
                                description: None,
                                recommended: false,
                            },
                        ],
                        allow_other: false,
                    }],
                })
                .await?;
            if !approved(&answer) {
                return Ok(json!({
                    "mode": mode_label(mode),
                    "switched": false,
                    "answer": answer,
                })
                .to_string());
            }
        }
        let next = PlanMode {
            mode,
            plan_id: args.plan_id,
            updated_at: now_ms(),
        };
        self.gateway.set_mode(next.clone()).await?;
        ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::PlanModeChanged {
            mode: next.mode,
            plan_id: next.plan_id.clone(),
        });
        Ok(json!({
            "mode": mode_label(next.mode),
            "plan_id": next.plan_id,
            "switched": true,
        })
        .to_string())
    }
}

fn mode_label(mode: PlanModeKind) -> &'static str {
    match mode {
        PlanModeKind::Plan => "plan",
        PlanModeKind::Execute => "execute",
    }
}

pub fn plan_tools(
    gateway: Arc<dyn PlanGateway>,
    todo: Arc<dyn AgentTodoGateway>,
    ask: Option<Arc<dyn AskUserGateway>>,
) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
    vec![
        Arc::new(PlanWriteTool::new(gateway.clone())),
        Arc::new(PlanReadTool::new(gateway.clone())),
        Arc::new(PlanListTool::new(gateway.clone())),
        Arc::new(PlanSubmitTool::new(gateway.clone(), todo, ask.clone())),
        Arc::new(PlanModeTool::new(gateway, ask)),
    ]
}

#[allow(dead_code)]
fn _plan_paths_base() -> PathBuf {
    std::env::temp_dir().join("xylitol-plans")
}
