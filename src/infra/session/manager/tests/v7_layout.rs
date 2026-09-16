use super::super::*;
use crate::protocol::session::{
    CompactionEntry, EntryBase, MessageEntry, SessionEntry, SessionManifest, SessionSegment,
};

fn message(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            timestamp: 1,
        },
        message: crate::protocol::session::fixture_message_json(role, text),
    })
}

#[tokio::test]
async fn persisted_session_uses_manifest_and_active_segment() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "v7-layout";

    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &message("", None, "user", "hello"))
        .await
        .unwrap();
    mgr.append(sid, &message("", None, "assistant", "world"))
        .await
        .unwrap();

    let session_dir = sessions.join(sid);
    assert!(session_dir.join("manifest.json").is_file());
    assert!(!sessions.join(format!("{sid}.jsonl")).exists());

    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(session_dir.join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.format_version, SESSION_VERSION);
    assert!(manifest.sealed_segments.is_empty());
    assert!(session_dir.join(&manifest.active_segment.path).is_file());
    assert_eq!(mgr.load(sid).await.unwrap().len(), 3);
}

#[tokio::test]
async fn v6_file_migrates_once_and_marks_policy_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    tokio::fs::create_dir_all(&sessions).await.unwrap();
    let sid = "legacy-v6";
    let legacy = sessions.join(format!("{sid}.jsonl"));
    let header = serde_json::json!({
        "type": "session",
        "version": 6,
        "id": sid,
        "timestamp": 1,
        "cwd": "."
    });
    let compaction = serde_json::json!({
        "type": "compaction",
        "id": "compact-1",
        "parentId": null,
        "timestamp": 2,
        "summary": "old",
        "firstKeptEntryId": "message-1",
        "tokensBefore": 10
    });
    tokio::fs::write(&legacy, format!("{header}\n{compaction}\n"))
        .await
        .unwrap();

    let mgr = SessionManager::new(sessions.clone());
    let entries = mgr.load(sid).await.unwrap();
    assert!(!legacy.exists(), "successful migration removes v6 file");
    assert!(sessions.join(sid).join("manifest.json").exists());
    assert!(entries.iter().any(|entry| {
        matches!(entry, SessionEntry::Header(header) if header.version == SESSION_VERSION)
    }));
    let policy = entries.iter().find_map(|entry| match entry {
        SessionEntry::Compaction(entry) => entry.policy.as_ref(),
        _ => None,
    });
    assert_eq!(
        policy.and_then(|policy| policy.status.as_deref()),
        Some("legacy/unknown")
    );
    let manifest_before = tokio::fs::read(sessions.join(sid).join("manifest.json"))
        .await
        .unwrap();
    let _ = mgr.load(sid).await.unwrap();
    assert_eq!(
        manifest_before,
        tokio::fs::read(sessions.join(sid).join("manifest.json"))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn compaction_seals_prefix_and_leaves_tail_active() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "sealed";
    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in [
        message("u1", None, "user", "old user"),
        message("a1", Some("u1"), "assistant", "old answer"),
        message("u2", Some("a1"), "user", "kept user"),
        message("a2", Some("u2"), "assistant", "kept answer"),
    ] {
        mgr.append_with_id(sid, &entry).await.unwrap();
    }

    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "ignored-by-append".into(),
                parent_id: None,
                timestamp: 3,
            },
            summary: "old history".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: Some(crate::protocol::session::CompactionPolicySnapshot::current(
                32_768, 1_024, 20_000, "test",
            )),
        }),
    )
    .await
    .unwrap();

    let session_dir = sessions.join(sid);
    let manifest_path = session_dir.join("manifest.json");
    let manifest: SessionManifest =
        serde_json::from_slice(&tokio::fs::read(&manifest_path).await.unwrap()).unwrap();
    assert_eq!(manifest.sealed_segments.len(), 1);
    let cold_path = session_dir.join(&manifest.sealed_segments[0].path);
    let cold_before = tokio::fs::read(&cold_path).await.unwrap();
    let active = tokio::fs::read_to_string(session_dir.join(&manifest.active_segment.path))
        .await
        .unwrap();
    assert!(!active.contains("old user"));
    assert!(active.contains("kept user"));
    assert!(active.contains("old history"));

    mgr.append(sid, &message("", None, "user", "after compact"))
        .await
        .unwrap();
    assert_eq!(cold_before, tokio::fs::read(&cold_path).await.unwrap());

    let logical = mgr.load(sid).await.unwrap();
    let ids: Vec<_> = logical
        .iter()
        .filter_map(|entry| entry.entry_id())
        .collect();
    assert!(ids.contains(&"u1"));
    assert!(ids.contains(&"u2"));
    assert!(ids.iter().any(|id| id != &"u1" && id != &"u2"));
}

