use super::super::*;
use crate::protocol::session::{EntryBase, MessageEntry, ThinkingLevelChangeEntry};

fn user_message(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message: crate::protocol::session::fixture_message_json("user", text),
    })
}

fn assistant_message(text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message: crate::protocol::session::fixture_message_json("assistant", text),
    })
}

#[tokio::test]
async fn session_context_defaults_thinking_to_off() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    mgr.create("fresh", Some("."), None).await.unwrap();

    let context = mgr.build_session_context("fresh").await.unwrap();
    assert_eq!(context.thinking_level, "off");
}

#[tokio::test]
async fn session_context_restores_thinking_literal_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "sticky-thinking";
    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(
        sid,
        &SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            base: EntryBase {
                entry_type: "thinking_level_change".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            thinking_level: "vendor-retired".into(),
        }),
    )
    .await
    .unwrap();
    mgr.append(sid, &assistant_message("flush")).await.unwrap();
    let before = std::fs::read(mgr.session_path(sid)).unwrap();

    let context = mgr.build_session_context(sid).await.unwrap();
    assert_eq!(context.thinking_level, "vendor-retired");
    assert_eq!(std::fs::read(mgr.session_path(sid)).unwrap(), before);
}

#[tokio::test]
async fn defer_before_assistant_keeps_disk_clean_then_flushes() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "defer-session";

    mgr.create(sid, Some("."), None).await.unwrap();
    assert!(!mgr.session_path(sid).exists());

    mgr.append(sid, &user_message("hello")).await.unwrap();
    assert!(!mgr.session_path(sid).exists());

    let pending = mgr.load(sid).await.unwrap();
    assert!(
        pending
            .iter()
            .any(|e| matches!(e, SessionEntry::Message(_)))
    );

    mgr.append(sid, &assistant_message("hi")).await.unwrap();
    assert!(mgr.session_path(sid).exists());

    let loaded = mgr.load(sid).await.unwrap();
    let roles: Vec<_> = loaded
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
            _ => None,
        })
        .collect();
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"assistant"));
}

#[tokio::test]
async fn append_before_create_auto_inserts_header_and_create_is_idempotent() {
    // Reproduce bind-then-`/model` race: body row lands in pending before
    // `create` / `ensure_session`. Load must still see a session header.
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "bind-before-create";

    mgr.append(sid, &user_message("hi")).await.unwrap();
    let before = mgr.load(sid).await.unwrap();
    assert!(
        before.iter().any(|e| matches!(e, SessionEntry::Header(_))),
        "append must auto-insert header: {before:?}"
    );

    mgr.create(sid, Some("/tmp/cwd"), None).await.unwrap();
    let after = mgr.load(sid).await.unwrap();
    let headers: Vec<_> = after
        .iter()
        .filter(|e| matches!(e, SessionEntry::Header(_)))
        .collect();
    assert_eq!(headers.len(), 1, "create must not duplicate header");
    assert!(
        after.iter().any(|e| matches!(e, SessionEntry::Message(_))),
        "create must not wipe body rows"
    );
}

#[tokio::test]
async fn append_with_id_after_create_persists_header_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "append-with-id-flush";

    mgr.create(sid, Some("."), None).await.unwrap();
    // No explicit flush — append_with_id must promote pending header first.
    mgr.append_with_id(
        sid,
        &SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: crate::protocol::session::fixture_message_json("user", "hello"),
        }),
    )
    .await
    .unwrap();
    mgr.append_with_id(
        sid,
        &SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: 0,
            },
            message: crate::protocol::session::fixture_message_json("assistant", "hi"),
        }),
    )
    .await
    .unwrap();

    assert!(
        !mgr.pending_store.read().expect("lock").contains_key(sid),
        "pending must be cleared after append_with_id"
    );

    let loaded = mgr.load(sid).await.unwrap();
    let ids: Vec<_> = loaded.iter().filter_map(|e| e.entry_id()).collect();
    assert!(
        loaded.iter().any(|e| matches!(e, SessionEntry::Header(_))),
        "header must be on disk: {loaded:?}"
    );
    assert_eq!(ids, vec!["u1", "a1"], "body rows must survive: {ids:?}");
}

