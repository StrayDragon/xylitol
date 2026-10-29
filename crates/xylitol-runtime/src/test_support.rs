use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use xylitol_plugin_sdk::error::{XyError, XySessionError, XySessionStoreError, XyToolError};
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::message::XyLlmMessage;
use xylitol_plugin_sdk::model::XyToolSchema;
use xylitol_plugin_sdk::session::{ForkPosition, SessionContext, SessionEntry};
use xylitol_plugin_sdk::{
    XyEventSink, XyGenerateOptions, XyModel, XyModelBuilder, XySessionStore, XyStream, XyTool,
    XyToolCtx,
};

use crate::agent::tools::ToolSet;

pub(crate) struct MemoryStore {
    sessions: Mutex<HashMap<String, Vec<SessionEntry>>>,
    leaves: Mutex<HashMap<String, Option<String>>>,
    load_leaf_branch_calls: AtomicUsize,
}

impl MemoryStore {
    pub(crate) fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            leaves: Mutex::new(HashMap::new()),
            load_leaf_branch_calls: AtomicUsize::new(0),
        }
    }

    pub(crate) fn load_leaf_branch_calls(&self) -> usize {
        self.load_leaf_branch_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl XySessionStore for MemoryStore {
    async fn exists(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .map(|m| m.contains_key(session_id))
            .unwrap_or(false)
    }

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.load_leaf_branch_calls.fetch_add(1, Ordering::SeqCst);
        self.load_entries(session_id).await
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.sessions
            .lock()
            .ok()
            .and_then(|m| m.get(session_id).cloned())
            .ok_or_else(|| XySessionStoreError::not_found(session_id))
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?;
        map.entry(session_id.to_string())
            .or_default()
            .push(entry.clone());
        Ok(())
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError> {
        let _ = self.load_entries(session_id).await?;
        Ok(SessionContext {
            messages: Vec::new(),
            thinking_level: String::new(),
            model: None,
        })
    }

    async fn create(
        &self,
        id: &str,
        _cwd: Option<&str>,
        _parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        self.sessions
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
            .entry(id.to_string())
            .or_default();
        Ok(())
    }

    async fn fork(
        &self,
        _parent_id: &str,
        _child_id: &str,
        at_entry_id: &str,
        _position: ForkPosition,
    ) -> Result<(), XySessionError> {
        Err(XySessionError::EntryNotFound {
            entry_id: at_entry_id.into(),
        })
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        if let Ok(mut m) = self.leaves.lock() {
            m.insert(session_id.to_string(), entry_id.map(str::to_string));
        }
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        self.leaves
            .lock()
            .ok()
            .and_then(|m| m.get(session_id).cloned().flatten())
    }
}

pub(crate) struct NoopSink;

#[async_trait]
impl XyEventSink for NoopSink {
    async fn emit(&self, _event: &XyEvent) {}
}

struct DummyModel;

#[async_trait]
impl XyModel for DummyModel {
    fn name(&self) -> &str {
        "dummy"
    }

    async fn generate_stream(
        &self,
        _messages: Vec<XyLlmMessage>,
        _tools: &[XyToolSchema],
        _stream: bool,
        _options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

pub(crate) fn dummy_model_builder() -> XyModelBuilder {
    Arc::new(|_| Arc::new(DummyModel) as Arc<dyn XyModel>)
}

struct NamedTool(&'static str);

#[async_trait]
impl XyTool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "fixture"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    async fn execute(&self, _: &XyToolCtx, _: serde_json::Value) -> Result<String, XyToolError> {
        Ok("ok".into())
    }
}

pub(crate) fn named_tool(name: &'static str) -> Arc<dyn XyTool> {
    Arc::new(NamedTool(name))
}

pub(crate) fn named_tools(names: &[&'static str]) -> ToolSet {
    ToolSet::from_iter(names.iter().copied().map(named_tool))
}

pub(crate) fn fixture_tools() -> ToolSet {
    named_tools(&["read", "todo_rewrite", "todo_update"])
}
