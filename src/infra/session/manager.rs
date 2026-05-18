use std::path::PathBuf;

use anyhow::Result;
use tracing::instrument;

use super::compaction::{CompactionKind, compact_conversation};
use super::config::{CompactionStrategy, SessionConfig};
use super::gc::PruneStrategy;
use super::storage::SnapshotStore;
use super::types::*;

/// Summary of a snapshot (for listing).
#[derive(Debug, Clone)]
pub struct SnapshotSummary {
    pub id: SnapshotId,
    pub created: TimestampMillis,
    pub parent_id: Option<SnapshotId>,
    pub model_id: String,
    pub tags: Vec<String>,
    pub turn_count: usize,
    pub cognition_size: usize,
}

/// Filter for context pruning during spawn.
#[derive(Debug, Clone, Default)]
pub struct ContextFilter {
    /// Only keep code summaries for files matching these patterns.
    pub files_include: Vec<String>,
    /// Exclude files matching these patterns.
    pub files_exclude: Vec<String>,
    /// Deprecate conversation turns older than this many seconds.
    pub deprecate_older_than_secs: Option<i64>,
}

/// Result of a diff operation.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub id_a: SnapshotId,
    pub id_b: SnapshotId,
    pub cognition_changes: Vec<String>,
    pub conversation_changes: Vec<String>,
    pub tool_call_diffs: Vec<String>,
}

/// Builder for creating new snapshots from agent state.
#[derive(Debug, Clone, Default)]
pub struct SnapshotBuilder {
    pub project_root: PathBuf,
    pub project_hash: String,
    pub model_id: String,
    pub tags: Vec<String>,
    pub conversation: Vec<ConversationTurn>,
    pub project_cognition: ProjectCognition,
    pub tool_call_log: Vec<ToolCallSummary>,
    pub config_fingerprint: ConfigFingerprint,
}

impl SnapshotBuilder {
    pub fn build(self, parent_id: Option<SnapshotId>) -> Snapshot {
        let id = uuid::Uuid::new_v4().to_string();
        Snapshot {
            id,
            created: TimestampMillis::now(),
            parent_snapshot_id: parent_id,
            meta: SnapshotMeta {
                project_root: self.project_root,
                project_hash: self.project_hash,
                model_id: self.model_id,
                tags: self.tags,
            },
            conversation: self.conversation,
            project_cognition: self.project_cognition,
            tool_call_log: self.tool_call_log,
            config_fingerprint: self.config_fingerprint,
        }
    }
}

/// Central snapshot manager — all snapshot operations go through this.
pub struct SnapshotManager {
    pub(crate) config: SessionConfig,
    pub(crate) store: SnapshotStore,
}

impl SnapshotManager {
    pub fn new(config: SessionConfig) -> Self {
        let store = SnapshotStore::new(config.storage.clone());
        Self { config, store }
    }

    // ── Core operations ────────────────────────────────────────────────

    /// Create a new snapshot from the given builder.
    #[instrument(skip(self, builder))]
    pub fn snapshot(&mut self, builder: SnapshotBuilder) -> Result<SnapshotId> {
        let id = uuid::Uuid::new_v4().to_string();
        let snapshot = Snapshot {
            id: id.clone(),
            created: TimestampMillis::now(),
            parent_snapshot_id: None,
            meta: SnapshotMeta {
                project_root: builder.project_root,
                project_hash: builder.project_hash,
                model_id: builder.model_id,
                tags: builder.tags,
            },
            conversation: builder.conversation,
            project_cognition: builder.project_cognition,
            tool_call_log: builder.tool_call_log,
            config_fingerprint: builder.config_fingerprint,
        };
        self.store.store(&snapshot)?;
        self.enforce_max_snapshots()?;
        Ok(id)
    }

    /// Load a snapshot by ID.
    pub fn restore(&self, id: &SnapshotId) -> Result<Snapshot> {
        let snapshot = self.store.load(id)?;
        // Stale warning: checked vs. current project hash by caller.
        Ok(snapshot)
    }

    /// Derive a new snapshot from a parent, with optional context pruning.
    #[instrument(skip(self))]
    pub fn spawn(
        &mut self,
        parent_id: &SnapshotId,
        filter: Option<ContextFilter>,
    ) -> Result<SnapshotId> {
        let parent = self.store.load(parent_id)?;
        let child = self.apply_filter(parent, filter);
        self.store.store(&child)?;
        self.enforce_max_snapshots()?;
        Ok(child.id)
    }

