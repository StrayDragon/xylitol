//! Resume projection anchor: trailing parent-less bookkeeping rows (modelChange
//! written by cold materialize before any leaf is known) must not truncate
//! `get_branch` — the projection source for transcript rebuild and LLM context.

use super::super::*;
use crate::protocol::session::{
    EntryBase, MessageEntry, ModelChangeEntry, SessionEntry, fixture_message_json,
};

fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: 0,
        },
        message: fixture_message_json(role, text),
    })
}

fn parentless_model_change(id: &str) -> SessionEntry {
    SessionEntry::ModelChange(ModelChangeEntry {
        base: EntryBase {
            entry_type: "model_change".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 0,
        },
        provider: "fake".into(),
        model_id: "fake/m".into(),
    })
}

async fn seed_session_with_trailing_model_change(mgr: &SessionManager, sid: &str) {
    mgr.create(sid, Some("."), None).await.unwrap();
    if matches!(&mgr.backend, SessionBackend::Persisted { .. }) {
        mgr.flush_pending_to_disk(sid).await.unwrap();
    }
    for e in [
        msg("u1", None, "user", "first"),
        msg("a1", Some("u1"), "assistant", "reply one"),
        msg("u2", Some("a1"), "user", "second"),
        msg("a2", Some("u2"), "assistant", "reply two"),
        // Cold materialize artifact: bookkeeping row appended before any leaf
        // was known, so it carries no parent and sits at the tail.
        parentless_model_change("mc_tail"),
    ] {
        match &mgr.backend {
            SessionBackend::Persisted { .. } => {
                mgr.append_with_id(sid, &e).await.unwrap();
            }
            SessionBackend::InMemory { .. } => {
                let mut store = mgr.in_memory_store.write().expect("lock");
                store.entry(sid.to_string()).or_default().push(e.clone());
            }
        }
    }
}

#[tokio::test]
async fn get_branch_after_restart_anchors_past_parentless_model_change() {
    let dir = tempfile::tempdir().unwrap();
    let sid = "s-anchor-restart";
    {
        let mgr = SessionManager::new(dir.path().join("sessions"));
        seed_session_with_trailing_model_change(&mgr, sid).await;
    }
    // Fresh process: leaf map empty (resume after restart).
    let fresh = SessionManager::new(dir.path().join("sessions"));
    let branch = fresh.get_branch(sid, None).await.unwrap();
    let ids: Vec<_> = branch.iter().filter_map(|e| e.entry_id()).collect();
    assert_eq!(
        ids,
        vec!["u1", "a1", "u2", "a2"],
        "full message chain expected, got {ids:?}"
    );
}

#[tokio::test]
async fn get_branch_with_metadata_leaf_hint_anchors_past_it() {
    let mgr = SessionManager::in_memory();
    seed_session_with_trailing_model_change(&mgr, "s-anchor-meta").await;

    // Polluted stored leaf: update_leaf_from_entry kept the parent-less
    // bookkeeping row as leaf.
    let branch = mgr
        .get_branch("s-anchor-meta", Some("mc_tail"))
        .await
        .unwrap();
    let ids: Vec<_> = branch.iter().filter_map(|e| e.entry_id()).collect();
    assert_eq!(
        ids,
        vec!["u1", "a1", "u2", "a2"],
        "metadata leaf must not truncate history, got {ids:?}"
    );
}

#[tokio::test]
async fn get_branch_splices_seam_when_messages_chain_through_parentless_bookkeeping() {
    let mgr = SessionManager::in_memory();
    let sid = "s-anchor-seam";
    mgr.create(sid, Some("."), None).await.unwrap();
    if matches!(&mgr.backend, SessionBackend::Persisted { .. }) {
        mgr.flush_pending_to_disk(sid).await.unwrap();
    }
    for e in [
        msg("u1", None, "user", "first"),
        msg("a1", Some("u1"), "assistant", "reply one"),
        // Historical pollution: the seam row took the chain; later messages
        // chained through it while it itself has no parent.
        parentless_model_change("mc_seam"),
        msg("u2", Some("mc_seam"), "user", "second"),
        msg("a2", Some("u2"), "assistant", "reply two"),
    ] {
        match &mgr.backend {
            SessionBackend::Persisted { .. } => {
                mgr.append_with_id(sid, &e).await.unwrap();
            }
            SessionBackend::InMemory { .. } => {
                let mut store = mgr.in_memory_store.write().expect("lock");
                store.entry(sid.to_string()).or_default().push(e.clone());
            }
        }
    }

    let branch = mgr.get_branch(sid, None).await.unwrap();
    let ids: Vec<_> = branch.iter().filter_map(|e| e.entry_id()).collect();
    assert_eq!(
        ids,
        vec!["u1", "a1", "mc_seam", "u2", "a2"],
        "seam splice must keep full history for LLM context, got {ids:?}"
    );
}
