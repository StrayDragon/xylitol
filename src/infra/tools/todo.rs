//! Builtin Todo tools (`todo_list` / `todo_rewrite` / `todo_update`) + session gateway.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use super::typed::TypedTool;
use crate::protocol::error::XyToolError;
use crate::protocol::ports::{AgentTodoGateway, XySessionStore, XyToolCtx, XyToolExecutionMode};
use crate::protocol::session::{
    TodoItemDraft, TodoList, TodoStatus, apply_todo_update, latest_agent_todo,
    normalize_rewrite_items,
};

/// Store-backed Todo SSOT with interior-mutable active session id.
///
/// When `session_id` is unset, mutates stay in an in-memory fallback (unit tests /
/// pre-bind). Product paths bind the session via [`Self::bind_session`].
pub struct SessionAgentTodoGateway {
    store: Arc<dyn XySessionStore>,
    session_id: RwLock<Option<String>>,
    memory: RwLock<TodoList>,
}

impl SessionAgentTodoGateway {
    pub fn new(store: Arc<dyn XySessionStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            session_id: RwLock::new(None),
            memory: RwLock::new(TodoList::default()),
        })
    }

    /// Bind (or clear) the active session for subsequent tool calls.
    pub async fn bind_session(&self, session_id: Option<String>) {
        *self.session_id.write().await = session_id;
    }

    pub async fn active_session_id(&self) -> Option<String> {
        self.session_id.read().await.clone()
    }

    async fn load_current(&self) -> Result<TodoList, XyToolError> {
        let Some(sid) = self.session_id.read().await.clone() else {
            return Ok(self.memory.read().await.clone());
        };
        let branch = self
            .store
            .load_leaf_branch(&sid)
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        Ok(latest_agent_todo(&branch).unwrap_or_default())
    }

    async fn persist(&self, list: &TodoList) -> Result<(), XyToolError> {
        let Some(sid) = self.session_id.read().await.clone() else {
            *self.memory.write().await = list.clone();
            return Ok(());
        };
        self.store
            .append_session_entry(&sid, &list.to_custom_entry_shell())
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        *self.memory.write().await = list.clone();
        Ok(())
    }
}

#[async_trait]
impl AgentTodoGateway for SessionAgentTodoGateway {
    async fn list(&self) -> Result<TodoList, XyToolError> {
        self.load_current().await
    }

    async fn rewrite(&self, items: Vec<TodoItemDraft>) -> Result<TodoList, XyToolError> {
        let list = normalize_rewrite_items(items).map_err(|e| XyToolError::InvalidArgs(e.0))?;
        self.persist(&list).await?;
        Ok(list)
    }

    async fn update(
        &self,
        id: &str,
        status: Option<TodoStatus>,
        content: Option<String>,
    ) -> Result<TodoList, XyToolError> {
        let current = self.load_current().await?;
        let list = apply_todo_update(&current, id, status, content)
            .map_err(|e| XyToolError::InvalidArgs(e.0))?;
        self.persist(&list).await?;
        Ok(list)
    }
}

fn list_json(list: &TodoList) -> String {
    serde_json::to_string(&list.to_data_value()).unwrap_or_else(|_| r#"{"items":[]}"#.into())
}

// ── todo_list ───────────────────────────────────────────────────────

pub struct TodoListTool {
    gateway: Arc<dyn AgentTodoGateway>,
}

impl TodoListTool {
    pub fn new(gateway: Arc<dyn AgentTodoGateway>) -> Self {
        Self { gateway }
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoListArgs {}

#[async_trait]
impl TypedTool for TodoListTool {
    type Args = TodoListArgs;

    fn name(&self) -> &str {
        "todo_list"
    }

    fn description(&self) -> &str {
        "Return the current session Todo checklist (full list). Read-only; does not mutate."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        _args: TodoListArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let list = self.gateway.list().await?;
        Ok(list_json(&list))
    }
}

// ── todo_rewrite ────────────────────────────────────────────────────

pub struct TodoRewriteTool {
    gateway: Arc<dyn AgentTodoGateway>,
}

impl TodoRewriteTool {
    pub fn new(gateway: Arc<dyn AgentTodoGateway>) -> Self {
        Self { gateway }
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoRewriteArgs {
    pub items: Vec<TodoItemDraft>,
}

#[async_trait]
impl TypedTool for TodoRewriteTool {
    type Args = TodoRewriteArgs;

    fn name(&self) -> &str {
        "todo_rewrite"
    }

    fn description(&self) -> &str {
        "Replace the entire session Todo checklist. Returns the full written list. At most one item may be in_progress."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "description": "Full replacement list (empty clears)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Stable id; omitted → server generates"
                            },
                            "content": {
                                "type": "string",
                                "description": "Non-empty task text"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "cancelled"],
                                "description": "Defaults to pending"
                            }
                        },
                        "required": ["content"]
                    }
                }
            },
            "required": ["items"]
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: TodoRewriteArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let list = self.gateway.rewrite(args.items).await?;
        // atd13: typed live projection straight from the SSOT mutation point.
        ctx.publish_state(crate::protocol::lifecycle::XyEvent::TodoUpdated {
            list: list.clone(),
        });
        Ok(list_json(&list))
    }
}

