use std::path::PathBuf;

use crate::agent::compaction::CompactionKind;
use crate::infra::session::config::{SessionConfig, StorageConfig};
use crate::infra::session::gc::PruneStrategy;
use crate::infra::session::manager::{ContextFilter, SnapshotBuilder, SnapshotManager};
use crate::infra::session::types::*;

fn make_manager() -> SnapshotManager {
    SnapshotManager::new(SessionConfig::default())
}

fn basic_builder() -> SnapshotBuilder {
    SnapshotBuilder {
        project_root: PathBuf::from("/tmp/test"),
        project_hash: "test-hash".into(),
        model_id: "test-model".into(),
        tags: vec!["test".into()],
        conversation: vec![ConversationTurn {
            role: ConversationRole::User,
            content: "hello".into(),
            tool_calls: None,
            timestamp: TimestampMillis::now(),
            deprecated: false,
        }],
        project_cognition: ProjectCognition::empty(),
        tool_call_log: vec![],
        config_fingerprint: ConfigFingerprint {
            features: vec!["infra-session".into()],
            config_hash: "cfg-hash".into(),
        },
    }
}

// ── Snapshot / Restore ─────────────────────────────────────────────────

#[test]
fn test_snapshot_and_restore() {
    let mut mgr = make_manager();
    let id = mgr.snapshot(basic_builder()).unwrap();

    let restored = mgr.restore(&id).unwrap();
    assert_eq!(restored.id, id);
    assert_eq!(restored.meta.project_hash, "test-hash");
    assert_eq!(restored.conversation.len(), 1);
}

#[test]
fn test_restore_nonexistent_returns_error() {
    let mgr = make_manager();
    assert!(mgr.restore(&"nonexistent".into()).is_err());
}

// ── List ───────────────────────────────────────────────────────────────

#[test]
fn test_list_snapshots() {
    let mut mgr = make_manager();
    assert!(mgr.list().unwrap().is_empty());

    mgr.snapshot(basic_builder()).unwrap();
    mgr.snapshot(basic_builder()).unwrap();

    let list = mgr.list().unwrap();
    assert_eq!(list.len(), 2);
    // Most recent first
    assert!(list[0].created >= list[1].created);
}

// ── Spawn (derive) ────────────────────────────────────────────────────