    /// List all stored snapshots with summary info.
    pub fn list(&self) -> Result<Vec<SnapshotSummary>> {
        let mut summaries = Vec::new();
        for id in self.store.list_ids() {
            let snap = self.store.load(&id)?;
            summaries.push(SnapshotSummary {
                id: snap.id,
                created: snap.created,
                parent_id: snap.parent_snapshot_id,
                model_id: snap.meta.model_id,
                tags: snap.meta.tags,
                turn_count: snap.conversation.len(),
                cognition_size: snap.project_cognition.code_summaries.len(),
            });
        }
        summaries.sort_by_key(|s| std::cmp::Reverse(s.created));
        Ok(summaries)
    }

    /// Remove snapshots according to the given strategy.
    #[instrument(skip(self))]
    pub fn prune(&mut self, strategy: PruneStrategy) -> Result<usize> {
        let ids_to_remove: Vec<SnapshotId> = match strategy {
            PruneStrategy::KeepLatest(n) => {
                let mut all = self.list()?;
                all.sort_by_key(|s| s.created);
                all.iter()
                    .take(all.len().saturating_sub(n))
                    .map(|s| s.id.clone())
                    .collect()
            }
            PruneStrategy::OlderThan(duration) => {
                let cutoff_ms = duration.num_milliseconds();
                let cutoff = TimestampMillis::now()
                    .checked_sub_millis(cutoff_ms)
                    .unwrap_or(TimestampMillis::now());
                self.list()?
                    .into_iter()
                    .filter(|s| s.created < cutoff)
                    .map(|s| s.id)
                    .collect()
            }
            PruneStrategy::Tagged(tag) => self
                .list()?
                .into_iter()
                .filter(|s| s.tags.contains(&tag))
                .map(|s| s.id)
                .collect(),
        };

        let count = ids_to_remove.len();
        for id in &ids_to_remove {
            self.store.delete(id)?;
        }
        Ok(count)
    }

    /// Diff two snapshots.
    pub fn diff(&self, id_a: &SnapshotId, id_b: &SnapshotId) -> Result<DiffReport> {
        let a = self.store.load(id_a)?;
        let b = self.store.load(id_b)?;

        let mut cognition_changes = Vec::new();

        // Compare code summaries
        for (key, summary_a) in &a.project_cognition.code_summaries {
            match b.project_cognition.code_summaries.get(key) {
                None => cognition_changes.push(format!("code_summary removed: {key}")),
                Some(summary_b) if summary_a.summary != summary_b.summary => {
                    cognition_changes.push(format!("code_summary changed: {key}"));
                }
                _ => {}
            }
        }
        for key in b.project_cognition.code_summaries.keys() {
            if !a.project_cognition.code_summaries.contains_key(key) {
                cognition_changes.push(format!("code_summary added: {key}"));
            }
        }

        // Compare conversation
        let conversation_changes = vec![format!(
            "turns: {} -> {}",
            a.conversation.len(),
            b.conversation.len()
        )];

        // Compare tool calls
        let tool_call_diffs = vec![format!(
            "tool calls: {} -> {}",
            a.tool_call_log.len(),
            b.tool_call_log.len()
        )];

        Ok(DiffReport {
            id_a: id_a.clone(),
            id_b: id_b.clone(),
            cognition_changes,
            conversation_changes,
            tool_call_diffs,
        })
    }

