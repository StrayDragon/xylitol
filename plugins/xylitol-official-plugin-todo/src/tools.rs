use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use xylitol_plugin_sdk::TypedTool;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::session::{
    TodoItemDraft, TodoItemPatch, TodoList, apply_todo_patches, latest_agent_todo,
    normalize_rewrite_items,
};
use xylitol_plugin_sdk::{AgentTodoGateway, XySessionStore, XyToolCtx, XyToolExecutionMode};

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

    async fn update(&self, patches: Vec<TodoItemPatch>) -> Result<TodoList, XyToolError> {
        let current = self.load_current().await?;
        let list =
            apply_todo_patches(&current, &patches).map_err(|e| XyToolError::InvalidArgs(e.0))?;
        self.persist(&list).await?;
        Ok(list)
    }

    async fn bind_session(&self, session_id: Option<String>) {
        SessionAgentTodoGateway::bind_session(self, session_id).await;
    }
}

fn list_json(list: &TodoList) -> String {
    serde_json::to_string(&list.to_data_value()).unwrap_or_else(|_| r#"{"items":[]}"#.into())
}

const TODO_STATUS_ENUM: [&str; 3] = ["pending", "in_progress", "completed"];

const TODO_UPDATE_GUIDELINES: &[&str] = &[
    "todo_rewrite for a new/replaced plan; todo_update to tick existing ids (batch in items[]). Do not rewrite the list to mark one item done. The current list is under <todo> in the <agent_status_bar> message when non-empty.",
];

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
        "Replace the whole checklist (new plan or []). Do not tick one item."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["content"],
                        "properties": {
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": TODO_STATUS_ENUM }
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
        args: TodoRewriteArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        let list = self.gateway.rewrite(args.items).await?;
        ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::TodoUpdated {
            list: list.clone(),
        });
        Ok(list_json(&list))
    }
}

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
    pub items: Vec<TodoItemPatch>,
}

#[async_trait]
impl TypedTool for TodoUpdateTool {
    type Args = TodoUpdateArgs;

    fn name(&self) -> &str {
        "todo_update"
    }

    fn description(&self) -> &str {
        "Patch existing items by id. items[{id, status?, content?, after_id?}]."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["items"],
            "properties": {
                "items": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["id"],
                        "properties": {
                            "id": { "type": "string" },
                            "status": { "type": "string", "enum": TODO_STATUS_ENUM },
                            "content": { "type": "string" },
                            "after_id": { "type": "string" }
                        }
                    }
                }
            }
        })
    }

    fn prompt_guidelines(&self) -> &[&str] {
        TODO_UPDATE_GUIDELINES
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
        let list = self.gateway.update(args.items).await?;
        ctx.publish_state(xylitol_plugin_sdk::lifecycle::XyEvent::TodoUpdated {
            list: list.clone(),
        });
        Ok(list_json(&list))
    }
}

