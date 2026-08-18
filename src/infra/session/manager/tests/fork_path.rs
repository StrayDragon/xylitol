//! c645: path-based fork (pi createBranchedSession) — not file-order slice.
use super::super::*;
use crate::protocol::error::XySessionError;
use crate::protocol::session::{
    EntryBase, ForkPosition, MessageEntry, fixture_message_json, is_user_message, message_text,
};

/// Deterministic unix-ms from a string id (test fixture; v6 ms baseline).
fn fork_entry_ms(id: &str) -> u64 {
    1_781_827_200_000u64
        + id.as_bytes()
            .iter()
            .fold(0u64, |acc, b| acc * 31 + *b as u64)
            % 1_000_000
}

fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: fork_entry_ms(id),
        },
        message: fixture_message_json(role, text),
    })
}

/// Branched tree written so **file order** would include the sibling if
/// sliced by index, while **get_branch** path would not:
/// ```text
/// u1
/// ├─ a_left  (file earlier)
/// └─ a_right (file later; fork target path)
/// ```
async fn seeded_sibling_tree(mgr: &SessionManager, sid: &str) {
    mgr.create(sid, Some("."), None).await.unwrap();
    if matches!(&mgr.backend, SessionBackend::Persisted { .. }) {
        mgr.flush_pending_to_disk(sid).await.unwrap();
    }
    for e in [
        msg("u1", None, "user", "root user"),
        msg(
            "a_left",
            Some("u1"),
            "assistant",
            "LEFT sibling — must not leak",
        ),
        msg("a_right", Some("u1"), "assistant", "RIGHT keep"),
        msg("u_right", Some("a_right"), "user", "fork me"),
    ] {
        match &mgr.backend {
            SessionBackend::Persisted { .. } => {
                mgr.append_with_id(sid, &e).await.unwrap();
            }
            SessionBackend::InMemory { .. } => {
                let mut store = mgr.in_memory_store.write().expect("lock");
                store.entry(sid.to_string()).or_default().push(e.clone());
                if let Some(id) = e.entry_id() {
                    mgr.set_leaf(sid, Some(id.to_string()));
                }
            }
        }
    }
}

fn unique_pair() -> (String, String) {
    let n = uuid::Uuid::new_v4();
    (format!("parent-{n}"), format!("child-{n}"))
}

#[tokio::test]
async fn fork_at_excludes_sibling_branch() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    seeded_sibling_tree(&mgr, &parent_id).await;

    mgr.fork(&parent_id, &child_id, "u_right", ForkPosition::At)
        .await
        .unwrap();

    let child = mgr.load(&child_id).await.unwrap();
    let ids: Vec<_> = child.iter().filter_map(|e| e.entry_id()).collect();
    assert!(
        ids.contains(&"u1") && ids.contains(&"a_right") && ids.contains(&"u_right"),
        "path must include u1→a_right→u_right: {ids:?}"
    );
    assert!(
        !ids.contains(&"a_left"),
        "sibling a_left must NOT leak into child (file-order bug): {ids:?}"
    );

    let parent = mgr.load(&parent_id).await.unwrap();
    assert_eq!(
        parent.iter().filter(|e| e.entry_id().is_some()).count(),
        4,
        "parent must be unchanged"
    );
    let header = child.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h),
        _ => None,
    });
    assert_eq!(
        header.and_then(|h| h.parent_session.as_deref()),
        Some(parent_id.as_str())
    );
}

#[tokio::test]
async fn fork_before_user_omits_user_and_prefills_source() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    seeded_sibling_tree(&mgr, &parent_id).await;

    mgr.fork(&parent_id, &child_id, "u_right", ForkPosition::Before)
        .await
        .unwrap();

    let child = mgr.load(&child_id).await.unwrap();
    let ids: Vec<_> = child.iter().filter_map(|e| e.entry_id()).collect();
    assert!(
        ids.contains(&"u1") && ids.contains(&"a_right"),
        "before-user path ends at parent of u_right: {ids:?}"
    );
    assert!(
        !ids.contains(&"u_right"),
        "selected user must not be copied (pi before): {ids:?}"
    );
    assert!(!ids.contains(&"a_left"), "no sibling leak: {ids:?}");

    let parent = mgr.load(&parent_id).await.unwrap();
    let u = parent
        .iter()
        .find(|e| e.entry_id() == Some("u_right"))
        .expect("u_right");
    assert!(is_user_message(u));
    let text = match u {
        SessionEntry::Message(m) => message_text(&m.message),
        _ => String::new(),
    };
    assert_eq!(text, "fork me");
}

