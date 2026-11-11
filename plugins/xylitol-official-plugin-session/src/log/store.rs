use std::collections::HashSet;

use async_trait::async_trait;
use uuid::Uuid;

use super::append::{append, commit_compaction, create, create_with_fork, list_log_files};
use super::page::load_entries_page;
use super::project::{
    append_edge, build_context_entries, build_session_context, list_session_ids, load_all,
    load_branch, session_display_name_from_entries,
};
use super::{LogSessionStore, StoreMode};
use crate::sync::lock_rwlock_write;
use crate::types::{BranchSummaryEntry, CustomEntry, EntryBase, LabelEntry, SessionEntry};
use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
use xylitol_plugin_sdk::message::now_ms;
use xylitol_plugin_sdk::session::{
    EdgeKind, EdgeRecord, ForkPosition, SessionProfile, count_tool_calls, is_user_message,
    message_text, tool_file_paths,
};
use xylitol_plugin_sdk::{SessionListEntry, XySessionStore};

const UNFLUSHED_FORK_ERR: &str = "This session has not been saved yet. Wait for the first assistant response before cloning or forking it.";

#[async_trait]
impl XySessionStore for LogSessionStore {
    async fn exists(&self, session_id: &str) -> bool {
        match self.mode {
            StoreMode::InMemory => {
                crate::sync::lock_rwlock_read(&self.memory).contains_key(session_id)
            }
            StoreMode::Persisted => !list_log_files(self, session_id)
                .await
                .unwrap_or_default()
                .is_empty(),
        }
    }

    async fn session_profile(
        &self,
        session_id: &str,
    ) -> Result<SessionProfile, XySessionStoreError> {
        let entries = load_all(self, session_id).await?;
        let mut profile = SessionProfile::default();
        for entry in &entries {
            match entry {
                SessionEntry::Header(header) => {
                    profile.name = header.agent_profile.clone();
                    profile.fingerprint = header.agent_profile_fingerprint.clone();
                }
                SessionEntry::Custom(custom) if custom.custom_type == "session_profile" => {
                    profile.name = custom
                        .data
                        .get("name")
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                    profile.fingerprint = custom
                        .data
                        .get("fingerprint")
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                }
                _ => {}
            }
        }
        Ok(profile)
    }

