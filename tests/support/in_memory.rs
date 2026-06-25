//! In-memory session store for agent unit tests.
//!
//! Implements [`SessionStore`] with a `HashMap` so the agent loop can be
//! tested without creating a real session directory.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use crate::core::message::AgentMessage;
use crate::core::ports::SessionStore;

/// In-memory session storage backed by a `HashMap<session_id, Vec<AgentMessage>>`.
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<String, SessionState>>,
}

struct SessionState {
    messages: Vec<AgentMessage>,
    entries: Vec<Value>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Seed a session with initial messages (like an existing loaded session).
    pub fn seed_messages(&self, session_id: &str, messages: Vec<AgentMessage>) {
        let mut map = self.sessions.lock().unwrap();
        map.entry(session_id.to_string())
            .or_insert_with(|| SessionState {
                messages: Vec::new(),
                entries: Vec::new(),
            })
            .messages = messages;
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn load_context(&self, session_id: &str) -> Result<Vec<AgentMessage>, String> {
        let map = self.sessions.lock().unwrap();
        map.get(session_id)
            .map(|s| s.messages.clone())
            .ok_or_else(|| format!("session '{session_id}' not found"))
    }

    async fn append_entry(&self, session_id: &str, entry: Value) -> Result<(), String> {
        let mut map = self.sessions.lock().unwrap();
        let state = map
            .get_mut(session_id)
            .ok_or_else(|| format!("session '{session_id}' not found"))?;
        state.entries.push(entry);
        Ok(())
    }

    async fn exists(&self, session_id: &str) -> bool {
        let map = self.sessions.lock().unwrap();
        map.contains_key(session_id)
    }
}
