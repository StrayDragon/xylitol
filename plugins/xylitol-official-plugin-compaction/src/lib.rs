pub mod binding;
pub mod plugin;
pub use binding::{CompactionModelAttribution, CompactionSummaryBinding};
pub mod cut_detector;
pub mod file_ops;
pub mod llm_summarizer;
pub mod obs;
pub mod orchestrator;
pub mod overflow;
pub mod settings;
pub mod settlement;
pub mod token_estimator;

pub use orchestrator::{CompactionOrchestrator, OverflowCompactOutcome};

pub use cut_detector::estimate_tokens_entry_for_cut;
pub use cut_detector::{estimate_tokens_entry, find_cut_point};
pub use file_ops::FileOps;
pub use file_ops::{compute_file_lists, extract_file_ops_from_messages, format_file_ops_xml};
pub use llm_summarizer::{generate_summary, generate_turn_prefix_summary};
pub use orchestrator::should_compact;
pub use overflow::error_message_is_context_overflow;
pub use overflow::{assistant_same_model, is_context_overflow_assistant};
pub use settings::CompactionSettings;
pub use settlement::{
    ContextTokenSettlement, ContextTokenSettlementReason, settle_from_session_entries,
};
pub use token_estimator::{EstimateOpts, FixedRequestContext, estimate_from_session_entries};

use anyhow::Result;
use serde_json::json;

use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::session::{CompactionEntry, EntryBase, MessageEntry, SessionEntry};

pub use xylitol_plugin_sdk::CompactionError;

fn timestamp_now() -> u64 {
    xylitol_plugin_sdk::message::now_ms()
}

pub(crate) fn effective_keep_budget(
    settings: &CompactionSettings,
    context_window: u64,
    fixed_overhead_tokens: u64,
) -> u64 {
    if context_window == 0 {
        return settings.keep_recent_tokens;
    }
    settings.keep_recent_tokens.min(
        context_window
            .saturating_sub(settings.reserve_tokens)
            .saturating_sub(fixed_overhead_tokens),
    )
}

const SUMMARY_PLACEHOLDER_TOKENS: u64 = 2_048;

pub fn summary_placeholder_tokens(entries: &[SessionEntry]) -> u64 {
    entries
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => {
                let est =
                    xylitol_plugin_sdk::accounting::heuristic_token_count(c.summary.len() as u64);
                Some(if est == 0 {
                    SUMMARY_PLACEHOLDER_TOKENS
                } else {
                    est
                })
            }
            _ => None,
        })
        .unwrap_or(SUMMARY_PLACEHOLDER_TOKENS)
}

pub fn projected_post_compact_tokens(
    settings: &CompactionSettings,
    context_window: u64,
    fixed_overhead_tokens: u64,
    summary_placeholder: u64,
) -> u64 {
    fixed_overhead_tokens
        + effective_keep_budget(settings, context_window, fixed_overhead_tokens)
        + summary_placeholder
}

pub use xylitol_plugin_sdk::effective_trigger_threshold;

fn boundary_start(entries: &[SessionEntry]) -> usize {
    for (i, entry) in entries.iter().enumerate().rev() {
        if let SessionEntry::Compaction(comp) = entry {
            let first_kept_id = comp.first_kept_entry_id.as_str();
            return entries
                .iter()
                .position(|e| e.entry_id() == Some(first_kept_id))
                .unwrap_or(i + 1);
        }
    }
    0
}

fn last_context_entry_is_compaction(entries: &[SessionEntry]) -> bool {
    entries
        .iter()
        .rev()
        .find(|entry| {
            matches!(
                entry,
                SessionEntry::Message(_) | SessionEntry::Compaction(_)
            )
        })
        .is_some_and(|entry| matches!(entry, SessionEntry::Compaction(_)))
}