#[test]
fn test_spawn_from_snapshot() {
    let mut mgr = make_manager();

    // Make parent with cognition.
    let mut builder = basic_builder();
    builder.project_cognition.code_summaries.insert(
        "src/main.rs".into(),
        CodeSummary {
            summary: "Entry point".into(),
            symbols: vec!["main".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    let parent_id = mgr.snapshot(builder).unwrap();

    // Spawn with no filter → full inheritance.
    let child_id = mgr.spawn(&parent_id, None).unwrap();
    assert_ne!(child_id, parent_id);

    let child = mgr.restore(&child_id).unwrap();
    assert_eq!(child.parent_snapshot_id, Some(parent_id));
    // Cognition should be inherited.
    assert_eq!(child.project_cognition.code_summaries.len(), 1);
}

#[test]
fn test_spawn_with_context_filter() {
    let mut mgr = make_manager();

    let mut builder = basic_builder();
    builder.project_cognition.code_summaries.insert(
        "src/main.rs".into(),
        CodeSummary {
            summary: "Entry point".into(),
            symbols: vec!["main".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    builder.project_cognition.code_summaries.insert(
        "src/lib.rs".into(),
        CodeSummary {
            summary: "Library".into(),
            symbols: vec!["helper".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    let parent_id = mgr.snapshot(builder).unwrap();

    // Spawn with filter: only keep "main.rs".
    let filter = ContextFilter {
        files_include: vec!["main.rs".into()],
        ..Default::default()
    };
    let child_id = mgr.spawn(&parent_id, Some(filter)).unwrap();
    let child = mgr.restore(&child_id).unwrap();
    assert_eq!(child.project_cognition.code_summaries.len(), 1);
    assert!(
        child
            .project_cognition
            .code_summaries
            .contains_key("src/main.rs")
    );
}

// ── Prune ──────────────────────────────────────────────────────────────

#[test]
fn test_prune_keep_latest() {
    let mut mgr = make_manager();
    mgr.snapshot(basic_builder()).unwrap();
    mgr.snapshot(basic_builder()).unwrap();
    mgr.snapshot(basic_builder()).unwrap();
    assert_eq!(mgr.list().unwrap().len(), 3);

    let removed = mgr.prune(PruneStrategy::KeepLatest(1)).unwrap();
    assert_eq!(removed, 2);
    assert_eq!(mgr.list().unwrap().len(), 1);
}

// ── Diff ────────────────────────────────────────────────────────────────

#[test]
fn test_diff_two_snapshots() {
    let mut mgr = make_manager();

    let id1 = mgr.snapshot(basic_builder()).unwrap();

    let mut builder = basic_builder();
    let mut cognition = ProjectCognition::empty();
    cognition.code_summaries.insert(
        "src/new.rs".into(),
        CodeSummary {
            summary: "New module".into(),
            symbols: vec!["new_fn".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    builder.project_cognition = cognition;
    let id2 = mgr.snapshot(builder).unwrap();

    let diff = mgr.diff(&id1, &id2).unwrap();
    assert_eq!(diff.id_a, id1);
    assert_eq!(diff.id_b, id2);
    assert!(!diff.cognition_changes.is_empty());
}

// ── Merge ──────────────────────────────────────────────────────────────

#[test]
fn test_merge_cognition() {
    let mut mgr = make_manager();

    // Source: has cognition
    let mut src_builder = basic_builder();
    let mut src_cog = ProjectCognition::empty();
    src_cog.code_summaries.insert(
        "src/foo.rs".into(),
        CodeSummary {
            summary: "Foo module".into(),
            symbols: vec!["foo".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    src_builder.project_cognition = src_cog;
    let src_id = mgr.snapshot(src_builder).unwrap();

    // Target: has different cognition
    let mut tgt_builder = basic_builder();
    let mut tgt_cog = ProjectCognition::empty();
    tgt_cog.code_summaries.insert(
        "src/bar.rs".into(),
        CodeSummary {
            summary: "Bar module".into(),
            symbols: vec!["bar".into()],
            last_indexed: TimestampMillis::now(),
            stale: false,
        },
    );
    tgt_builder.project_cognition = tgt_cog;
    let tgt_id = mgr.snapshot(tgt_builder).unwrap();

    let merged_id = mgr.merge(&src_id, &tgt_id).unwrap();
    let merged = mgr.restore(&merged_id).unwrap();
    assert_eq!(merged.project_cognition.code_summaries.len(), 2);
}

// ── Compaction ─────────────────────────────────────────────────────────

#[test]
fn test_compact_snapshot() {
    let mut mgr = make_manager();
    let mut builder = basic_builder();
    for i in 0..10 {
        builder.conversation.push(ConversationTurn {
            role: ConversationRole::User,
            content: format!("msg {i}"),
            tool_calls: None,
            timestamp: TimestampMillis::now(),
            deprecated: false,
        });
    }
    let id = mgr.snapshot(builder).unwrap();

    let compacted_id = mgr.compact(&id, CompactionKind::Intra).unwrap();
    let compacted = mgr.restore(&compacted_id).unwrap();

    // Conversation should be reduced (11 turns → compacted down)
    assert!(compacted.conversation.len() < 11);
    // First turn should be a system summary
    assert!(matches!(
        compacted.conversation[0].role,
        ConversationRole::System
    ));
}

// ── Serialisation round-trip ──────────────────────────────────────────

#[test]
fn test_msgpack_zstd_roundtrip() {
    let store = crate::infra::session::storage::SnapshotStore::new(StorageConfig::default());

    let snap = Snapshot {
        id: "roundtrip-test".into(),
        created: TimestampMillis::now(),
        parent_snapshot_id: None,
        meta: SnapshotMeta {
            project_root: PathBuf::from("/p"),
            project_hash: "h".into(),
            model_id: "m".into(),
            tags: vec!["t".into()],
        },
        conversation: vec![ConversationTurn {
            role: ConversationRole::System,
            content: "test content".into(),
            tool_calls: None,
            timestamp: TimestampMillis::now(),
            deprecated: false,
        }],
        project_cognition: ProjectCognition {
            code_summaries: std::collections::HashMap::new(),
            codebase_graph: CodebaseGraph {
                nodes: vec!["a".into()],
                edges: vec![("a".into(), "b".into())],
            },
            debugger_state: None,
        },
        tool_call_log: vec![ToolCallSummary {
            tool: "test_tool".into(),
            params: serde_json::json!({"key": "value"}),
            result_summary: "ok".into(),
            tokens_consumed: 100,
        }],
        config_fingerprint: ConfigFingerprint {
            features: vec!["test-feat".into()],
            config_hash: "ch".into(),
        },
    };

    let blob = store.encode(&snap).unwrap();
    assert!(!blob.is_empty());

    let decoded = store.decode(&blob).unwrap();
    assert_eq!(decoded.id, snap.id);
    assert_eq!(decoded.conversation[0].content, "test content");
    assert_eq!(decoded.tool_call_log[0].tool, "test_tool");
    assert_eq!(decoded.project_cognition.codebase_graph.nodes.len(), 1);
}