    async fn set_session_profile(
        &self,
        session_id: &str,
        profile: SessionProfile,
    ) -> Result<(), XySessionStoreError> {
        let entry = SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "session_profile".into(),
            data: serde_json::json!({
                "name": profile.name,
                "fingerprint": profile.fingerprint,
            }),
        });
        append(self, session_id, &entry, false).await
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        load_all(self, session_id).await
    }

    async fn load_entries_page(
        &self,
        session_id: &str,
        limit: Option<usize>,
        before_entry_id: Option<&str>,
    ) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
        load_entries_page(self, session_id, limit, before_entry_id).await
    }

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        load_branch(self, session_id, None).await
    }

    async fn build_context_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        build_context_entries(self, session_id).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        append(self, session_id, entry, false).await
    }

    async fn load_done_bash_ids(
        &self,
        session_id: &str,
    ) -> Result<HashSet<String>, XySessionStoreError> {
        let entries = load_all(self, session_id).await?;
        Ok(xylitol_plugin_sdk::session::done_bash_ids(&entries))
    }

    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        commit_compaction(self, session_id, entry).await
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<xylitol_plugin_sdk::session::SessionContext, XySessionStoreError> {
        build_session_context(self, session_id).await
    }

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        create(self, id, cwd, parent).await
    }

    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError> {
        fork_impl(self, parent_id, child_id, at_entry_id, position).await
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        LogSessionStore::set_leaf(self, session_id, entry_id.map(str::to_string));
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        self.get_leaf(session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        let ids = list_session_ids(self).await?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let entries = match load_all(self, &id).await {
                Ok(entries) => entries,
                Err(error) => {
                    log::debug!(target: "xylitol::session", "list_sessions skip {id}: {error}");
                    continue;
                }
            };
            let name = session_display_name_from_entries(&entries).await;
            let mut message_count = 0usize;
            let mut first_message = None;
            let mut parent_session_id = None;
            let mut modified_unix = None;
            let mut cwd = None;
            for entry in &entries {
                if let SessionEntry::Header(header) = entry {
                    parent_session_id = header.parent_session.clone();
                    if !header.cwd.is_empty() {
                        cwd = Some(header.cwd.clone());
                    }
                    modified_unix = Some(header.timestamp / 1000);
                    continue;
                }
                if matches!(entry, SessionEntry::Message(_)) {
                    message_count += 1;
                }
                if first_message.is_none()
                    && is_user_message(entry)
                    && let SessionEntry::Message(message) = entry
                {
                    let text = message_text(&message.message);
                    let cleaned = text
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !cleaned.is_empty() {
                        first_message = Some(cleaned);
                    }
                }
            }
            if modified_unix.is_none() {
                let path = self.layout.session_dir(&id);
                if let Ok(meta) = tokio::fs::metadata(&path).await
                    && let Ok(modified) = meta.modified()
                    && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
                {
                    modified_unix = Some(duration.as_secs());
                }
            }
            let path = if self.layout.log_dir(&id).exists() {
                Some(self.layout.session_dir(&id).to_string_lossy().into_owned())
            } else {
                None
            };
            out.push(SessionListEntry {
                id,
                name,
                first_message,
                message_count,
                modified_unix,
                parent_session_id,
                tree_prefix: String::new(),
                cwd,
                path,
            });
        }
        out.sort_by(|a, b| {
            b.modified_unix
                .unwrap_or(0)
                .cmp(&a.modified_unix.unwrap_or(0))
        });
        Ok(out)
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        match self.mode {
            StoreMode::InMemory => {
                lock_rwlock_write(&self.memory).remove(session_id);
            }
            StoreMode::Persisted => {
                let dir = self.layout.session_dir(session_id);
                if dir.exists() {
                    tokio::fs::remove_dir_all(&dir)
                        .await
                        .map_err(|e| XySessionStoreError::io("delete session dir", e))?;
                }
            }
        }
        self.release_write_lock(session_id);
        LogSessionStore::set_leaf(self, session_id, None);
        Ok(())
    }

    fn try_acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        self.acquire_write_lock(session_id)
    }

    fn release_write_lock(&self, session_id: &str) {
        LogSessionStore::release_write_lock(self, session_id);
    }
}

