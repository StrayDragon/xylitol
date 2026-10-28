use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use xylitol_plugin_sdk::TypedTool;
use xylitol_plugin_sdk::ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway};
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::session::{
    PlanDoc, PlanDocStatus, PlanDocStep, PlanMode, PlanModeKind, PlanRef, PlanScope, TodoItemDraft,
    latest_plan_doc, latest_plan_doc_by_id, latest_plan_mode, latest_plan_ref, plan_docs,
};
use xylitol_plugin_sdk::{
    AgentTodoGateway, PlanGateway, XySessionStore, XyToolCtx, XyToolExecutionMode,
};

pub struct SessionPlanGateway {
    store: Arc<dyn XySessionStore>,
    session_id: RwLock<Option<String>>,
    memory: RwLock<HashMap<String, PlanDoc>>,
    memory_mode: RwLock<PlanMode>,
    project_root: PathBuf,
    global_root: PathBuf,
    trusted: bool,
}

impl SessionPlanGateway {
    pub fn new(store: Arc<dyn XySessionStore>, project_root: PathBuf, trusted: bool) -> Arc<Self> {
        let global_root = dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".xylitol");
        Self::new_with_roots(store, project_root, global_root, trusted)
    }

    pub fn new_with_roots(
        store: Arc<dyn XySessionStore>,
        project_root: PathBuf,
        global_root: PathBuf,
        trusted: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            session_id: RwLock::new(None),
            memory: RwLock::new(HashMap::new()),
            memory_mode: RwLock::new(PlanMode::execute()),
            project_root,
            global_root,
            trusted,
        })
    }

    pub async fn bind_session(&self, session_id: Option<String>) {
        *self.session_id.write().await = session_id;
    }

    async fn entries(&self) -> Option<Vec<xylitol_plugin_sdk::session::SessionEntry>> {
        let sid = self.session_id.read().await.clone()?;
        self.store.load_leaf_branch(&sid).await.ok()
    }

    async fn active_ref(&self) -> Option<PlanRef> {
        self.entries()
            .await
            .and_then(|entries| latest_plan_ref(&entries))
    }

    async fn resolve_scope(&self, explicit: Option<PlanScope>) -> PlanScope {
        match explicit {
            Some(scope) => scope,
            None => self
                .active_ref()
                .await
                .map(|plan_ref| plan_ref.scope)
                .unwrap_or(PlanScope::Session),
        }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        std::env::temp_dir()
            .join("xylitol-plans")
            .join(session_id)
            .join("plans")
    }

    fn project_dir(&self) -> PathBuf {
        self.project_root.join(".xylitol").join("plans")
    }

    fn global_dir(&self) -> PathBuf {
        self.global_root.join("plans")
    }

    fn scope_dir(
        &self,
        scope: PlanScope,
        session_id: Option<&str>,
    ) -> Result<PathBuf, XyToolError> {
        match scope {
            PlanScope::Session => {
                let Some(session_id) = session_id else {
                    return Err(XyToolError::InvalidArgs(
                        "session scope requires a bound session".into(),
                    ));
                };
                Ok(self.session_dir(session_id))
            }
            PlanScope::Project => {
                let dir = self.project_dir();
                if !self.trusted {
                    return Err(XyToolError::PermissionDenied(format!(
                        "project plan access requires trust: {}",
                        dir.display()
                    )));
                }
                Ok(dir)
            }
            PlanScope::Global => Ok(self.global_dir()),
        }
    }

    async fn persist_session(&self, doc: &PlanDoc) -> Result<(), XyToolError> {
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
        let dir = self.session_dir(session_id);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let path = dir.join(format!("{}.md", doc.plan_id));
        let _ = std::fs::write(&path, doc.to_markdown());
    }

    async fn persist_file(&self, doc: &PlanDoc) -> Result<(), XyToolError> {
        let sid = self.session_id.read().await.clone();
        let dir = self.scope_dir(doc.scope, sid.as_deref())?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        let path = dir.join(format!("{}.md", doc.plan_id));
        let tmp = dir.join(format!("{}.md.tmp", doc.plan_id));
        let content = doc
            .to_file_markdown()
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        if let Err(e) = std::fs::write(&tmp, content) {
            let _ = std::fs::remove_file(&tmp);
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(e)));
        }
        if let Err(e) = std::fs::rename(&tmp, &path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(e)));
        }
        if let Some(sid) = sid {
            let plan_ref = PlanRef {
                scope: doc.scope,
                plan_id: doc.plan_id.clone(),
                path: Some(path.to_string_lossy().into_owned()),
            };
            self.store
                .append_session_entry(&sid, &plan_ref.to_custom_entry_shell())
                .await
                .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        }
        Ok(())
    }

    fn read_file_path(&self, path: &std::path::Path) -> Result<Option<PlanDoc>, XyToolError> {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Ok(None);
        };
        let doc = PlanDoc::from_file_markdown(&content)
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        Ok(Some(doc))
    }

    async fn read_file(
        &self,
        scope: PlanScope,
        plan_id: Option<&str>,
    ) -> Result<Option<PlanDoc>, XyToolError> {
        let sid = self.session_id.read().await.clone();
        let dir = self.scope_dir(scope, sid.as_deref())?;
        match plan_id {
            Some(id) => {
                let path = dir.join(format!("{id}.md"));
                let Ok(content) = std::fs::read_to_string(&path) else {
                    return Ok(None);
                };
                let doc = PlanDoc::from_file_markdown(&content)
                    .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
                Ok(Some(doc))
            }
            None => Ok(self
                .list_files(&dir)?
                .into_iter()
                .max_by_key(|doc| doc.updated_at)),
        }
    }

    fn list_files(&self, dir: &std::path::Path) -> Result<Vec<PlanDoc>, XyToolError> {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return Ok(Vec::new());
        };
        let mut docs = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Ok(doc) = PlanDoc::from_file_markdown(&content) {
                docs.push(doc);
            }
        }
        Ok(docs)
    }

    async fn list_scope(&self, scope: PlanScope) -> Result<Vec<PlanDoc>, XyToolError> {
        let sid = self.session_id.read().await.clone();
        let dir = self.scope_dir(scope, sid.as_deref())?;
        self.list_files(&dir)
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
    async fn list(&self, scope: Option<PlanScope>) -> Result<Vec<PlanDoc>, XyToolError> {
        let mut docs = match scope {
            Some(scope) => self.list_scope(scope).await?,
            None => {
                let mut docs = self
                    .entries()
                    .await
                    .map(|e| plan_docs(&e))
                    .unwrap_or_default();
                if self.trusted {
                    docs.extend(self.list_scope(PlanScope::Project).await?);
                }
                docs.extend(self.list_scope(PlanScope::Global).await?);
                docs
            }
        };
        let memory = self.memory.read().await;
        for doc in memory.values() {
            if !docs
                .iter()
                .any(|d| d.plan_id == doc.plan_id && d.scope == doc.scope)
            {
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

    async fn read(
        &self,
        scope: Option<PlanScope>,
        plan_id: Option<&str>,
    ) -> Result<Option<PlanDoc>, XyToolError> {
        let scope = self.resolve_scope(scope).await;
        match scope {
            PlanScope::Session => {
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
            PlanScope::Project | PlanScope::Global => {
                let active = self
                    .active_ref()
                    .await
                    .filter(|plan_ref| plan_ref.scope == scope);
                let id = match plan_id {
                    Some(id) => Some(id.to_string()),
                    None => active.as_ref().map(|plan_ref| plan_ref.plan_id.clone()),
                };
                if let Some(path) = active
                    .as_ref()
                    .filter(|plan_ref| Some(&plan_ref.plan_id) == id.as_ref())
                    .and_then(|plan_ref| plan_ref.path.as_deref())
                {
                    return self.read_file_path(std::path::Path::new(path));
                }
                self.read_file(scope, id.as_deref()).await
            }
        }
    }

    async fn write(&self, doc: PlanDoc) -> Result<PlanDoc, XyToolError> {
        doc.validate().map_err(|e| XyToolError::InvalidArgs(e.0))?;
        match doc.scope {
            PlanScope::Session => self.persist_session(&doc).await?,
            PlanScope::Project | PlanScope::Global => self.persist_file(&doc).await?,
        }
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

    async fn active_scope(&self) -> PlanScope {
        self.resolve_scope(None).await
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
    #[serde(default)]
    pub scope: Option<PlanScope>,
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
                "scope": { "type": "string", "enum": ["session", "project", "global"] },
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
        let scope = match args.scope {
            Some(scope) => scope,
            None => self.gateway.active_scope().await,
        };
        let previous = self.gateway.read(Some(scope), Some(&args.plan_id)).await?;
        let now = now_ms();
        let doc = PlanDoc {
            plan_id: args.plan_id,
            title: args.title,
            status: args.status.unwrap_or(PlanDocStatus::Draft),
            scope,
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
    #[serde(default)]
    pub scope: Option<PlanScope>,
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
                "plan_id": { "type": "string" },
                "scope": { "type": "string", "enum": ["session", "project", "global"] }
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
        match self
            .gateway
            .read(args.scope, args.plan_id.as_deref())
            .await?
        {
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

#[derive(Debug, Deserialize, Default)]
pub struct PlanListArgs {
    #[serde(default)]
    pub scope: Option<PlanScope>,
}

#[async_trait]
impl TypedTool for PlanListTool {
    type Args = PlanListArgs;

    fn name(&self) -> &str {
        "plan_list"
    }

    fn description(&self) -> &str {
        "List plan documents with id, title, status and updated_at."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "scope": { "type": "string", "enum": ["session", "project", "global"] }
            }
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: PlanListArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let docs = self.gateway.list(args.scope).await?;
        let items: Vec<Value> = docs
            .iter()
            .map(|doc| {
                json!({
                    "plan_id": doc.plan_id,
                    "title": doc.title,
                    "status": doc.status,
                    "scope": doc.scope,
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
    todo: Option<Arc<dyn AgentTodoGateway>>,
    ask: Option<Arc<dyn AskUserGateway>>,
}

impl PlanSubmitTool {
    pub fn new(
        gateway: Arc<dyn PlanGateway>,
        todo: Option<Arc<dyn AgentTodoGateway>>,
        ask: Option<Arc<dyn AskUserGateway>>,
    ) -> Self {
        Self { gateway, todo, ask }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanSubmitArgs {
    pub plan_id: String,
    #[serde(default)]
    pub scope: Option<PlanScope>,
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
            "properties": {
                "plan_id": { "type": "string" },
                "scope": { "type": "string", "enum": ["session", "project", "global"] }
            }
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
        let Some(mut doc) = self.gateway.read(args.scope, Some(&args.plan_id)).await? else {
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
            let todo_seeded = match self.todo.as_ref() {
                Some(todo) => match todo.rewrite(items).await {
                    Ok(_) => true,
                    Err(err) => {
                        log::warn!(
                            target: "xylitol::tools",
                            "plan todo handoff failed: {err}"
                        );
                        false
                    }
                },
                None => {
                    log::warn!(
                        target: "xylitol::tools",
                        "plan todo handoff unavailable: todo plugin is disabled"
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
                .read(None, Some(id))
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
    todo: Option<Arc<dyn AgentTodoGateway>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::session::{PlanDocStep, PlanScope, TodoStatus};

    fn doc(plan_id: &str, scope: PlanScope, updated_at: u64) -> PlanDoc {
        PlanDoc {
            plan_id: plan_id.into(),
            title: format!("Plan {plan_id}"),
            status: PlanDocStatus::Draft,
            scope,
            steps: vec![PlanDocStep {
                id: "s1".into(),
                content: "do it".into(),
                depends_on: Vec::new(),
                verification: None,
                status: TodoStatus::Pending,
            }],
            created_at: 1,
            updated_at,
        }
    }

    struct Fixture {
        gateway: Arc<SessionPlanGateway>,
        store: Arc<dyn XySessionStore>,
        project: tempfile::TempDir,
        global: tempfile::TempDir,
    }

    async fn fixture(trusted: bool) -> Fixture {
        let store: Arc<dyn XySessionStore> =
            Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        store.create("s1", Some("."), None).await.expect("create");
        let project = tempfile::tempdir().expect("project dir");
        let global = tempfile::tempdir().expect("global dir");
        let gateway = SessionPlanGateway::new_with_roots(
            store.clone(),
            project.path().to_path_buf(),
            global.path().to_path_buf(),
            trusted,
        );
        gateway.bind_session(Some("s1".into())).await;
        Fixture {
            gateway,
            store,
            project,
            global,
        }
    }

    #[tokio::test]
    async fn session_scope_keeps_custom_entry_ssot() {
        let f = fixture(true).await;
        let written = f
            .gateway
            .write(doc("p1", PlanScope::Session, 10))
            .await
            .expect("write");
        assert_eq!(written.scope, PlanScope::Session);
        let entries = f.store.load_leaf_branch("s1").await.expect("entries");
        assert!(
            entries.iter().any(|entry| matches!(
                entry,
                xylitol_plugin_sdk::session::SessionEntry::Custom(custom)
                    if custom.custom_type == xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_DOC
            )),
            "session plan must persist a plan_doc entry"
        );
        let read = f
            .gateway
            .read(Some(PlanScope::Session), Some("p1"))
            .await
            .expect("read")
            .expect("doc");
        assert_eq!(read.plan_id, "p1");
    }

    #[tokio::test]
    async fn project_scope_requires_trust_and_writes_file() {
        let untrusted = fixture(false).await;
        let err = untrusted
            .gateway
            .write(doc("p1", PlanScope::Project, 10))
            .await
            .expect_err("untrusted project must fail");
        assert!(matches!(err, XyToolError::PermissionDenied(_)), "{err:?}");

        let trusted = fixture(true).await;
        trusted
            .gateway
            .write(doc("p1", PlanScope::Project, 10))
            .await
            .expect("write");
        let path = trusted.project.path().join(".xylitol/plans/p1.md");
        assert!(path.is_file(), "project plan file must exist");
        let read = trusted
            .gateway
            .read(Some(PlanScope::Project), Some("p1"))
            .await
            .expect("read")
            .expect("doc");
        assert_eq!(read.scope, PlanScope::Project);
    }

    #[tokio::test]
    async fn global_scope_writes_user_file() {
        let f = fixture(false).await;
        f.gateway
            .write(doc("g1", PlanScope::Global, 20))
            .await
            .expect("global write");
        assert!(f.global.path().join("plans/g1.md").is_file());
        let read = f
            .gateway
            .read(Some(PlanScope::Global), Some("g1"))
            .await
            .expect("read")
            .expect("doc");
        assert_eq!(read.plan_id, "g1");
    }

    #[tokio::test]
    async fn active_ref_resolves_scope_and_resumes() {
        let f = fixture(true).await;
        f.gateway
            .write(doc("p1", PlanScope::Project, 30))
            .await
            .expect("project write");
        assert_eq!(f.gateway.active_scope().await, PlanScope::Project);

        let resumed = SessionPlanGateway::new_with_roots(
            f.store.clone(),
            f.project.path().to_path_buf(),
            f.global.path().to_path_buf(),
            true,
        );
        resumed.bind_session(Some("s1".into())).await;
        let read = resumed.read(None, None).await.expect("read").expect("doc");
        assert_eq!(read.plan_id, "p1");
        assert_eq!(read.scope, PlanScope::Project);
    }

    #[tokio::test]
    async fn list_merges_scopes_with_labels() {
        let f = fixture(true).await;
        f.gateway
            .write(doc("s1-plan", PlanScope::Session, 10))
            .await
            .expect("session");
        f.gateway
            .write(doc("p1", PlanScope::Project, 20))
            .await
            .expect("project");
        f.gateway
            .write(doc("g1", PlanScope::Global, 30))
            .await
            .expect("global");
        let docs = f.gateway.list(None).await.expect("list");
        assert_eq!(docs.len(), 3);
        let scopes: Vec<_> = docs.iter().map(|doc| doc.scope).collect();
        assert!(scopes.contains(&PlanScope::Session));
        assert!(scopes.contains(&PlanScope::Project));
        assert!(scopes.contains(&PlanScope::Global));
    }

    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;
    use xylitol_plugin_sdk::lifecycle::XyEvent;
    use xylitol_plugin_sdk::session::{TodoItem, TodoItemPatch, TodoList};

    struct FakeAsk {
        answer: String,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl AskUserGateway for FakeAsk {
        async fn prompt(&self, _: AskArgs) -> Result<String, XyToolError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.answer.clone())
        }
    }

    #[derive(Default)]
    struct FakeTodo {
        rewrites: Mutex<Vec<Vec<TodoItemDraft>>>,
    }

    #[async_trait]
    impl AgentTodoGateway for FakeTodo {
        async fn list(&self) -> Result<TodoList, XyToolError> {
            Ok(TodoList::default())
        }
        async fn rewrite(&self, items: Vec<TodoItemDraft>) -> Result<TodoList, XyToolError> {
            self.rewrites.lock().unwrap().push(items.clone());
            Ok(TodoList::new(
                items
                    .into_iter()
                    .map(|draft| TodoItem {
                        id: draft.id.unwrap_or_default(),
                        content: draft.content,
                        status: draft.status,
                    })
                    .collect(),
            ))
        }
        async fn update(&self, _: Vec<TodoItemPatch>) -> Result<TodoList, XyToolError> {
            Ok(TodoList::default())
        }
    }

    fn tool_ctx() -> (XyToolCtx, mpsc::UnboundedReceiver<XyEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (XyToolCtx::new("call-1").with_state_event_tx(tx), rx)
    }

    fn drain_events(rx: &mut mpsc::UnboundedReceiver<XyEvent>) -> Vec<XyEvent> {
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        events
    }

    #[tokio::test]
    async fn plan_submit_approves_and_hands_off_to_todo() {
        let f = fixture(true).await;
        f.gateway
            .write(doc("p1", PlanScope::Session, 10))
            .await
            .expect("seed plan");
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"approve"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let todo = Arc::new(FakeTodo::default());
        let tool = PlanSubmitTool::new(f.gateway.clone(), Some(todo.clone()), Some(ask.clone()));
        let (ctx, mut rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanSubmitArgs {
                    plan_id: "p1".into(),
                    scope: Some(PlanScope::Session),
                },
            )
            .await
            .expect("submit");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["approved"], true);
        assert_eq!(value["todo_seeded"], true);
        assert_eq!(ask.calls.load(Ordering::SeqCst), 1);
        let stored = f
            .gateway
            .read(Some(PlanScope::Session), Some("p1"))
            .await
            .expect("read")
            .expect("plan");
        assert_eq!(stored.status, PlanDocStatus::Approved);
        assert_eq!(
            f.gateway.mode().await.expect("mode").mode,
            PlanModeKind::Execute
        );
        let rewrites = todo.rewrites.lock().unwrap();
        assert_eq!(rewrites.len(), 1);
        assert_eq!(rewrites[0][0].id.as_deref(), Some("s1"));
        let events = drain_events(&mut rx);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, XyEvent::PlanDocUpdated { .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, XyEvent::PlanModeChanged { .. }))
        );
    }

    #[tokio::test]
    async fn plan_submit_rejection_returns_to_draft() {
        let f = fixture(true).await;
        f.gateway
            .write(doc("p1", PlanScope::Session, 10))
            .await
            .expect("seed plan");
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"revise"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let todo = Arc::new(FakeTodo::default());
        let tool = PlanSubmitTool::new(f.gateway.clone(), Some(todo.clone()), Some(ask));
        let (ctx, mut rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanSubmitArgs {
                    plan_id: "p1".into(),
                    scope: Some(PlanScope::Session),
                },
            )
            .await
            .expect("submit");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["approved"], false);
        let stored = f
            .gateway
            .read(Some(PlanScope::Session), Some("p1"))
            .await
            .expect("read")
            .expect("plan");
        assert_eq!(stored.status, PlanDocStatus::Draft);
        assert_eq!(
            f.gateway.mode().await.expect("mode").mode,
            PlanModeKind::Plan
        );
        assert!(todo.rewrites.lock().unwrap().is_empty());
        let events = drain_events(&mut rx);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, XyEvent::PlanDocUpdated { .. }))
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, XyEvent::PlanModeChanged { .. })),
            "rejection must not publish a mode change"
        );
    }

    #[tokio::test]
    async fn plan_submit_errors_without_plan_or_ask() {
        let f = fixture(true).await;
        let tool = PlanSubmitTool::new(f.gateway.clone(), None, None);
        let (ctx, _rx) = tool_ctx();
        let err = tool
            .execute_typed(
                &ctx,
                PlanSubmitArgs {
                    plan_id: "missing".into(),
                    scope: Some(PlanScope::Session),
                },
            )
            .await
            .expect_err("unknown plan");
        assert!(matches!(err, XyToolError::ExecutionFailed(_)), "{err:?}");

        f.gateway
            .write(doc("p1", PlanScope::Session, 10))
            .await
            .expect("seed plan");
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"approve"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let tool = PlanSubmitTool::new(f.gateway.clone(), None, None);
        let err = tool
            .execute_typed(
                &ctx,
                PlanSubmitArgs {
                    plan_id: "p1".into(),
                    scope: Some(PlanScope::Session),
                },
            )
            .await
            .expect_err("no ask gateway");
        assert!(matches!(err, XyToolError::ExecutionFailed(_)), "{err:?}");
        drop(ask);
    }

    #[tokio::test]
    async fn plan_submit_rejects_unknown_plan_id() {
        let f = fixture(true).await;
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"approve"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let tool = PlanSubmitTool::new(f.gateway.clone(), None, Some(ask));
        let (ctx, _rx) = tool_ctx();
        let err = tool
            .execute_typed(
                &ctx,
                PlanSubmitArgs {
                    plan_id: "missing".into(),
                    scope: Some(PlanScope::Session),
                },
            )
            .await
            .expect_err("unknown plan");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err:?}");
    }

    #[tokio::test]
    async fn plan_mode_switches_to_plan_with_ask() {
        let f = fixture(true).await;
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"approve"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let tool = PlanModeTool::new(f.gateway.clone(), Some(ask.clone()));
        let (ctx, mut rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanModeArgs {
                    mode: "plan".into(),
                    plan_id: None,
                    reason: Some("research".into()),
                },
            )
            .await
            .expect("switch");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["switched"], true);
        assert_eq!(value["mode"], "plan");
        assert_eq!(ask.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            f.gateway.mode().await.expect("mode").mode,
            PlanModeKind::Plan
        );
        let events = drain_events(&mut rx);
        assert!(events.iter().any(|event| matches!(
            event,
            XyEvent::PlanModeChanged { mode, .. } if *mode == PlanModeKind::Plan
        )));
    }

    #[tokio::test]
    async fn plan_mode_declines_switch() {
        let f = fixture(true).await;
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"revise"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let tool = PlanModeTool::new(f.gateway.clone(), Some(ask));
        let (ctx, _rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanModeArgs {
                    mode: "plan".into(),
                    plan_id: None,
                    reason: None,
                },
            )
            .await
            .expect("switch");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["switched"], false);
        assert_eq!(
            f.gateway.mode().await.expect("mode").mode,
            PlanModeKind::Execute
        );
    }

    #[tokio::test]
    async fn plan_mode_skips_ask_for_approved_plan() {
        let f = fixture(true).await;
        let mut approved = doc("p1", PlanScope::Session, 10);
        approved.status = PlanDocStatus::Approved;
        f.gateway.write(approved).await.expect("seed approved");
        let tool = PlanModeTool::new(f.gateway.clone(), None);
        let (ctx, _rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanModeArgs {
                    mode: "execute".into(),
                    plan_id: Some("p1".into()),
                    reason: None,
                },
            )
            .await
            .expect("switch");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["switched"], true);
        assert_eq!(
            f.gateway.mode().await.expect("mode").mode,
            PlanModeKind::Execute
        );
    }

    #[tokio::test]
    async fn plan_mode_asks_for_unapproved_execute_and_rejects_unknown_mode() {
        let f = fixture(true).await;
        f.gateway
            .write(doc("p1", PlanScope::Session, 10))
            .await
            .expect("seed draft");
        let ask = Arc::new(FakeAsk {
            answer: r#"{"plan_mode":"approve"}"#.into(),
            calls: AtomicUsize::new(0),
        });
        let tool = PlanModeTool::new(f.gateway.clone(), Some(ask.clone()));
        let (ctx, _rx) = tool_ctx();
        let out = tool
            .execute_typed(
                &ctx,
                PlanModeArgs {
                    mode: "execute".into(),
                    plan_id: Some("p1".into()),
                    reason: None,
                },
            )
            .await
            .expect("switch");
        let value: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(value["switched"], true);
        assert_eq!(ask.calls.load(Ordering::SeqCst), 1);

        let err = tool
            .execute_typed(
                &ctx,
                PlanModeArgs {
                    mode: "bogus".into(),
                    plan_id: None,
                    reason: None,
                },
            )
            .await
            .expect_err("unknown mode");
        assert!(matches!(err, XyToolError::InvalidArgs(_)), "{err:?}");
    }

    #[tokio::test]
    async fn atomic_write_leaves_no_partial_file() {
        let f = fixture(true).await;
        let dir = f.project.path().join(".xylitol/plans");
        std::fs::create_dir_all(dir.join("p1.md")).expect("blocking directory");
        let err = f
            .gateway
            .write(doc("p1", PlanScope::Project, 10))
            .await
            .expect_err("rename onto a directory must fail");
        assert!(matches!(err, XyToolError::ExecutionFailed(_)), "{err:?}");
        assert!(
            !dir.join("p1.md.tmp").exists(),
            "failed atomic write must remove the temp file"
        );
    }
}