    /// Merge project cognition from source into target.
    /// Conversation is NOT merged (design decision — only cognition).
    #[instrument(skip(self))]
    pub fn merge(&mut self, source_id: &SnapshotId, target_id: &SnapshotId) -> Result<SnapshotId> {
        let source = self.store.load(source_id)?;
        let mut target = self.store.load(target_id)?;

        // Merge code summaries (source overwrites target on key conflict).
        for (key, summary) in source.project_cognition.code_summaries {
            target.project_cognition.code_summaries.insert(key, summary);
        }

        // Merge graph nodes/edges (deduplicate).
        let existing_nodes: std::collections::HashSet<_> = target
            .project_cognition
            .codebase_graph
            .nodes
            .iter()
            .cloned()
            .collect();
        for node in &source.project_cognition.codebase_graph.nodes {
            if !existing_nodes.contains(node) {
                target
                    .project_cognition
                    .codebase_graph
                    .nodes
                    .push(node.clone());
            }
        }
        let existing_edges: std::collections::HashSet<_> = target
            .project_cognition
            .codebase_graph
            .edges
            .iter()
            .cloned()
            .collect();
        for edge in &source.project_cognition.codebase_graph.edges {
            if !existing_edges.contains(edge) {
                target
                    .project_cognition
                    .codebase_graph
                    .edges
                    .push(edge.clone());
            }
        }

        // Save as a new snapshot (immutable — merge produces a new version).
        // Preserve the original target's conversation and config.
        let merged_id = uuid::Uuid::new_v4().to_string();
        let merged = Snapshot {
            id: merged_id.clone(),
            created: TimestampMillis::now(),
            parent_snapshot_id: Some(target_id.clone()),
            meta: target.meta,
            conversation: target.conversation,
            project_cognition: target.project_cognition,
            tool_call_log: target.tool_call_log,
            config_fingerprint: target.config_fingerprint,
        };
        self.store.store(&merged)?;
        Ok(merged_id)
    }

    // ── Compaction ─────────────────────────────────────────────────────

    /// Compact a snapshot's conversation, producing a new compacted snapshot.
    #[instrument(skip(self))]
    pub fn compact(&mut self, id: &SnapshotId, kind: CompactionKind) -> Result<SnapshotId> {
        let snapshot = self.store.load(id)?;
        let max_turns = match &self.config.compaction.strategy {
            CompactionStrategy::Intra | CompactionStrategy::Manual | CompactionStrategy::Derive => {
                // Keep at most 1/4 of the conversation turns; the rest gets
                // summarised into a system message.
                std::cmp::max(1, snapshot.conversation.len() / 4)
            }
        };

        let compacted = compact_conversation(&snapshot, max_turns, kind)?;

        let new_id = uuid::Uuid::new_v4().to_string();
        let new_snapshot = Snapshot {
            id: new_id.clone(),
            created: TimestampMillis::now(),
            parent_snapshot_id: Some(id.clone()),
            meta: snapshot.meta,
            conversation: compacted,
            project_cognition: snapshot.project_cognition,
            tool_call_log: snapshot.tool_call_log,
            config_fingerprint: snapshot.config_fingerprint,
        };
        self.store.store(&new_snapshot)?;
        Ok(new_id)
    }

    // ── Internal helpers ───────────────────────────────────────────────

    fn apply_filter(&self, mut parent: Snapshot, filter: Option<ContextFilter>) -> Snapshot {
        // Always assign new ID and parent relationship.
        let original_id = parent.id.clone();
        parent.id = uuid::Uuid::new_v4().to_string();
        parent.parent_snapshot_id = Some(original_id);

        let Some(filter) = filter else {
            return parent;
        };

        // Filter code summaries.
        if !filter.files_include.is_empty() {
            parent
                .project_cognition
                .code_summaries
                .retain(|key, _| filter.files_include.iter().any(|pat| key.contains(pat)));
        }
        for exclude in &filter.files_exclude {
            parent
                .project_cognition
                .code_summaries
                .retain(|key, _| !key.contains(exclude));
        }

        // Deprecate old conversation turns.
        if let Some(older_than_secs) = filter.deprecate_older_than_secs {
            let cutoff_ms = older_than_secs * 1000;
            let cutoff = TimestampMillis::now()
                .checked_sub_millis(cutoff_ms)
                .unwrap_or(TimestampMillis::now());
            for turn in &mut parent.conversation {
                if turn.timestamp < cutoff {
                    turn.deprecated = true;
                }
            }
        }

        parent
    }

    fn enforce_max_snapshots(&mut self) -> Result<()> {
        let count = self.store.len();
        if count > self.config.max_snapshots {
            let to_remove = count - self.config.max_snapshots;
            self.prune(PruneStrategy::KeepLatest(self.config.max_snapshots))?;
            tracing::debug!("pruned {to_remove} snapshots to maintain max_snapshots limit");
        }
        Ok(())
    }
}