pub fn prepare_compaction(
    entries: &[SessionEntry],
    settings: &CompactionSettings,
    context_window: u64,
    fixed_overhead_tokens: u64,
) -> Result<(), CompactionError> {
    if entries.is_empty() {
        return Err("Nothing to compact (empty session)".into());
    }
    if last_context_entry_is_compaction(entries) {
        return Err("Already compacted".into());
    }

    let boundary_start = boundary_start(entries);
    let boundary_end = entries.len();
    let cut = find_cut_point(
        entries,
        boundary_start,
        boundary_end,
        effective_keep_budget(settings, context_window, fixed_overhead_tokens),
    );
    let first_kept = &entries[cut.first_kept_entry_index];
    if first_kept.entry_id().is_none() {
        return Err("Nothing to compact (no summarizable history beyond keep window)".into());
    }
    let history_end = cut.split_turn.unwrap_or(cut.first_kept_entry_index);
    let history_count = entries[boundary_start..history_end]
        .iter()
        .filter_map(|e| e.as_agent_message())
        .count();
    let turn_prefix_count = cut
        .split_turn
        .map(|turn_start| {
            entries[turn_start..cut.first_kept_entry_index]
                .iter()
                .filter_map(|e| e.as_agent_message())
                .count()
        })
        .unwrap_or(0);
    if history_count == 0 && turn_prefix_count == 0 {
        return Err("Nothing to compact (no summarizable history beyond keep window)".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn compact_session(
    store: &dyn XySessionStore,
    session_id: &str,
    summary: &crate::CompactionSummaryBinding,
    settings: &CompactionSettings,
    custom_instructions: Option<&str>,
    context_window: u64,
    fixed_context: Option<&FixedRequestContext>,
    obs_parent: Option<fastrace::prelude::SpanContext>,
) -> Result<CompactionEntry, CompactionError> {
    let model = summary.model.as_ref();
    let generate_options = &summary.generate_options;
    if !settings.enabled {
        return Err("compaction disabled".into());
    }

    let entries = store.load_leaf_branch(session_id).await?;

    if entries.is_empty() {
        return Err("empty session, nothing to compact".into());
    }

    let prev_compaction = entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Compaction(comp) => Some(comp),
        _ => None,
    });
    let boundary_start = boundary_start(&entries);

    let boundary_end = entries.len();

    let estimated = estimate_from_session_entries(&entries, &EstimateOpts::default()).tokens;
    let tokens_before = if estimated > 0 {
        estimated
    } else {
        entries[boundary_start..boundary_end]
            .iter()
            .map(estimate_tokens_entry)
            .sum()
    };

    let cut = find_cut_point(
        &entries,
        boundary_start,
        boundary_end,
        effective_keep_budget(
            settings,
            context_window,
            fixed_context.map_or(0, FixedRequestContext::overhead_tokens),
        ),
    );

    let first_kept_entry = &entries[cut.first_kept_entry_index];
    let first_kept_entry_id = first_kept_entry
        .entry_id()
        .ok_or_else(|| "first kept entry has no id".to_string())?
        .to_string();

    let history_end = cut.split_turn.unwrap_or(cut.first_kept_entry_index);

    let messages_to_summarize: Vec<xylitol_plugin_sdk::message::AgentMessage> = entries
        [boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    let turn_prefix_messages: Vec<xylitol_plugin_sdk::message::AgentMessage> = cut
        .split_turn
        .map(|turn_start| {
            entries[turn_start..cut.first_kept_entry_index]
                .iter()
                .filter_map(|entry| entry.as_agent_message())
                .collect()
        })
        .unwrap_or_default();

    let mut file_ops_messages = messages_to_summarize.clone();
    file_ops_messages.extend(turn_prefix_messages.iter().cloned());
    let file_ops = extract_file_ops_from_messages(&file_ops_messages, prev_compaction);

    let previous_summary = prev_compaction.map(|c| c.summary.as_str());
    let summary = if cut.split_turn.is_some() && !turn_prefix_messages.is_empty() {
        let history_text = if messages_to_summarize.is_empty() {
            "No prior history.".to_string()
        } else {
            match generate_summary(
                &messages_to_summarize,
                model,
                settings.reserve_tokens,
                previous_summary,
                custom_instructions,
                obs_parent,
                generate_options,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("LLM history summarization failed, using fallback: {e}");
                    format!(
                        "[Compacted: {} entries, ~{tokens_before} tokens]",
                        messages_to_summarize.len()
                    )
                }
            }
        };
        let turn_prefix_text = match generate_turn_prefix_summary(
            &turn_prefix_messages,
            model,
            settings.reserve_tokens,
            obs_parent,
            generate_options,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                log::warn!("LLM turn-prefix summarization failed, using fallback: {e}");
                format!("[Turn prefix: {} entries]", turn_prefix_messages.len())
            }
        };
        format!("{history_text}\n\n---\n\n**Turn Context (split turn):**\n\n{turn_prefix_text}")
    } else {
        match generate_summary(
            &messages_to_summarize,
            model,
            settings.reserve_tokens,
            previous_summary,
            custom_instructions,
            obs_parent,
            generate_options,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                log::warn!("LLM summarization failed, using fallback: {e}");
                format!(
                    "[Compacted: {} entries, ~{tokens_before} tokens]",
                    messages_to_summarize.len()
                )
            }
        }
    };

    let (read_files, modified_files) = compute_file_lists(&file_ops);
    let summary_with_files = format!(
        "{}{}",
        summary,
        format_file_ops_xml(&read_files, &modified_files)
    );

    let now = timestamp_now();
    let entry = CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            timestamp: now,
        },
        summary: summary_with_files,
        first_kept_entry_id,
        tokens_before,
        details: Some(json!({
            "readFiles": read_files,
            "modifiedFiles": modified_files,
        })),
        from_hook: Some(false),
        policy: Some(
            xylitol_plugin_sdk::session::CompactionPolicySnapshot::current(
                context_window,
                settings.reserve_tokens,
                settings.keep_recent_tokens,
                crate::token_estimator::ESTIMATOR_VERSION,
            ),
        ),
    };

    store
        .commit_compaction(session_id, &SessionEntry::Compaction(entry.clone()))
        .await?;

    ensure_state_after_compact(store, session_id, &entries).await;

    Ok(entry)
}

