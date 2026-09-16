//! Runtime boundary for session persistence.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::protocol::error::{XySessionError, XySessionStoreError};
use crate::protocol::session::{
    EntryBase, SessionContext, SessionEntry, SessionInfoEntry, SessionTreeNode, build_session_tree,
};

pub use crate::protocol::session::ForkPosition;

/// Row for session resume picker (XyDriver seam; mtime order is store-defined).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionListEntry {
    pub id: String,
    pub name: Option<String>,
    /// First user-message preview (pi `firstMessage`); unnamed sessions use this as label.
    pub first_message: Option<String>,
    pub message_count: usize,
    /// Last activity / file mtime (unix secs); used for relative age in the picker.
    pub modified_unix: Option<u64>,
    /// Parent session id when this session was forked/cloned (`SessionHeader.parent_session`).
    pub parent_session_id: Option<String>,
    /// Tree glyph prefix after forest flatten (`└─ ` / `├─ `…); empty for roots.
    pub tree_prefix: String,
    /// Session cwd from header (scope=Current filter).
    pub cwd: Option<String>,
    /// Persisted jsonl path when known (optional path display).
    pub path: Option<String>,
}

/// Persistence port — abstracts session storage so the agent can be
/// unit-tested without a real filesystem and the server can host
/// sessions without coupling to the file store.
///
/// Methods are the **minimum** the ReAct loop and compaction call.
/// Full session management (fork/navigate/export) stays on the concrete
/// `infra::session::SessionManager` for the composition root.
#[async_trait]
pub trait XySessionStore: Send + Sync {
    /// Check whether a session exists.
    async fn exists(&self, session_id: &str) -> bool;

    /// Load the raw typed session entries (for compaction / export).
    ///
    /// # Errors
    ///
    /// `Err` when the session does not exist (`NotFound`) or the store IO fails.
    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError>;

    /// Load entries on the current leaf→root branch (pi `getBranch`).
    ///
    /// Compaction prepare/cut MUST use this path so sibling branches are excluded.
    /// Default falls back to [`Self::load_entries`] for linear/stub stores.
    ///
    /// # Errors
    ///
    /// Same as [`Self::load_entries`].
    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.load_entries(session_id).await
    }

    /// Append a typed session entry (compaction summary, bash exec, etc.).
    ///
    /// # Errors
    ///
    /// `Err` when the session does not exist (`NotFound`) or the store IO fails.
    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError>;

    /// Commit a compaction entry and its storage-specific seal transaction.
    ///
    /// The default is append-only so lightweight stores and test doubles keep
    /// their existing semantics. Persisted stores may override this to publish
    /// new segments atomically.
    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.append_session_entry(session_id, entry).await
    }

    /// Build the full session context (messages, thinking level, model).
    ///
    /// # Errors
    ///
    /// `Err` when the session does not exist (`NotFound`) or the store IO fails.
    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError>;

    /// Create a new session (writes header, initializes leaf tracking).
    ///
    /// # Errors
    ///
    /// `Err` when the store cannot write the initial header / leaf state.
    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError>;
    /// Fork a session into a new child.
    ///
    /// Tree-positioned (not pure IO): missing `at_entry_id` is
    /// [`XySessionError::EntryNotFound`]; persist failures stay
    /// [`XySessionError::Store`].
    ///
    /// - [`ForkPosition::At`]: child path is `get_branch` through `at_entry_id` (re-chained).
    /// - [`ForkPosition::Before`]: `at_entry_id` must be a user message; path ends at its
    ///   parent (pi `/fork`); the user row is not copied.
    ///
    /// # Errors
    ///
    /// `Err(EntryNotFound)` when `at_entry_id` is not on the branch;
    /// `Err(Store)` when the child session cannot be persisted.
    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError>;

    /// Set the active leaf entry for branching / travel.
    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>);

    /// Current leaf entry id, if any.
    fn leaf_id(&self, session_id: &str) -> Option<String>;

    /// Build the message-history session tree (default: load entries + `build_session_tree`).
    ///
    /// # Errors
    ///
    /// Same as [`Self::load_entries`].
    async fn message_history_tree(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionTreeNode>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(build_session_tree(&entries))
    }

    /// List resumable sessions (mtime descending when persisted).
    ///
    /// Used by the XyDriver for `/session-resume` (not a `protocol::Command`).
    /// Default returns an empty list so minimal store stubs stay usable.
    ///
    /// # Errors
    ///
    /// `Err` on store listing failure. Default returns `Ok(vec![])`.
    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        let _ = self;
        Ok(Vec::new())
    }

    /// Latest display name from `session_info` entries (c1020 `/session-name`).
    ///
    /// # Errors
    ///
    /// `Err` when the session does not exist (`NotFound`) or the store IO fails.
    async fn get_session_name(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        for entry in entries.iter().rev() {
            if let SessionEntry::SessionInfo(si) = entry {
                return Ok(si
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string));
            }
        }
        Ok(None)
    }

    /// Append a sanitized session display name (CR/LF → space, trim; c1020).
    ///
    /// Returns the name actually stored.
    ///
    /// # Errors
    ///
    /// `Err` when the session does not exist (`NotFound`) or the store IO fails.
    async fn set_session_name(
        &self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XySessionStoreError> {
        let sanitized = sanitize_session_display_name(name);
        let entry = SessionEntry::SessionInfo(SessionInfoEntry {
            base: EntryBase {
                entry_type: "session_info".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            name: Some(sanitized.clone()),
        });
        self.append_session_entry(session_id, &entry).await?;
        Ok(sanitized)
    }

    /// Delete a persisted session (XyDriver `/session-resume` panel; c1065).
    ///
    /// Default returns an error so minimal store stubs stay safe.
    ///
    /// # Errors
    ///
    /// `Err` when unsupported (default stub) or the store cannot delete.
    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        let _ = session_id;
        let _ = self;
        Err(XySessionStoreError::unsupported("delete_session"))
    }
}

/// pi-aligned session display name sanitize (CR/LF runs → one space, then trim).
pub fn sanitize_session_display_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut saw_nl = false;
    for c in name.chars() {
        if c == '\r' || c == '\n' {
            if !saw_nl {
                out.push(' ');
                saw_nl = true;
            }
        } else {
            saw_nl = false;
            out.push(c);
        }
    }
    out.trim().to_string()
}