#[tokio::test]
async fn flush_pending_merges_when_file_already_has_body() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "merge-pending";

    mgr.create(sid, Some("."), None).await.unwrap();
    // Simulate pre-fix append_with_id: body on disk, header still pending.
    let path = mgr.session_path(sid);
    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .unwrap();
    let body = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "u1".into(),
            parent_id: None,
            timestamp: 0,
        },
        message: crate::protocol::session::fixture_message_json("user", "kept"),
    });
    let line = serde_json::to_string(&body).unwrap();
    tokio::fs::write(&path, format!("{line}\n")).await.unwrap();

    mgr.flush_pending_to_disk(sid).await.unwrap();

    let loaded = mgr.load(sid).await.unwrap();
    assert!(
        loaded.iter().any(|e| matches!(e, SessionEntry::Header(_))),
        "merged flush must keep header"
    );
    assert!(
        loaded.iter().any(|e| e.entry_id() == Some("u1")),
        "merged flush must not clobber body: {loaded:?}"
    );
}

#[tokio::test]
async fn in_memory_append_never_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::in_memory();
    let sid = "mem-session";

    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &user_message("one")).await.unwrap();
    mgr.append(sid, &assistant_message("two")).await.unwrap();

    assert!(std::fs::read_dir(dir.path()).is_err() || dir.path().read_dir().unwrap().count() == 0);

    let loaded = mgr.load(sid).await.unwrap();
    assert_eq!(
        loaded
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(_)))
            .count(),
        2
    );
}

#[tokio::test]
async fn write_entries_to_disk_rewrites_without_tmp_leftover() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "atomic-rewrite";

    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &user_message("one")).await.unwrap();
    mgr.append(sid, &assistant_message("two")).await.unwrap();

    let file = mgr.get_session_file(sid).unwrap();
    let before = tokio::fs::read_to_string(&file)
        .await
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
    let entries = mgr.load(sid).await.unwrap();
    mgr.write_entries_to_disk(sid, &entries).await.unwrap();
    let after = tokio::fs::read_to_string(mgr.get_session_file(sid).unwrap())
        .await
        .unwrap();
    assert_eq!(before, after, "rewrite must be content-preserving");

    assert!(sessions.join(sid).is_dir(), "session directory exists");
    assert!(
        !sessions.join(format!("{sid}.jsonl")).exists(),
        "v6 single-file layout must not remain"
    );
    assert!(
        std::fs::read_dir(sessions.join(sid))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp")),
        "no tmp leftovers"
    );
}

#[tokio::test]
async fn write_entries_to_disk_failure_keeps_original_intact() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "atomic-failure";

    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &user_message("one")).await.unwrap();
    mgr.append(sid, &assistant_message("two")).await.unwrap();

    let file = mgr.get_session_file(sid).unwrap();
    let original = tokio::fs::read_to_string(&file).await.unwrap();

    // Read-only session root makes the generation replacement fail; the
    // manifest and active segment must stay intact.
    let session_root = sessions.join(sid);
    let mut perms = std::fs::metadata(&session_root).unwrap().permissions();
    perms.set_mode(0o555);
    std::fs::set_permissions(&session_root, perms).unwrap();

    let entries = mgr.load(sid).await.unwrap();
    let result = mgr.write_entries_to_disk(sid, &entries).await;

    let mut perms = std::fs::metadata(&session_root).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&session_root, perms).unwrap();

    assert!(result.is_err(), "tmp creation must fail in read-only dir");
    let after = tokio::fs::read_to_string(&file).await.unwrap();
    assert_eq!(original, after, "failed rewrite must not touch original");
}
