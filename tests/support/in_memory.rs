//! In-memory session store for agent unit tests.
//!
//! Implements [`XySessionStore`] with a `HashMap` so the agent loop can be
//! tested without creating a real session directory.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::session_types::{SessionContext, SessionEntry};
use crate::runtime_protocol::XySessionStore;

/// In-memory session storage backed by a `HashMap` of typed entries.
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<String, SessionState>>,
}

struct SessionState {
    entries: Vec<Value>,
    leaf_id: Option<String>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl XySessionStore for InMemorySessionStore {
    async fn exists(&self, session_id: &str) -> bool {
        let map = self.sessions.lock().unwrap();
        map.contains_key(session_id)
    }

    async fn load_entries(&self, session_id: &str) -> Result<Vec<SessionEntry>, String> {
        let map = self.sessions.lock().unwrap();
        let state = map
            .get(session_id)
            .ok_or_else(|| format!("session '{session_id}' not found"))?;
        state
            .entries
            .iter()
            .map(|v| {
                serde_json::from_value::<SessionEntry>(v.clone())
                    .map_err(|e| format!("deserialize session entry: {e}"))
            })
            .collect()
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), String> {
        let mut map = self.sessions.lock().unwrap();
        let state = map
            .get_mut(session_id)
            .ok_or_else(|| format!("session '{session_id}' not found"))?;
        let value =
            serde_json::to_value(entry).map_err(|e| format!("serialize session entry: {e}"))?;
        state.entries.push(value);
        Ok(())
    }

    async fn build_session_context(&self, session_id: &str) -> Result<SessionContext, String> {
        let map = self.sessions.lock().unwrap();
        if !map.contains_key(session_id) {
            return Err(format!("session '{session_id}' not found"));
        }
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
    ) -> Result<(), String> {
        let mut map = self.sessions.lock().unwrap();
        map.entry(id.to_string()).or_insert_with(|| SessionState {
            entries: Vec::new(),
            leaf_id: None,
        });
        Ok(())
    }

    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        _at_entry_id: &str,
    ) -> Result<(), String> {
        let entries = self.load_entries(parent_id).await?;
        let mut map = self.sessions.lock().unwrap();
        map.insert(
            child_id.to_string(),
            SessionState {
                entries: entries
                    .iter()
                    .map(|e| serde_json::to_value(e).unwrap_or_default())
                    .collect(),
                leaf_id: None,
            },
        );
        Ok(())
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        let mut map = self.sessions.lock().unwrap();
        if let Some(state) = map.get_mut(session_id) {
            state.leaf_id = entry_id.map(str::to_string);
        }
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        let map = self.sessions.lock().unwrap();
        map.get(session_id).and_then(|s| s.leaf_id.clone())
    }
}
