//! In-memory [`XySessionStore`] for designing tests and ignored labs.
//! Product TUI does not persist; this is crate-local, not a second session plugin.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use xylitol_plugin_sdk::SessionListEntry;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
use xylitol_plugin_sdk::session::{ForkPosition, SessionContext, SessionEntry};

/// Process-local session map with cwd metadata for `list_sessions`.
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<String, Vec<SessionEntry>>>,
    cwd: Mutex<HashMap<String, Option<String>>>,
    leaves: Mutex<HashMap<String, Option<String>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            cwd: Mutex::new(HashMap::new()),
            leaves: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl XySessionStore for InMemorySessionStore {
    async fn exists(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .map(|m| m.contains_key(session_id))
            .unwrap_or(false)
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
        let new_id = uuid::Uuid::new_v4().to_string();
        let parent_id = self.leaf_id(session_id);
        let stamped = stamp_entry_ids(entry, &new_id, parent_id.as_deref());
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?;
        let list = map
            .get_mut(session_id)
            .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
        list.push(stamped);
        drop(map);
        self.set_leaf(session_id, Some(&new_id));
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
        cwd: Option<&str>,
        _parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        self.sessions
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
            .insert(id.to_string(), Vec::new());
        self.cwd
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
            .insert(id.to_string(), cwd.map(str::to_string));
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

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?;
        let cwd = self
            .cwd
            .lock()
            .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?;
        Ok(sessions
            .keys()
            .map(|id| SessionListEntry {
                id: id.clone(),
                name: None,
                first_message: None,
                message_count: sessions.get(id).map(Vec::len).unwrap_or(0),
                modified_unix: Some(1),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: cwd.get(id).cloned().flatten(),
                path: None,
            })
            .collect())
    }
}

fn stamp_entry_ids(entry: &SessionEntry, id: &str, parent_id: Option<&str>) -> SessionEntry {
    use xylitol_plugin_sdk::session::{EntryBase, LabelEntry, MessageEntry};
    match entry {
        SessionEntry::Message(m) => SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: m.base.entry_type.clone(),
                id: id.to_string(),
                parent_id: parent_id.map(str::to_string),
                timestamp: m.base.timestamp,
            },
            message: m.message.clone(),
        }),
        SessionEntry::Label(l) => SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: l.base.entry_type.clone(),
                id: id.to_string(),
                parent_id: parent_id.map(str::to_string),
                timestamp: l.base.timestamp,
            },
            target_id: l.target_id.clone(),
            label: l.label.clone(),
        }),
        other => other.clone(),
    }
}
