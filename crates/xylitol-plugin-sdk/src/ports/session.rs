use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{XySessionError, XySessionStoreError};
use crate::session::{
    EntryBase, SessionContext, SessionEntry, SessionInfoEntry, SessionTreeNode, build_session_tree,
};

pub use crate::session::ForkPosition;

// ~keep: plugins must write through the runtime's live store, not the
// session-plugin service instance.
#[derive(Clone)]
pub struct RuntimeSessionStore(pub Arc<dyn XySessionStore>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionListEntry {
    pub id: String,
    pub name: Option<String>,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub modified_unix: Option<u64>,
    pub parent_session_id: Option<String>,
    pub tree_prefix: String,
    pub cwd: Option<String>,
    pub path: Option<String>,
}

#[async_trait]
pub trait XySessionStore: Send + Sync {
    async fn exists(&self, session_id: &str) -> bool;

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError>;

    async fn load_entries_page(
        &self,
        session_id: &str,
        limit: Option<usize>,
        before_entry_id: Option<&str>,
    ) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(crate::session::paginate_entries(
            entries,
            limit,
            before_entry_id,
        ))
    }

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.load_entries(session_id).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError>;

    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.append_session_entry(session_id, entry).await
    }

    async fn load_done_bash_ids(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashSet<String>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(crate::session::done_bash_ids(&entries))
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError>;

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError>;
    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError>;

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>);

    fn leaf_id(&self, session_id: &str) -> Option<String>;

    async fn message_history_tree(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionTreeNode>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(build_session_tree(&entries))
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        let _ = self;
        Ok(Vec::new())
    }

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

    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        let _ = session_id;
        let _ = self;
        Err(XySessionStoreError::unsupported("delete_session"))
    }

    fn try_acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        let _ = session_id;
        Ok(())
    }

    fn release_write_lock(&self, session_id: &str) {
        let _ = session_id;
    }

    async fn read_recent(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let all = self.load_entries(session_id).await?;
        let skip = all.len().saturating_sub(limit);
        Ok(all.into_iter().skip(skip).collect())
    }

    async fn migrate_legacy_custom_types(
        &self,
        session_id: &str,
    ) -> Result<usize, XySessionStoreError> {
        let branch = self.load_leaf_branch(session_id).await?;
        let mut present: HashSet<(String, String)> = HashSet::new();
        for entry in &branch {
            let SessionEntry::Custom(custom) = entry else {
                continue;
            };
            if crate::session::legacy_custom_type_target(&custom.custom_type).is_some() {
                continue;
            }
            present.insert((
                custom.custom_type.clone(),
                serde_json::to_string(&custom.data)?,
            ));
        }

        let mut migrated = 0usize;
        for entry in &branch {
            let SessionEntry::Custom(custom) = entry else {
                continue;
            };
            let Some(target) = crate::session::legacy_custom_type_target(&custom.custom_type)
            else {
                continue;
            };
            let key = (target.to_string(), serde_json::to_string(&custom.data)?);
            if !present.insert(key) {
                continue;
            }
            let mut shell = custom.clone();
            shell.custom_type = target.to_string();
            shell.base = EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            };
            self.append_session_entry(session_id, &SessionEntry::Custom(shell))
                .await?;
            migrated += 1;
        }
        Ok(migrated)
    }
}

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

fn is_invisible_char(c: char) -> bool {
    matches!(c, '\u{200B}' | '\u{200E}' | '\u{200F}' | '\u{FEFF}')
        || ('\u{202A}'..='\u{202E}').contains(&c)
        || ('\u{2060}'..='\u{2064}').contains(&c)
        || ('\u{2066}'..='\u{206F}').contains(&c)
}

fn strip_terminal_sequences(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek().copied() {
                Some(']') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if n == '\x07' {
                            break;
                        }
                        if n == '\x1b' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some('[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&n) {
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
            continue;
        }
        if c.is_control() || is_invisible_char(c) {
            continue;
        }
        out.push(c);
    }
    out
}

pub fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = 0usize;
    for (idx, _) in text.char_indices() {
        if idx > max_bytes {
            break;
        }
        end = idx;
    }
    let candidate = &text[..end];
    if candidate.len() <= max_bytes {
        candidate.to_string()
    } else {
        let mut cut = max_bytes;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text[..cut].to_string()
    }
}

pub fn normalize_stage_name(raw: &str, max_bytes: usize) -> String {
    let cleaned = strip_terminal_sequences(raw);
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_utf8(&collapsed, max_bytes)
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        ('\u{3400}'..='\u{4dbf}').contains(&c)
            || ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{f900}'..='\u{faff}').contains(&c)
    })
}