pub fn todo_tools(gateway: Arc<dyn AgentTodoGateway>) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
    vec![
        Arc::new(TodoRewriteTool::new(gateway.clone())),
        Arc::new(TodoUpdateTool::new(gateway)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::session::{CUSTOM_TYPE_AGENT_TODO, SessionEntry, TodoStatus};
    use xylitol_plugin_sdk::{XySessionStore, XyTool};

    struct MemStore {
        entries: tokio::sync::Mutex<Vec<SessionEntry>>,
    }

    #[async_trait::async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _id: &str) -> bool {
            true
        }
        async fn load_entries(
            &self,
            _id: &str,
        ) -> Result<Vec<SessionEntry>, xylitol_plugin_sdk::error::XySessionStoreError> {
            Ok(self.entries.lock().await.clone())
        }
        async fn append_session_entry(
            &self,
            _id: &str,
            entry: &SessionEntry,
        ) -> Result<(), xylitol_plugin_sdk::error::XySessionStoreError> {
            self.entries.lock().await.push(entry.clone());
            Ok(())
        }
        async fn build_session_context(
            &self,
            _id: &str,
        ) -> Result<
            xylitol_plugin_sdk::session::SessionContext,
            xylitol_plugin_sdk::error::XySessionStoreError,
        > {
            Err(xylitol_plugin_sdk::error::XySessionStoreError::unsupported(
                "build_session_context",
            ))
        }
        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), xylitol_plugin_sdk::error::XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            at_entry_id: &str,
            _position: xylitol_plugin_sdk::session::ForkPosition,
        ) -> Result<(), xylitol_plugin_sdk::error::XySessionError> {
            Err(xylitol_plugin_sdk::error::XySessionError::EntryNotFound {
                entry_id: at_entry_id.into(),
            })
        }
        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}
        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    async fn bound_gw() -> (
        Arc<SessionAgentTodoGateway>,
        Arc<dyn XySessionStore>,
        String,
    ) {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore {
            entries: tokio::sync::Mutex::new(Vec::new()),
        });
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
        assert!(
            v["items"][1]["id"].as_str().unwrap().starts_with("t_"),
            "omitted id must be minted: {}",
            v["items"][1]["id"]
        );
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

    #[tokio::test]
    async fn mutation_publishes_typed_todo_updated() {
        use tokio::sync::mpsc::unbounded_channel;
        use xylitol_plugin_sdk::lifecycle::XyEvent;

        let (gw, _, _) = bound_gw().await;
        let (tx, mut rx) = unbounded_channel::<XyEvent>();
        let rewrite = TodoRewriteTool::new(gw.clone());
        let ctx = XyToolCtx::new("c1").with_state_event_tx(tx);
        rewrite
            .execute(
                &ctx,
                json!({"items": [{"id": "a", "content": "one"}, {"id": "b", "content": "two"}]}),
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
        assert_eq!(
            xylitol_plugin_sdk::TypedTool::execution_mode(&update),
            XyToolExecutionMode::Sequential
        );
        update
            .execute(&ctx, json!({"items": [{"id": "a", "status": "completed"}]}))
            .await
            .unwrap();
        match rx.recv().await {
            Some(XyEvent::TodoUpdated { list }) => {
                assert_eq!(list.items[0].status, TodoStatus::Completed);
            }
            other => panic!("expected TodoUpdated, got {other:?}"),
        }

        let bare = XyToolCtx::new("c2");
        let (gw2, _, _) = bound_gw().await;
        TodoRewriteTool::new(gw2)
            .execute(&bare, json!({"items": [{"content": "x"}]}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn clear_rewrite_publishes_empty_list() {
        use tokio::sync::mpsc::unbounded_channel;
        use xylitol_plugin_sdk::lifecycle::XyEvent;

        let (gw, _, _) = bound_gw().await;
        let (tx, mut rx) = unbounded_channel::<XyEvent>();
        let ctx = XyToolCtx::new("c1").with_state_event_tx(tx);
        let tool = TodoRewriteTool::new(gw);
        tool.execute(&ctx, json!({"items": [{"content": "a"}]}))
            .await
            .unwrap();
        tool.execute(&ctx, json!({"items": []})).await.unwrap();
        let _ = rx.recv().await;
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
                json!({"items": [{"id": "nope", "status": "completed"}]}),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("unknown todo id"));
        assert_eq!(store.load_leaf_branch(&sid).await.unwrap().len(), before);
    }

    #[tokio::test]
    async fn old_top_level_update_shape_rejects() {
        let (gw, _, _) = bound_gw().await;
        let err = TodoUpdateTool::new(gw)
            .execute(
                &XyToolCtx::new("c"),
                json!({"id": "a", "status": "completed"}),
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("invalid") || err.to_string().contains("items"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn dual_in_progress_rewrite_persists() {
        let (gw, store, sid) = bound_gw().await;
        let tool = TodoRewriteTool::new(gw);
        let out = tool
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
            .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["items"].as_array().unwrap().len(), 2);
        assert_eq!(v["items"][0]["status"], "in_progress");
        assert_eq!(v["items"][1]["status"], "in_progress");
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

    #[test]
    fn update_has_guideline() {
        let gw = SessionAgentTodoGateway::new(Arc::new(MemStore {
            entries: tokio::sync::Mutex::new(Vec::new()),
        }));
        let tool = TodoUpdateTool::new(gw);
        let g = TypedTool::prompt_guidelines(&tool);
        assert!(!g.is_empty());
        assert!(g[0].contains("<agent_status_bar>"));
        assert!(g[0].contains("<todo>"));
    }
}
