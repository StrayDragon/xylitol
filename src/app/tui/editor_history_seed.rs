//! Editor ↑/↓ send-history seeds from session store (c1560 / ati41).

use std::sync::Arc;

use crate::app::core::driver::{XyDriver, XyDriverError};
use crate::app::tui::session_resume::cwd_matches;
use crate::protocol::error::XySessionStoreError;
use crate::protocol::ports::XySessionStore;
use crate::protocol::session::{SessionEntry, message_role, message_text};

/// Extract user prompt texts from session entries (chronological order; skip `/…`).
pub fn user_prompt_texts_from_entries(entries: &[SessionEntry]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        let SessionEntry::Message(m) = entry else {
            continue;
        };
        if message_role(&m.message) != Some("user") {
            continue;
        }
        let text = message_text(&m.message);
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('/') {
            continue;
        }
        out.push(text);
    }
    out
}

/// Prior same-cwd sessions via [`XySessionStore`] (spawn-safe; no `&dyn XyDriver`).
pub async fn collect_new_session_seed_from_store(
    store: &dyn XySessionStore,
    current_cwd: &str,
    current_id: Option<&str>,
    n: u32,
) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    let listed = store.list_sessions().await.unwrap_or_default();
    let mut matched: Vec<_> = listed
        .into_iter()
        .filter(|e| cwd_matches(e.cwd.as_deref(), current_cwd))
        .filter(|e| current_id.is_none_or(|id| e.id != id))
        .collect();
    matched.sort_by(|a, b| {
        b.modified_unix
            .unwrap_or(0)
            .cmp(&a.modified_unix.unwrap_or(0))
    });
    matched.truncate(n as usize);
    // Process older sessions first so ↑ lands on globally newest prompt.
    matched.reverse();
    let mut texts = Vec::new();
    for entry in matched {
        match store.load_entries(&entry.id).await {
            Ok(entries) => texts.extend(user_prompt_texts_from_entries(&entries)),
            Err(e) => {
                log::debug!(
                    target: "xylitol::tui",
                    "editor history seed skip session {}: {e}",
                    entry.id
                );
            }
        }
    }
    texts
}

/// Prior same-cwd sessions (mtime desc, exclude `current_id`), take `n`, oldest→newest texts.
pub async fn collect_new_session_seed(
    driver: &dyn XyDriver,
    current_cwd: &str,
    current_id: Option<&str>,
    n: u32,
) -> Result<Vec<String>, XyDriverError> {
    if let Some(store) = driver.session_store() {
        return Ok(
            collect_new_session_seed_from_store(store.as_ref(), current_cwd, current_id, n).await,
        );
    }
    if n == 0 {
        return Ok(Vec::new());
    }
    let listed = driver.list_sessions().await.unwrap_or_default();
    let mut matched: Vec<_> = listed
        .into_iter()
        .filter(|e| cwd_matches(e.cwd.as_deref(), current_cwd))
        .filter(|e| current_id.is_none_or(|id| e.id != id))
        .collect();
    matched.sort_by(|a, b| {
        b.modified_unix
            .unwrap_or(0)
            .cmp(&a.modified_unix.unwrap_or(0))
    });
    matched.truncate(n as usize);
    matched.reverse();
    let mut texts = Vec::new();
    for entry in matched {
        match driver.load_session_entries(&entry.id).await {
            Ok(entries) => texts.extend(user_prompt_texts_from_entries(&entries)),
            Err(e) => {
                log::debug!(
                    target: "xylitol::tui",
                    "editor history seed skip session {}: {e}",
                    entry.id
                );
            }
        }
    }
    Ok(texts)
}

/// Background ↑/↓ history seed (product in-process driver).
pub fn spawn_new_session_seed(
    store: Arc<dyn XySessionStore>,
    current_cwd: String,
    current_id: Option<String>,
    n: u32,
) -> tokio::task::JoinHandle<Vec<String>> {
    tokio::spawn(async move {
        collect_new_session_seed_from_store(store.as_ref(), &current_cwd, current_id.as_deref(), n)
            .await
    })
}

/// Background CLI `--session` JSONL load (same data as [`XyDriver::get_messages`](crate::app::core::driver::XyDriver::get_messages)).
pub fn spawn_cli_session_load(
    store: Arc<dyn XySessionStore>,
    session_id: String,
) -> tokio::task::JoinHandle<Result<Vec<SessionEntry>, XySessionStoreError>> {
    tokio::spawn(async move { store.load_entries(&session_id).await })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{EntryBase, MessageEntry, fixture_message_json};

    fn msg(role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "e".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: fixture_message_json(role, text),
        })
    }

    #[test]
    fn extracts_user_skips_slash_and_assistant() {
        let entries = vec![
            msg("user", "hello"),
            msg("assistant", "hi"),
            msg("user", "/session-name x"),
            msg("user", "  /model "),
            msg("user", "world"),
        ];
        assert_eq!(
            user_prompt_texts_from_entries(&entries),
            vec!["hello".to_string(), "world".to_string()]
        );
    }

    #[tokio::test]
    async fn collect_seed_from_store_skips_other_cwd() {
        use crate::infra::session::SessionManager;
        use crate::protocol::ports::XySessionStore;
        use std::sync::Arc;

        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(mgr);

        let here = format!("here-{}", uuid::Uuid::new_v4());
        let other = format!("other-{}", uuid::Uuid::new_v4());
        store.create(&here, Some("/proj-a"), None).await.unwrap();
        store
            .append_session_entry(
                &here,
                &SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: "message".into(),
                        id: "u1".into(),
                        parent_id: None,
                        timestamp: 0,
                    },
                    message: fixture_message_json("user", "seed me"),
                }),
            )
            .await
            .unwrap();
        store.create(&other, Some("/proj-b"), None).await.unwrap();
        store
            .append_session_entry(
                &other,
                &SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: "message".into(),
                        id: "u2".into(),
                        parent_id: None,
                        timestamp: 0,
                    },
                    message: fixture_message_json("user", "wrong cwd"),
                }),
            )
            .await
            .unwrap();

        let texts = collect_new_session_seed_from_store(store.as_ref(), "/proj-a", None, 3).await;
        assert!(
            texts.iter().any(|t| t == "seed me"),
            "same-cwd prompts: {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t == "wrong cwd"),
            "other cwd must not seed: {texts:?}"
        );
    }

    #[tokio::test]
    async fn spawn_cli_session_load_returns_entries() {
        use crate::infra::session::SessionManager;
        use crate::protocol::ports::XySessionStore;
        use std::sync::Arc;

        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(mgr);
        let sid = format!("cli-{}", uuid::Uuid::new_v4());
        store.create(&sid, Some("."), None).await.unwrap();
        store
            .append_session_entry(
                &sid,
                &SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: "message".into(),
                        id: "u1".into(),
                        parent_id: None,
                        timestamp: 0,
                    },
                    message: fixture_message_json("user", "restored"),
                }),
            )
            .await
            .unwrap();

        let entries = spawn_cli_session_load(store, sid).await.unwrap().unwrap();
        assert!(
            user_prompt_texts_from_entries(&entries)
                .iter()
                .any(|t| t == "restored")
        );
    }
}