#[tokio::test]
async fn fork_before_rejects_non_user() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    seeded_sibling_tree(&mgr, &parent_id).await;
    let err = mgr
        .fork(&parent_id, &child_id, "a_right", ForkPosition::Before)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("user"),
        "Before on assistant must err: {err}"
    );
}

#[tokio::test]
async fn fork_missing_entry_is_session_not_store() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    seeded_sibling_tree(&mgr, &parent_id).await;
    let err = mgr
        .fork(&parent_id, &child_id, "no-such-entry", ForkPosition::At)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            XySessionError::EntryNotFound { ref entry_id } if entry_id == "no-such-entry"
        ),
        "fork miss must be session EntryNotFound, got {err:?}"
    );
}

#[tokio::test]
async fn persisted_fork_does_not_mutate_parent_file() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    seeded_sibling_tree(&mgr, &parent_id).await;
    let before = tokio::fs::read(mgr.session_path(&parent_id)).await.unwrap();

    mgr.fork(&parent_id, &child_id, "u_right", ForkPosition::At)
        .await
        .unwrap();

    let after = tokio::fs::read(mgr.session_path(&parent_id)).await.unwrap();
    assert_eq!(before, after, "parent JSONL bytes must be identical");
    assert!(mgr.session_path(&child_id).exists());
}

#[tokio::test]
async fn unflushed_parent_rejects_fork() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let parent = format!("parent-{}", uuid::Uuid::new_v4());
    let child = format!("child-{}", uuid::Uuid::new_v4());
    mgr.create(&parent, Some("."), None).await.unwrap();
    // User only in pending — no JSONL yet.
    mgr.append(&parent, &msg("u1", None, "user", "not flushed"))
        .await
        .unwrap();
    assert!(!mgr.session_path(&parent).exists());

    let err = mgr
        .fork(&parent, &child, "u1", ForkPosition::At)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not been saved yet"),
        "pi unflushed guard: {err}"
    );
    assert!(!mgr.session_path(&child).exists());
}

#[tokio::test]
async fn fork_user_only_path_defers_child_file() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let (parent_id, child_id) = unique_pair();
    mgr.create(&parent_id, Some("."), None).await.unwrap();
    mgr.flush_pending_to_disk(&parent_id).await.unwrap();
    mgr.append_with_id(&parent_id, &msg("u1", None, "user", "first"))
        .await
        .unwrap();
    mgr.append_with_id(&parent_id, &msg("a1", Some("u1"), "assistant", "answer"))
        .await
        .unwrap();

    // Fork At first user → path has no assistant → child stays pending.
    mgr.fork(&parent_id, &child_id, "u1", ForkPosition::At)
        .await
        .unwrap();
    assert!(
        !mgr.session_path(&child_id).exists(),
        "pi: no assistant on path → no child file yet"
    );
    let loaded = mgr.load(&child_id).await.unwrap();
    let ids: Vec<_> = loaded.iter().filter_map(|e| e.entry_id()).collect();
    assert_eq!(ids, vec!["u1"], "child pending body: {ids:?}");
    assert!(
        loaded
            .iter()
            .any(|e| matches!(e, SessionEntry::Header(h) if h.parent_session.as_deref() == Some(parent_id.as_str()))),
        "header parent_session: {loaded:?}"
    );

    // First assistant on child flushes (deferred persist).
    mgr.append(
        &child_id,
        &msg("a_new", Some("u1"), "assistant", "branched"),
    )
    .await
    .unwrap();
    assert!(mgr.session_path(&child_id).exists());
    let flushed = mgr.load(&child_id).await.unwrap();
    assert!(
        flushed.iter().any(|e| e.entry_id() == Some("u1")),
        "flushed keeps forked user: {flushed:?}"
    );
    assert!(
        flushed.iter().any(
            |e| matches!(e, SessionEntry::Message(m) if message_role_of(m) == Some("assistant"))
        ),
        "assistant present after flush: {flushed:?}"
    );
}

fn message_role_of(m: &MessageEntry) -> Option<&str> {
    m.message.get("role").and_then(|r| r.as_str())
}