pub fn fallback_stage_name(
    first_user: &str,
    target_words: usize,
    target_cjk_chars: usize,
    max_bytes: usize,
) -> String {
    let cleaned = normalize_stage_name(first_user, usize::MAX);
    let candidate = if contains_cjk(&cleaned) && !cleaned.contains(char::is_whitespace) {
        cleaned
            .chars()
            .take(target_cjk_chars.max(1))
            .collect::<String>()
    } else {
        let words: Vec<&str> = cleaned
            .split_whitespace()
            .take(target_words.max(1))
            .collect();
        words.join(" ")
    };
    let candidate = truncate_utf8(&candidate, max_bytes);
    if candidate.is_empty() {
        truncate_utf8("未命名会话", max_bytes)
    } else {
        candidate
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::session::{CustomEntry, MessageEntry};

    struct MemStore {
        entries: Mutex<Vec<SessionEntry>>,
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _id: &str) -> bool {
            false
        }
        async fn load_entries(&self, _id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().unwrap().clone())
        }
        async fn append_session_entry(
            &self,
            _id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries.lock().unwrap().push(entry.clone());
            Ok(())
        }
        async fn build_session_context(
            &self,
            _id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Err(XySessionStoreError::unsupported("build_session_context"))
        }
        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            _at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::EntryNotFound {
                entry_id: _at_entry_id.into(),
            })
        }
        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}
        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn text_entry(id: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            message: serde_json::to_value(crate::message::AgentMessage::assistant(
                text.to_string(),
            ))
            .unwrap(),
        })
    }

    fn custom_entry(id: &str, custom_type: &str, data: serde_json::Value) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            custom_type: custom_type.into(),
            data,
        })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn legacy_custom_types_migrate_once_and_append_only() {
        let workflow = serde_json::json!({"goal": "ship", "steps": []});
        let checkpoint = serde_json::json!({"goal": "ship", "steps": []});
        let store = MemStore {
            entries: Mutex::new(vec![
                custom_entry("legacy-workflow", "plan", workflow.clone()),
                custom_entry("legacy-checkpoint", "plan_checkpoint", checkpoint.clone()),
                text_entry("m0", "hello"),
            ]),
        };

        assert_eq!(store.migrate_legacy_custom_types("s").await.unwrap(), 2);
        let entries = store.load_entries("s").await.unwrap();
        assert_eq!(entries.len(), 5, "migration must append, never rewrite");
        assert!(entries.iter().any(|entry| matches!(
            entry,
            SessionEntry::Custom(custom)
                if custom.custom_type == "workflow" && custom.data == workflow
        )));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            SessionEntry::Custom(custom)
                if custom.custom_type == "todo_checkpoint" && custom.data == checkpoint
        )));
        assert!(entries.iter().any(|entry| matches!(
            entry,
            SessionEntry::Custom(custom) if custom.custom_type == "plan"
        )));

        assert_eq!(
            store.migrate_legacy_custom_types("s").await.unwrap(),
            0,
            "a second load must not append again"
        );
        assert_eq!(store.load_entries("s").await.unwrap().len(), 5);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn migrated_new_type_prevents_duplicate_migration() {
        let data = serde_json::json!({"goal": "ship", "steps": []});
        let store = MemStore {
            entries: Mutex::new(vec![
                custom_entry("new", "workflow", data.clone()),
                custom_entry("legacy", "plan", data),
            ]),
        };

        assert_eq!(store.migrate_legacy_custom_types("s").await.unwrap(), 0);
        assert_eq!(store.load_entries("s").await.unwrap().len(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_recent_returns_last_n_in_append_order() {
        let store = MemStore {
            entries: Mutex::new(Vec::new()),
        };
        for i in 0..5 {
            store
                .append_session_entry("s", &text_entry(&format!("m{i}"), &format!("m{i}")))
                .await
                .unwrap();
        }
        let recent = store.read_recent("s", 2).await.unwrap();
        assert_eq!(recent.len(), 2, "read_recent MUST 只返回最近 limit 条");
        let last = recent.last().unwrap();
        let text = last.as_agent_message().map(|m| m.text());
        assert_eq!(text.as_deref(), Some("m4"), "返回条目 MUST 为最近");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_recent_over_limit_returns_all() {
        let store = MemStore {
            entries: Mutex::new(Vec::new()),
        };
        for i in 0..3 {
            store
                .append_session_entry("s", &text_entry(&format!("m{i}"), &format!("m{i}")))
                .await
                .unwrap();
        }
        let all = store.read_recent("s", 99).await.unwrap();
        assert_eq!(all.len(), 3, "limit 超总量 MUST 返回全部");
    }
}

#[cfg(test)]
mod stage_name_tests {
    use super::{fallback_stage_name, normalize_stage_name, truncate_utf8};

    #[test]
    fn strips_terminal_sequences_and_controls() {
        let raw = "\u{1b}]8;;https://x\u{1b}\\\u{1b}[31m新功能A\u{1b}[0m\u{0}";
        assert_eq!(normalize_stage_name(raw, 120), "新功能A");
    }

    #[test]
    fn collapses_whitespace_and_removes_directional_controls() {
        let raw = "  新功能\u{200b}A \n\t 标题  ";
        assert_eq!(normalize_stage_name(raw, 120), "新功能A 标题");
    }

    #[test]
    fn truncates_utf8_safely() {
        let raw = "新功能A";
        let truncated = truncate_utf8(raw, 4);
        assert_eq!(truncated, "新");
        assert!(raw.starts_with(&truncated));
    }

    #[test]
    fn fallback_uses_leading_words_and_bounds_bytes() {
        assert_eq!(
            fallback_stage_name("帮我修复登录失败的问题并补充测试", 3, 6, 120),
            "帮我修复登录"
        );
        assert_eq!(fallback_stage_name("   ", 20, 15, 120), "未命名会话");
    }

    #[test]
    fn normalize_empty_returns_empty() {
        assert_eq!(normalize_stage_name("\u{1b}[31m\u{1b}[0m", 120), "");
    }
}