#[tokio::test]
async fn ordinary_append_keeps_compaction_append_only() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "append-only";
    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append_with_id(sid, &message("old", None, "user", "old"))
        .await
        .unwrap();
    mgr.append_with_id(sid, &message("keep", Some("old"), "assistant", "keep"))
        .await
        .unwrap();

    let compaction = SessionEntry::Compaction(CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "input-id".into(),
            parent_id: None,
            timestamp: 3,
        },
        summary: "summary".into(),
        first_kept_entry_id: "keep".into(),
        tokens_before: 10,
        details: None,
        from_hook: None,
        policy: None,
    });
    mgr.append_session_entry(sid, &compaction).await.unwrap();

    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(sessions.join(sid).join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        manifest.sealed_segments.is_empty(),
        "ordinary append must not trigger a seal"
    );
    let active = tokio::fs::read_to_string(sessions.join(sid).join(&manifest.active_segment.path))
        .await
        .unwrap();
    assert!(active.contains("\"type\":\"compaction\""));
}

#[tokio::test]
async fn leaf_branch_stops_at_compaction_cut_before_older_segment() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let sid = "cut";
    let session_dir = sessions.join(sid);
    tokio::fs::create_dir_all(session_dir.join("segments"))
        .await
        .unwrap();

    let kept = message("keep", Some("old"), "user", "kept");
    let compaction = SessionEntry::Compaction(CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "compact".into(),
            parent_id: Some("keep".into()),
            timestamp: 2,
        },
        summary: "old history".into(),
        first_kept_entry_id: "keep".into(),
        tokens_before: 10,
        details: None,
        from_hook: None,
        policy: None,
    });
    let tail = message("tail", Some("compact"), "assistant", "tail");
    let active_path = session_dir.join("active-2.jsonl");
    tokio::fs::write(
        &active_path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&compaction).unwrap(),
            serde_json::to_string(&tail).unwrap()
        ),
    )
    .await
    .unwrap();
    let kept_path = session_dir.join("segments/00000000000000000001-sealed.jsonl");
    tokio::fs::write(
        &kept_path,
        format!("{}\n", serde_json::to_string(&kept).unwrap()),
    )
    .await
    .unwrap();

    let manifest = SessionManifest {
        format_version: SESSION_VERSION,
        session_id: sid.into(),
        active_segment: SessionSegment {
            path: "active-2.jsonl".into(),
            generation: 2,
            first_entry_id: Some("compact".into()),
            last_entry_id: Some("tail".into()),
            includes_header: false,
        },
        sealed_segments: vec![
            SessionSegment {
                path: "segments/00000000000000000000-sealed.jsonl".into(),
                generation: 0,
                first_entry_id: Some("header".into()),
                last_entry_id: Some("old".into()),
                includes_header: true,
            },
            SessionSegment {
                path: "segments/00000000000000000001-sealed.jsonl".into(),
                generation: 1,
                first_entry_id: Some("keep".into()),
                last_entry_id: Some("keep".into()),
                includes_header: false,
            },
        ],
        leaf_entry_id: Some("tail".into()),
    };
    tokio::fs::write(
        session_dir.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )
    .await
    .unwrap();

    let mgr = SessionManager::new(sessions);
    let branch = mgr.get_branch(sid, None).await.unwrap();
    let ids: Vec<_> = branch.iter().filter_map(|entry| entry.entry_id()).collect();
    assert_eq!(ids, vec!["keep", "compact", "tail"]);
}

#[tokio::test]
async fn unreferenced_segment_does_not_affect_load() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "orphan";
    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &message("", None, "assistant", "persisted"))
        .await
        .unwrap();

    let session_dir = sessions.join(sid);
    tokio::fs::write(
        session_dir.join("orphan.jsonl"),
        b"{\"type\":\"unknown\",\"id\":\"orphan\"}\n",
    )
    .await
    .unwrap();
    let entries = mgr.load(sid).await.unwrap();
    assert!(entries.iter().any(|entry| {
        matches!(entry, SessionEntry::Message(message) if message.message.get("content").is_some())
    }));
    assert!(
        !entries
            .iter()
            .any(|entry| entry.entry_id() == Some("orphan"))
    );
}

#[tokio::test]
async fn v5_migration_is_rejected_without_deleting_legacy_file() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    tokio::fs::create_dir_all(&sessions).await.unwrap();
    let sid = "legacy-v5";
    let legacy = sessions.join(format!("{sid}.jsonl"));
    tokio::fs::write(
        &legacy,
        format!(
            "{}\n",
            serde_json::json!({
                "type": "session",
                "version": 5,
                "id": sid,
                "timestamp": 1,
                "cwd": "."
            })
        ),
    )
    .await
    .unwrap();

    let mgr = SessionManager::new(sessions.clone());
    let error = mgr.load(sid).await.unwrap_err();
    assert!(error.to_string().contains("only v6"));
    assert!(legacy.exists());
    assert!(!sessions.join(sid).join("manifest.json").exists());
}