#[tokio::test]
async fn fork_strips_path_labels_and_rebuilds_at_end() {
    let mgr = SessionManager::in_memory();
    let parent = "parent-labels";
    let child = "child-labels";
    mgr.create(parent, Some("."), None).await.unwrap();
    for e in [
        msg("u1", None, "user", "hello"),
        msg("a1", Some("u1"), "assistant", "hi"),
    ] {
        let mut store = mgr.in_memory_store.write().expect("lock");
        store.entry(parent.to_string()).or_default().push(e.clone());
        if let Some(id) = e.entry_id() {
            mgr.set_leaf(parent, Some(id.to_string()));
        }
    }
    // Label targets u1; fork at a1 rebuilds label at end (pi createBranchedSession).
    {
        let mut store = mgr.in_memory_store.write().expect("lock");
        store
            .entry(parent.to_string())
            .or_default()
            .push(SessionEntry::Label(crate::protocol::session::LabelEntry {
                base: EntryBase {
                    entry_type: "label".into(),
                    id: "lbl1".into(),
                    parent_id: Some("a1".into()),
                    timestamp: 0,
                },
                target_id: "u1".into(),
                label: Some("checkpoint".into()),
            }));
    }

    mgr.fork(parent, child, "a1", ForkPosition::At)
        .await
        .unwrap();

    let child_entries = mgr.load(child).await.unwrap();
    let ids: Vec<_> = child_entries.iter().filter_map(|e| e.entry_id()).collect();
    assert!(ids.contains(&"u1") && ids.contains(&"a1"), "{ids:?}");
    assert!(
        !ids.contains(&"lbl1"),
        "original label id must not be copied: {ids:?}"
    );
    let labels: Vec<_> = child_entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Label(l) => Some((l.target_id.as_str(), l.label.as_deref())),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec![("u1", Some("checkpoint"))], "{labels:?}");
    // Rebuilt label is after path; a1's parent must be u1 (not the old label).
    let a1 = child_entries
        .iter()
        .find(|e| e.entry_id() == Some("a1"))
        .unwrap();
    assert_eq!(a1.parent_id(), Some("u1"));
}

#[tokio::test]
async fn create_writes_session_version_five() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = format!("ver-{}", uuid::Uuid::new_v4());
    mgr.create(&sid, Some("."), None).await.unwrap();
    mgr.flush_pending_to_disk(&sid).await.unwrap();
    let entries = mgr.load(&sid).await.unwrap();
    let version = entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.version),
        _ => None,
    });
    assert_eq!(version, Some(SESSION_VERSION));
    let raw = tokio::fs::read_to_string(mgr.session_path(&sid))
        .await
        .unwrap();
    assert!(
        raw.contains("\"version\":6") || raw.contains("\"version\": 6"),
        "{raw}"
    );
    assert!(!raw.contains("\"type\":\"bashExecution\""));
}

#[tokio::test]
async fn list_sessions_skips_unreadable_file() {
    use crate::protocol::ports::XySessionStore;
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let good = format!("good-{}", uuid::Uuid::new_v4());
    mgr.create(&good, Some("."), None).await.unwrap();
    mgr.flush_pending_to_disk(&good).await.unwrap();

    let bad_path = sessions.join("bad-legacy.jsonl");
    tokio::fs::write(
        &bad_path,
        r#"{"type":"session","version":4,"id":"bad-legacy","timestamp":"t","cwd":"."}
"#,
    )
    .await
    .unwrap();

    let listed = mgr.list_sessions().await.unwrap();
    let ids: Vec<_> = listed.iter().map(|e| e.id.as_str()).collect();
    assert!(ids.contains(&good.as_str()), "{ids:?}");
    assert!(!ids.contains(&"bad-legacy"), "{ids:?}");
}

#[tokio::test]
async fn list_sessions_skips_many_legacy_with_one_summary() {
    use crate::protocol::ports::XySessionStore;
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let good = format!("good-{}", uuid::Uuid::new_v4());
    mgr.create(&good, Some("."), None).await.unwrap();
    mgr.flush_pending_to_disk(&good).await.unwrap();

    for i in 0..20 {
        let bad_path = sessions.join(format!("legacy-{i}.jsonl"));
        tokio::fs::write(
            &bad_path,
            format!(
                r#"{{"type":"session","version":4,"id":"legacy-{i}","timestamp":"t","cwd":"."}}
"#
            ),
        )
        .await
        .unwrap();
    }

    let listed = mgr.list_sessions().await.unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0].id, good);
}