fn session_cwd_from_entries(entries: &[SessionEntry]) -> Option<&str> {
    entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.cwd.as_str()),
        _ => None,
    })
}

pub async fn ensure_agent_todo_after_compact(
    store: &dyn XySessionStore,
    session_id: &str,
    entries_before: &[SessionEntry],
) {
    use xylitol_plugin_sdk::session::{build_context_entries, latest_agent_todo};

    let Some(snapshot) = latest_agent_todo(entries_before) else {
        return;
    };
    let entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("post-compact agent_todo ensure: reload failed ({e})");
            return;
        }
    };
    let cut = build_context_entries(&entries);
    if latest_agent_todo(&cut).is_some() {
        return;
    }
    if let Err(e) = store
        .append_session_entry(session_id, &snapshot.to_custom_entry_shell())
        .await
    {
        log::warn!("post-compact agent_todo persist failed: {e}");
    }
}

async fn ensure_state_after_compact(
    store: &dyn XySessionStore,
    session_id: &str,
    entries_before: &[SessionEntry],
) -> bool {
    let mut entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("post-compact state ensure: reload failed ({e})");
            return false;
        }
    };
    let mut pending = Vec::new();
    pending.extend(session_env_entry(&entries, entries_before));
    pending.extend(session_prepare_entry(entries_before));
    pending.extend(agent_todo_entry(&entries, entries_before));
    pending.extend(state_entries(&entries, entries_before));
    let mut all_ok = true;
    for entry in pending {
        match store.append_session_entry(session_id, &entry).await {
            Ok(()) => entries.push(entry),
            Err(e) => {
                log::warn!("post-compact state persist failed: {e}");
                all_ok = false;
            }
        }
    }
    all_ok
}

fn agent_todo_entry(
    entries: &[SessionEntry],
    entries_before: &[SessionEntry],
) -> Option<SessionEntry> {
    use xylitol_plugin_sdk::session::{build_context_entries, latest_agent_todo};

    let snapshot = latest_agent_todo(entries_before)?;
    let cut = build_context_entries(entries);
    if latest_agent_todo(&cut).is_some() {
        return None;
    }
    Some(snapshot.to_custom_entry_shell())
}