// ── todo_update ─────────────────────────────────────────────────────

pub struct TodoUpdateTool {
    gateway: Arc<dyn AgentTodoGateway>,
}

impl TodoUpdateTool {
    pub fn new(gateway: Arc<dyn AgentTodoGateway>) -> Self {
        Self { gateway }
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoUpdateArgs {
    pub id: String,
    pub status: Option<TodoStatus>,
    pub content: Option<String>,
}

#[async_trait]
impl TypedTool for TodoUpdateTool {
    type Args = TodoUpdateArgs;

    fn name(&self) -> &str {
        "todo_update"
    }

    fn description(&self) -> &str {
        "Update one Todo item by id (status and/or content). Returns the full list. Rejects unknown id or dual in_progress."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Todo item id" },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "cancelled"]
                },
                "content": { "type": "string", "description": "Replacement content" }
            },
            "required": ["id"]
        })
    }

    fn execution_mode(&self) -> XyToolExecutionMode {
        XyToolExecutionMode::Sequential
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: TodoUpdateArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let list = self
            .gateway
            .update(&args.id, args.status, args.content)
            .await?;
        // atd13: typed live projection straight from the SSOT mutation point.
        ctx.publish_state(crate::protocol::lifecycle::XyEvent::TodoUpdated {
            list: list.clone(),
        });
        Ok(list_json(&list))
    }
}