async fn fork_impl(
    store: &LogSessionStore,
    parent_id: &str,
    child_id: &str,
    at_entry_id: &str,
    position: ForkPosition,
) -> Result<(), XySessionError> {
    if matches!(store.mode, StoreMode::Persisted) && !store.layout.log_dir(parent_id).exists() {
        return Err(XySessionStoreError::validation(UNFLUSHED_FORK_ERR).into());
    }
    let parent_entries = load_all(store, parent_id).await?;
    let selected = parent_entries
        .iter()
        .find(|entry| entry.entry_id() == Some(at_entry_id))
        .ok_or_else(|| XySessionError::entry_not_found(at_entry_id))?;
    let path_leaf: Option<&str> = match position {
        ForkPosition::At => Some(at_entry_id),
        ForkPosition::Before => {
            if !is_user_message(selected) {
                return Err(XySessionStoreError::validation(
                    "ForkPosition::Before requires a user message entry (pi /fork)",
                )
                .into());
            }
            selected.parent_id()
        }
    };
    let path = match path_leaf {
        None => Vec::new(),
        Some(leaf) => load_branch(store, parent_id, Some(leaf)).await?,
    };
    let from_id = path_leaf.unwrap_or(at_entry_id).to_string();
    let path_ids: HashSet<&str> = path.iter().filter_map(|entry| entry.entry_id()).collect();
    let skipped: Vec<SessionEntry> = parent_entries
        .iter()
        .filter(|entry| matches!(entry, SessionEntry::Message(_)))
        .filter(|entry| entry.entry_id().is_some_and(|id| !path_ids.contains(id)))
        .cloned()
        .collect();
    let summary = generate_branch_summary(&skipped);
    let summary = if summary.is_empty() {
        format!("{child_id}: no skipped entries")
    } else {
        format!("{child_id}: {summary}")
    };

    let mut labels_by_target: std::collections::HashMap<String, (Option<String>, u64)> =
        std::collections::HashMap::new();
    for entry in &parent_entries {
        if let SessionEntry::Label(label) = entry
            && path_ids.contains(label.target_id.as_str())
        {
            if label.label.is_none() {
                labels_by_target.remove(&label.target_id);
            } else {
                labels_by_target.insert(
                    label.target_id.clone(),
                    (label.label.clone(), label.base.timestamp),
                );
            }
        }
    }
    let mut label_targets: Vec<_> = labels_by_target.into_iter().collect();
    label_targets.sort_by(|a, b| a.1.1.cmp(&b.1.1).then_with(|| a.0.cmp(&b.0)));

    let parent_cwd = parent_entries.iter().find_map(|entry| match entry {
        SessionEntry::Header(header) => Some(header.cwd.as_str()),
        _ => None,
    });

    create_with_fork(store, child_id, parent_cwd, Some(parent_id), at_entry_id).await?;

    let summary_entry = SessionEntry::BranchSummary(BranchSummaryEntry {
        base: EntryBase {
            entry_type: "branchSummary".into(),
            id: String::new(),
            parent_id: Some(from_id.clone()),
            timestamp: 0,
        },
        from_id,
        summary,
        details: None,
        from_hook: None,
    });
    append(store, child_id, &summary_entry, true).await?;
    let mut label_parent = store.get_leaf(child_id);
    for (target_id, (label, timestamp)) in label_targets {
        let id = Uuid::new_v4().to_string();
        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: id.clone(),
                parent_id: label_parent.clone(),
                timestamp,
            },
            target_id,
            label,
        });
        append(store, child_id, &entry, true).await?;
        label_parent = Some(id);
    }
    if let Some(leaf) = store.get_leaf(child_id) {
        append_edge(
            store,
            child_id,
            &EdgeRecord {
                id: format!("fork:{child_id}"),
                edge_kind: EdgeKind::Fork,
                from: at_entry_id.to_string(),
                to: leaf,
                order: 0,
                enabled: true,
                timestamp: now_ms(),
            },
        )
        .await?;
    }
    Ok(())
}

pub(crate) fn generate_branch_summary(skipped_entries: &[SessionEntry]) -> String {
    if skipped_entries.is_empty() {
        return String::new();
    }
    let total = skipped_entries.len();
    let user_count = skipped_entries
        .iter()
        .filter(|entry| is_user_message(entry))
        .count();
    let assistant_count = skipped_entries
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                SessionEntry::Message(message)
                    if message.message.get("role").and_then(|role| role.as_str()) == Some("assistant")
            )
        })
        .count();
    let tool_calls: usize = skipped_entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(message) => Some(count_tool_calls(&message.message)),
            _ => None,
        })
        .sum();
    let mut files: Vec<String> = Vec::new();
    for entry in skipped_entries {
        if let SessionEntry::Message(message) = entry {
            for path in tool_file_paths(&message.message) {
                if !files.contains(&path) {
                    files.push(path);
                }
            }
        }
    }
    let last_user_msg = skipped_entries.iter().rev().find_map(|entry| {
        if !is_user_message(entry) {
            return None;
        }
        let SessionEntry::Message(message) = entry else {
            return None;
        };
        let text = message_text(&message.message);
        if text.is_empty() {
            return None;
        }
        let truncated: String = text.chars().take(100).collect();
        Some(if text.chars().count() > 100 {
            format!("{truncated}...")
        } else {
            truncated
        })
    });
    let mut summary = format!("分支摘要:\n- 跳过 {total} 条记录\n");
    summary.push_str(&format!(
        "- 包含: {user_count} 条用户消息, {assistant_count} 条助手消息"
    ));
    if tool_calls > 0 {
        summary.push_str(&format!(", {tool_calls} 次工具调用"));
    }
    summary.push('\n');
    if let Some(ref message) = last_user_msg {
        summary.push_str(&format!("- 最后一条用户消息: \"{message}\"\n"));
    }
    if !files.is_empty() {
        files.sort();
        files.dedup();
        summary.push_str(&format!("- 涉及文件: {}\n", files.join(", ")));
    }
    summary
}