fn state_entries(entries: &[SessionEntry], entries_before: &[SessionEntry]) -> Vec<SessionEntry> {
    use xylitol_plugin_sdk::session::{
        CUSTOM_TYPE_TODO_CHECKPOINT, CUSTOM_TYPE_WORKFLOW, build_context_entries,
        latest_todo_checkpoint, latest_workflow,
    };

    let cut = build_context_entries(entries);
    let has_custom = |custom_type: &str| {
        cut.iter().any(|entry| {
            matches!(entry, SessionEntry::Custom(custom) if custom.custom_type == custom_type)
        })
    };

    let mut snapshots: Vec<SessionEntry> = Vec::new();
    if !has_custom(CUSTOM_TYPE_WORKFLOW)
        && let Some(workflow) = latest_workflow(entries_before)
    {
        snapshots.push(workflow.to_custom_entry_shell());
    }
    if !has_custom(CUSTOM_TYPE_TODO_CHECKPOINT)
        && let Some(checkpoint) = latest_todo_checkpoint(entries_before)
    {
        snapshots.push(checkpoint.to_custom_entry_shell());
    }
    const CUSTOM_TYPE_CONTEXT_FINGERPRINT: &str = "context-fingerprint-snapshot";
    if !has_custom(CUSTOM_TYPE_CONTEXT_FINGERPRINT)
        && let Some(snapshot) = latest_custom_entry(entries_before, CUSTOM_TYPE_CONTEXT_FINGERPRINT)
    {
        snapshots.push(snapshot);
    }
    snapshots
}

fn latest_custom_entry(entries: &[SessionEntry], custom_type: &str) -> Option<SessionEntry> {
    entries
        .iter()
        .rev()
        .find(|entry| matches!(entry, SessionEntry::Custom(custom) if custom.custom_type == custom_type))
        .cloned()
}

fn session_env_entry(
    entries: &[SessionEntry],
    entries_before_reload: &[SessionEntry],
) -> Option<SessionEntry> {
    let cut = xylitol_plugin_sdk::session::build_context_entries(entries);
    let mut history: Vec<_> = cut.iter().filter_map(|e| e.as_agent_message()).collect();
    let cwd = session_cwd_from_entries(entries)
        .or_else(|| session_cwd_from_entries(entries_before_reload))
        .unwrap_or(".");
    let snap = xylitol_plugin_sdk::session_env::snapshot_for_cwd(cwd);
    if !xylitol_plugin_sdk::session_env::ensure_session_env_in_history(&mut history, &snap) {
        return None;
    }
    history
        .last()
        .and_then(|msg| serde_json::to_value(msg).ok())
        .map(|message| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                message,
            })
        })
}