/// Three Todo tools sharing `gateway`.
pub fn todo_tools(
    gateway: Arc<dyn AgentTodoGateway>,
) -> Vec<Arc<dyn crate::protocol::ports::XyTool>> {
    vec![
        Arc::new(TodoListTool::new(gateway.clone())),
        Arc::new(TodoRewriteTool::new(gateway.clone())),
        Arc::new(TodoUpdateTool::new(gateway)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;
    use crate::protocol::ports::XyTool;
    use crate::protocol::session::{CUSTOM_TYPE_AGENT_TODO, SessionEntry};

    async fn bound_gw() -> (
        Arc<SessionAgentTodoGateway>,
        Arc<dyn XySessionStore>,
        String,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().to_path_buf());
        let store: Arc<dyn XySessionStore> = Arc::new(mgr);
        let sid = "s-todo".to_string();
        store.create(&sid, Some("/tmp"), None).await.unwrap();
        let gw = SessionAgentTodoGateway::new(store.clone());
        gw.bind_session(Some(sid.clone())).await;
        (gw, store, sid)
    }

    #[tokio::test]
    async fn rewrite_returns_full_list_and_persists() {
        let (gw, store, sid) = bound_gw().await;
        let tool = TodoRewriteTool::new(gw);
        let ctx = XyToolCtx::new("c1");
        let out = tool
            .execute(
                &ctx,
                json!({
                    "items": [
                        {"id": "a", "content": "one", "status": "completed"},
                        {"content": "two", "status": "in_progress"}
                    ]
                }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["items"].as_array().unwrap().len(), 2);
        let branch = store.load_leaf_branch(&sid).await.unwrap();
        let customs: Vec<_> = branch
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Custom(c) if c.custom_type == CUSTOM_TYPE_AGENT_TODO => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(customs.len(), 1);
        assert_eq!(customs[0].data["items"], v["items"]);
    }

    /// atd13: successful writes publish a typed TodoUpdated event through the
    /// ctx uplink; todo_list (read-only) publishes nothing; no uplink = silent.
    #[tokio::test]
    async fn mutation_publishes_typed_todo_updated() {
        use crate::protocol::lifecycle::XyEvent;
        use tokio::sync::mpsc::unbounded_channel;

        let (gw, _, _) = bound_gw().await;
        let (tx, mut rx) = unbounded_channel::<XyEvent>();
        let rewrite = TodoRewriteTool::new(gw.clone());
        let ctx = XyToolCtx::new("c1").with_state_event_tx(tx);
        rewrite
            .execute(
                &ctx,
                json!({"items": [{"id": "a", "content": "one"}, {"content": "two"}]}),
            )
            .await
            .unwrap();
        match rx.recv().await {
            Some(XyEvent::TodoUpdated { list }) => {
                assert_eq!(list.items.len(), 2);
                assert_eq!(list.items[0].id, "a");
            }
            other => panic!("expected TodoUpdated, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "rewrite must publish exactly one event"
        );

        let update = TodoUpdateTool::new(gw.clone());
        update
            .execute(&ctx, json!({"id": "a", "status": "completed"}))
            .await
            .unwrap();
        match rx.recv().await {
            Some(XyEvent::TodoUpdated { list }) => {
                assert_eq!(list.items[0].status, crate::protocol::session::TodoStatus::Completed);
            }
            other => panic!("expected TodoUpdated, got {other:?}"),
        }

        let list_tool = TodoListTool::new(gw);
        list_tool.execute(&ctx, json!({})).await.unwrap();
        assert!(
            rx.try_recv().is_err(),
            "read-only todo_list MUST NOT publish"
        );

        // Unbound ctx (no uplink) is a silent no-op.
        let bare = XyToolCtx::new("c2");
        let (gw2, _, _) = bound_gw().await;
        TodoRewriteTool::new(gw2)
            .execute(&bare, json!({"items": [{"content": "x"}]}))
            .await
            .unwrap();
    }

    /// atd13: clearing via empty rewrite publishes an empty-list event.
    #[tokio::test]
    async fn clear_rewrite_publishes_empty_list() {
        use crate::protocol::lifecycle::XyEvent;
        use tokio::sync::mpsc::unbounded_channel;

        let (gw, _, _) = bound_gw().await;
        let (tx, mut rx) = unbounded_channel::<XyEvent>();
        let ctx = XyToolCtx::new("c1").with_state_event_tx(tx);
        let tool = TodoRewriteTool::new(gw);
        tool.execute(&ctx, json!({"items": [{"content": "a"}]}))
            .await
            .unwrap();
        tool.execute(&ctx, json!({"items": []})).await.unwrap();
        let _ = rx.recv().await; // first write
        match rx.recv().await {
            Some(XyEvent::TodoUpdated { list }) => assert!(list.is_empty()),
            other => panic!("expected TodoUpdated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn update_unknown_id_rejects_without_snapshot() {
        let (gw, store, sid) = bound_gw().await;
        gw.rewrite(vec![TodoItemDraft {
            id: Some("a".into()),
            content: "x".into(),
            status: TodoStatus::Pending,
        }])
        .await
        .unwrap();
        let before = store.load_leaf_branch(&sid).await.unwrap().len();
        let tool = TodoUpdateTool::new(gw);
        let err = tool
            .execute(
                &XyToolCtx::new("c"),
                json!({"id": "nope", "status": "completed"}),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("unknown todo id"));
        assert_eq!(store.load_leaf_branch(&sid).await.unwrap().len(), before);
    }

    #[tokio::test]
    async fn dual_in_progress_rejects() {
        let (gw, _, _) = bound_gw().await;
        let tool = TodoRewriteTool::new(gw);
        let err = tool
            .execute(
                &XyToolCtx::new("c"),
                json!({
                    "items": [
                        {"content": "a", "status": "in_progress"},
                        {"content": "b", "status": "in_progress"}
                    ]
                }),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("in_progress"));
    }

    #[tokio::test]
    async fn todo_list_readonly() {
        let (gw, store, sid) = bound_gw().await;
        gw.rewrite(vec![TodoItemDraft {
            id: Some("a".into()),
            content: "x".into(),
            status: TodoStatus::Pending,
        }])
        .await
        .unwrap();
        let before = store.load_leaf_branch(&sid).await.unwrap().len();
        let out = TodoListTool::new(gw)
            .execute(&XyToolCtx::new("c"), json!({}))
            .await
            .unwrap();
        assert!(out.contains("\"id\":\"a\""));
        assert_eq!(store.load_leaf_branch(&sid).await.unwrap().len(), before);
    }
}
