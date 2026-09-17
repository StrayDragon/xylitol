use super::super::*;
use crate::protocol::ports::XySessionStore;
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
async fn ordinary_append_refreshes_manifest_leaf_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "manifest-fresh";

    mgr.create(sid, Some("."), None).await.unwrap();
    mgr.append(sid, &message("", None, "user", "hello"))
        .await
        .unwrap();
    mgr.append(sid, &message("", None, "assistant", "world"))
        .await
        .unwrap();

    let entries = mgr.load(sid).await.unwrap();
    let leaf_id = entries.last().and_then(SessionEntry::entry_id).unwrap();
    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(sessions.join(sid).join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest.active_segment.last_entry_id.as_deref(),
        Some(leaf_id)
    );
    assert_eq!(manifest.leaf_entry_id.as_deref(), Some(leaf_id));
}

#[tokio::test]
async fn invalid_manifest_rejects_append_without_creating_orphan_segment() {
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "invalid-manifest";

    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in [
        message("old", None, "user", "old"),
        message("keep", Some("old"), "assistant", "keep"),
    ] {
        mgr.append_with_id(sid, &entry).await.unwrap();
    }
    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
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
        }),
    )
    .await
    .unwrap();

    let session_dir = sessions.join(sid);
    let manifest_path = session_dir.join("manifest.json");
    let manifest: SessionManifest =
        serde_json::from_slice(&tokio::fs::read(&manifest_path).await.unwrap()).unwrap();
    let active_path = session_dir.join(&manifest.active_segment.path);
    let active_before = tokio::fs::read(&active_path).await.unwrap();
    tokio::fs::write(&manifest_path, b"{not-json")
        .await
        .unwrap();

    let error = mgr
        .append_with_id(
            sid,
            &message("new", Some("keep"), "user", "must not persist"),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("invalid session manifest"));
    assert_eq!(active_before, tokio::fs::read(&active_path).await.unwrap());
    assert!(
        !session_dir.join("active-0.jsonl").exists(),
        "invalid manifest must not route writes to the fallback segment"
    );
}

#[tokio::test]
async fn v6_file_is_rejected_at_legacy_boundary_and_preserved() {
    // Zero-compat (c2810): a legacy-only session must NOT be migrated,
    // silently deleted, or overwritten; reads surface an actionable error and
    // keep the file for an explicit delete_session.
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
    let error = mgr.load(sid).await.unwrap_err();
    assert!(
        error.to_string().contains("legacy"),
        "actionable error: {error}"
    );
    assert!(legacy.exists(), "legacy file preserved after rejected read");
    assert!(
        !sessions.join(sid).join("manifest.json").exists(),
        "no v7 manifest created for a legacy session"
    );

    // Explicit delete still cleans the legacy file.
    mgr.delete_session(sid).await.unwrap();
    assert!(!legacy.exists(), "explicit delete removes legacy file");
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
    let leaf_id = mgr
        .load(sid)
        .await
        .unwrap()
        .last()
        .and_then(SessionEntry::entry_id)
        .unwrap()
        .to_owned();
    assert_eq!(
        manifest.active_segment.last_entry_id.as_deref(),
        Some(leaf_id.as_str())
    );
    assert_eq!(manifest.leaf_entry_id.as_deref(), Some(leaf_id.as_str()));
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
            index_path: None,
        },
        sealed_segments: vec![
            SessionSegment {
                path: "segments/00000000000000000000-sealed.jsonl".into(),
                generation: 0,
                first_entry_id: Some("header".into()),
                last_entry_id: Some("old".into()),
                includes_header: true,
                index_path: None,
            },
            SessionSegment {
                path: "segments/00000000000000000001-sealed.jsonl".into(),
                generation: 1,
                first_entry_id: Some("keep".into()),
                last_entry_id: Some("keep".into()),
                includes_header: false,
                index_path: None,
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
    assert!(
        error.to_string().contains("legacy"),
        "zero-compat boundary error: {error}"
    );
    assert!(legacy.exists());
    assert!(!sessions.join(sid).join("manifest.json").exists());
}

#[tokio::test]
async fn seal_writes_sidecar_index_and_manifest_index_path() {
    // T6: after a compaction seal, a sidecar index exists next to the cold
    // JSONL and the manifest's sealed segment records its index path.
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "sidecar-seal";
    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in [
        message("u1", None, "user", "old user"),
        message("a1", Some("u1"), "assistant", "old answer"),
        message("u2", Some("a1"), "user", "kept user"),
    ] {
        mgr.append_with_id(sid, &entry).await.unwrap();
    }

    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "comp".into(),
                parent_id: None,
                timestamp: 3,
            },
            summary: "sum".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: None,
        }),
    )
    .await
    .unwrap();

    let session_dir = sessions.join(sid);
    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(session_dir.join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest.sealed_segments.len(), 1);
    let index_path = manifest.sealed_segments[0]
        .index_path
        .as_deref()
        .expect("seal must record index path");
    assert!(index_path.ends_with(".index.json"));
    let index_text = tokio::fs::read_to_string(session_dir.join(index_path))
        .await
        .unwrap();
    let index: crate::protocol::session::SealedIndex = serde_json::from_str(&index_text).unwrap();
    assert!(index.entry_ids.contains(&"u1".to_string()));
    assert!(index.entry_ids.contains(&"a1".to_string()));
    assert!(
        !index.entry_ids.contains(&"u2".to_string()),
        "tail stays active"
    );
}