fn session_prepare_entry(entries_before_reload: &[SessionEntry]) -> Option<SessionEntry> {
    let before: Vec<_> = entries_before_reload
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect();
    let snap = xylitol_plugin_sdk::last_session_prepare(&before)?;
    let cut = xylitol_plugin_sdk::session::build_context_entries(entries_before_reload);
    let mut history: Vec<_> = cut.iter().filter_map(|e| e.as_agent_message()).collect();
    if !xylitol_plugin_sdk::ensure_session_prepare_in_history(&mut history, &snap) {
        return None;
    }
    history
        .last()
        .and_then(|msg| serde_json::to_value(msg).ok())
        .map(|message| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                message,
            })
        })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use xylitol_plugin_sdk::session::{
        CompactionEntry, CustomEntry, EntryBase, ForkPosition, SessionContext, SessionEntry,
        TodoCheckpoint, TodoCheckpointStep, TodoStatus, Workflow, WorkflowTask,
    };
    use xylitol_plugin_sdk::{XySessionError, XySessionStoreError};

    #[derive(Default)]
    struct MemStore {
        branch: Mutex<Vec<SessionEntry>>,
        appended: Mutex<Vec<SessionEntry>>,
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _: &str) -> bool {
            true
        }
        async fn load_entries(&self, _: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.branch.lock().unwrap().clone())
        }
        async fn append_session_entry(
            &self,
            _: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.appended.lock().unwrap().push(entry.clone());
            Ok(())
        }
        async fn build_session_context(
            &self,
            _: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Err(XySessionStoreError::unsupported("build_session_context"))
        }
        async fn create(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _: &str,
            _: &str,
            at_entry_id: &str,
            _: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::EntryNotFound {
                entry_id: at_entry_id.into(),
            })
        }
        fn set_leaf(&self, _: &str, _: Option<&str>) {}
        fn leaf_id(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn workflow() -> Workflow {
        Workflow {
            goal: "ship".into(),
            tasks: vec![WorkflowTask {
                id: "s1".into(),
                description: "do it".into(),
                depends_on: Vec::new(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                tools: Vec::new(),
                risk: Default::default(),
                verification: None,
                rollback: None,
                requires_human: false,
                status: Default::default(),
                output_ref: None,
                sub_workflow: None,
            }],
            resume_cursor: None,
            updated_at: 1,
        }
    }

    fn checkpoint() -> TodoCheckpoint {
        TodoCheckpoint {
            goal: Some("ship".into()),
            steps: vec![TodoCheckpointStep {
                id: "s1".into(),
                content: "do it".into(),
                status: TodoStatus::InProgress,
                verification: None,
                output_ref: None,
            }],
            last_verified_artifact: None,
            last_failure: Some("read: missing".into()),
            resume_cursor: None,
            updated_at: 1,
        }
    }

    fn custom_types(entries: &[SessionEntry]) -> Vec<String> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                SessionEntry::Custom(custom) => Some(custom.custom_type.clone()),
                _ => None,
            })
            .collect()
    }

    fn fingerprint_snapshot() -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "context-fingerprint-snapshot".into(),
            data: serde_json::json!({ "skills": {}, "tools": null, "prepare": null }),
        })
    }

    #[tokio::test]
    async fn compact_restores_every_state_custom_type() {
        let before = vec![
            workflow().to_custom_entry_shell(),
            checkpoint().to_custom_entry_shell(),
            fingerprint_snapshot(),
        ];
        let store = Arc::new(MemStore::default());
        ensure_state_after_compact(store.as_ref(), "s1", &before).await;

        let appended = store.appended.lock().unwrap().clone();
        let types = custom_types(&appended);
        for expected in [
            xylitol_plugin_sdk::session::CUSTOM_TYPE_WORKFLOW,
            xylitol_plugin_sdk::session::CUSTOM_TYPE_TODO_CHECKPOINT,
            "context-fingerprint-snapshot",
        ] {
            assert!(types.contains(&expected.to_string()), "missing {expected}");
        }
        assert_eq!(types.len(), 3, "one entry per missing state type");
    }

    #[tokio::test]
    async fn compact_retains_context_fingerprint_snapshot_payload() {
        let before = vec![fingerprint_snapshot()];
        let store = Arc::new(MemStore::default());
        ensure_state_after_compact(store.as_ref(), "s1", &before).await;
        let appended = store.appended.lock().unwrap().clone();
        let fingerprint = appended
            .iter()
            .find(|entry| {
                matches!(entry, SessionEntry::Custom(custom) if custom.custom_type == "context-fingerprint-snapshot")
            })
            .expect("fingerprint snapshot MUST be appended");
        match fingerprint {
            SessionEntry::Custom(custom) => {
                assert_eq!(custom.data["skills"], serde_json::json!({}));
            }
            other => panic!("expected fingerprint snapshot entry, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn compact_skips_state_already_in_the_context() {
        let workflow = workflow().to_custom_entry_shell();
        let before = vec![workflow.clone()];
        let store = Arc::new(MemStore::default());
        store.branch.lock().unwrap().extend([workflow]);
        ensure_state_after_compact(store.as_ref(), "s1", &before).await;
        assert_eq!(
            custom_types(&store.appended.lock().unwrap().clone()).len(),
            0,
            "state already present in the cut must not be duplicated"
        );
    }

    fn compaction_entry(first_kept: &str) -> SessionEntry {
        SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: 0,
            },
            summary: "summary".into(),
            first_kept_entry_id: first_kept.into(),
            tokens_before: 10,
            details: None,
            from_hook: None,
            policy: None,
        })
    }

    fn custom_entry(id: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "todo".into(),
            data: serde_json::json!({}),
        })
    }

    fn message_entry(id: &str, role: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::json!({"role": role, "content": "hi"}),
        })
    }

    #[test]
    fn boundary_start_is_shared_and_skips_non_message_tail() {
        let entries = vec![
            custom_entry("m1"),
            compaction_entry("m1"),
            custom_entry("tail"),
        ];
        assert_eq!(boundary_start(&entries), 0);
        assert!(
            last_context_entry_is_compaction(&entries),
            "a custom tail MUST NOT hide the trailing compaction"
        );
    }

    #[test]
    fn last_context_entry_after_compaction_is_a_message() {
        let entries = vec![compaction_entry("m1"), message_entry("m2", "user")];
        assert!(!last_context_entry_is_compaction(&entries));
    }
}