#[tokio::test]
async fn done_bash_ids_merge_sidecar_and_fallback_on_corruption() {
    // T6: done-bash pairing uses sealed sidecars for cold segments, keeps the
    // active-scan semantics, and falls back to a full scan when a sidecar is
    // corrupt or absent (no false negatives).
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "done-sidecar";
    mgr.create(sid, Some("."), None).await.unwrap();
    // Use `append` so the store injects real non-empty entry ids (the bash
    // helper produces an empty shell id).
    mgr.append(
        sid,
        &crate::protocol::session::bash_execution_message_entry(
            "b1",
            "echo old",
            "old out",
            None,
            false,
            false,
            None,
            false,
            crate::protocol::message::BashExecutionStatus::Done,
        ),
    )
    .await
    .unwrap();
    // Tail is still running (orphan until a done row appears in a sealed seg).
    mgr.append(
        sid,
        &crate::protocol::session::bash_execution_message_entry(
            "b2",
            "sleep",
            "",
            None,
            false,
            false,
            None,
            true,
            crate::protocol::message::BashExecutionStatus::Running,
        ),
    )
    .await
    .unwrap();
    // The injected running entry id becomes the first-kept compaction boundary.
    let all = mgr.load(sid).await.unwrap();
    let running_id = all
        .iter()
        .rev()
        .find_map(|entry| {
            entry
                .entry_id()
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
        })
        .expect("injected running entry id");
    assert_ne!(running_id, "b1");

    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "comp".into(),
                parent_id: None,
                timestamp: 3,
            },
            summary: "sum".into(),
            first_kept_entry_id: running_id.clone(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: None,
        }),
    )
    .await
    .unwrap();

    // b1 lives in the sealed segment with bash_id "b1"; active is the tail.
    let done = mgr.load_done_bash_ids(sid).await.unwrap();
    assert!(
        done.contains("b1"),
        "sealed done must be paired via sidecar (done={done:?})"
    );

    // Corrupt the sidecar and confirm the fallback scan still finds b1.
    let session_dir = sessions.join(sid);
    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(session_dir.join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    let index_path = manifest.sealed_segments[0].index_path.as_deref().unwrap();
    tokio::fs::write(session_dir.join(index_path), "{ not json")
        .await
        .unwrap();
    let done_fallback = mgr.load_done_bash_ids(sid).await.unwrap();
    assert!(
        done_fallback.contains("b1"),
        "corrupt sidecar must fall back to scanning (no false negative)"
    );

    // Absent index path (old manifest shape) also falls back.
    tokio::fs::remove_file(session_dir.join(index_path))
        .await
        .unwrap();
    let done_old_manifest = mgr.load_done_bash_ids(sid).await.unwrap();
    assert!(done_old_manifest.contains("b1"));
}

#[tokio::test]
async fn leaf_branch_resolver_uses_sidecar_screening_and_falls_back() {
    // T6: the leaf resolver screens sealed segments through sidecar entryIds,
    // so a cold-only ancestor resolves without reading candidates the index
    // says cannot contain it; a missing sidecar still resolves (fallback).
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "resolver";
    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in [
        message("u1", None, "user", "root"),
        message("a1", Some("u1"), "assistant", "first answer"),
        message("u2", Some("a1"), "user", "kept"),
        message("a2", Some("u2"), "assistant", "kept answer"),
    ] {
        mgr.append_with_id(sid, &entry).await.unwrap();
    }
    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "comp".into(),
                parent_id: None,
                timestamp: 3,
            },
            summary: "sum".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: None,
        }),
    )
    .await
    .unwrap();

    let branch = mgr.get_branch(sid, Some("a2")).await.unwrap();
    let ids: Vec<_> = branch.iter().filter_map(|e| e.entry_id()).collect();
    assert!(
        ids.contains(&"u1"),
        "cold ancestor resolves via sidecar screen"
    );
    assert!(ids.contains(&"a1"));
    assert!(ids.contains(&"u2"));

    // Remove the sidecar: resolution must still succeed (fallback scan).
    let session_dir = sessions.join(sid);
    let manifest: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(session_dir.join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    let index_path = manifest.sealed_segments[0].index_path.as_deref().unwrap();
    tokio::fs::remove_file(session_dir.join(index_path))
        .await
        .unwrap();
    let branch_fallback = mgr.get_branch(sid, Some("a2")).await.unwrap();
    let ids_fb: Vec<_> = branch_fallback
        .iter()
        .filter_map(|e| e.entry_id())
        .collect();
    assert!(
        ids_fb.contains(&"u1"),
        "missing sidecar falls back to scanning branch"
    );
}

#[tokio::test]
async fn replace_entries_removes_sealed_sidescar_index() {
    // T6: rewriting a session's layout (replace_entries) must also drop the
    // old sealed sidecar files so they never linger as permanent orphans.
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    let mgr = SessionManager::new(sessions.clone());
    let sid = "replace";
    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in [
        message("u1", None, "user", "old"),
        message("a1", Some("u1"), "assistant", "a"),
        message("u2", Some("a1"), "user", "kept"),
    ] {
        mgr.append_with_id(sid, &entry).await.unwrap();
    }
    mgr.commit_compaction(
        sid,
        &SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "comp".into(),
                parent_id: None,
                timestamp: 3,
            },
            summary: "sum".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: None,
        }),
    )
    .await
    .unwrap();
    let session_dir = sessions.join(sid);
    let before: SessionManifest = serde_json::from_slice(
        &tokio::fs::read(session_dir.join("manifest.json"))
            .await
            .unwrap(),
    )
    .unwrap();
    let old_index = before.sealed_segments[0]
        .index_path
        .as_deref()
        .unwrap()
        .to_string();
    let index_file = session_dir.join(&old_index);
    assert!(index_file.exists());

    // Flatten everything back into one active segment via replace_entries.
    let all = mgr.load(sid).await.unwrap();
    mgr.replace_entries(sid, &all).await.unwrap();
    assert!(
        !index_file.exists(),
        "replace_entries must clean up the sealed sidecar"
    );
}
