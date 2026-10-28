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

pub fn prepare_compaction(
    entries: &[SessionEntry],
    settings: &CompactionSettings,
    context_window: u64,
    fixed_overhead_tokens: u64,
) -> Result<(), CompactionError> {
    if entries.is_empty() {
        return Err("Nothing to compact (empty session)".into());
    }
    if matches!(entries.last(), Some(SessionEntry::Compaction(_))) {
        return Err("Already compacted".into());
    }

    let mut boundary_start = 0usize;
    for (i, entry) in entries.iter().enumerate().rev() {
        if let SessionEntry::Compaction(comp) = entry {
            let first_kept_id = comp.first_kept_entry_id.as_str();
            if let Some(idx) = entries
                .iter()
                .position(|e| e.entry_id() == Some(first_kept_id))
            {
                boundary_start = idx;
            } else {
                boundary_start = i + 1;
            }
            break;
        }
    }
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
    let history_end = if cut.is_split_turn {
        cut.turn_start_index.max(0) as usize
    } else {
        cut.first_kept_entry_index
    };
    let history_count = entries[boundary_start..history_end]
        .iter()
        .filter_map(|e| e.as_agent_message())
        .count();
    let turn_prefix_count = if cut.is_split_turn {
        let turn_start = cut.turn_start_index.max(0) as usize;
        entries[turn_start..cut.first_kept_entry_index]
            .iter()
            .filter_map(|e| e.as_agent_message())
            .count()
    } else {
        0
    };
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

    let mut prev_compaction: Option<&CompactionEntry> = None;
    let mut boundary_start = 0usize;
    for (i, entry) in entries.iter().enumerate().rev() {
        if let SessionEntry::Compaction(comp) = entry {
            prev_compaction = Some(comp);
            let first_kept_id = comp.first_kept_entry_id.as_str();
            if let Some(idx) = entries
                .iter()
                .position(|e| e.entry_id() == Some(first_kept_id))
            {
                boundary_start = idx;
            } else {
                boundary_start = i + 1;
            }
            break;
        }
    }

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

    let history_end = if cut.is_split_turn {
        cut.turn_start_index.max(0) as usize
    } else {
        cut.first_kept_entry_index
    };

    let messages_to_summarize: Vec<xylitol_plugin_sdk::message::AgentMessage> = entries
        [boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    let turn_prefix_messages: Vec<xylitol_plugin_sdk::message::AgentMessage> = if cut.is_split_turn
    {
        let turn_start = cut.turn_start_index.max(0) as usize;
        entries[turn_start..cut.first_kept_entry_index]
            .iter()
            .filter_map(|entry| entry.as_agent_message())
            .collect()
    } else {
        Vec::new()
    };

    let mut file_ops_messages = messages_to_summarize.clone();
    file_ops_messages.extend(turn_prefix_messages.iter().cloned());
    let file_ops = extract_file_ops_from_messages(&file_ops_messages, prev_compaction);

    let previous_summary = prev_compaction.map(|c| c.summary.as_str());
    let summary = if cut.is_split_turn && !turn_prefix_messages.is_empty() {
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

    ensure_session_env_after_compact(store, session_id, &entries).await;
    ensure_session_prepare_after_compact(store, session_id, &entries).await;
    ensure_agent_todo_after_compact(store, session_id, &entries).await;
    ensure_state_entries_after_compact(store, session_id, &entries).await;

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

async fn ensure_state_entries_after_compact(
    store: &dyn XySessionStore,
    session_id: &str,
    entries_before: &[SessionEntry],
) {
    use std::collections::HashSet;

    use xylitol_plugin_sdk::session::{
        CUSTOM_TYPE_PLAN_DOC, CUSTOM_TYPE_PLAN_MODE, CUSTOM_TYPE_PLAN_REF,
        CUSTOM_TYPE_TODO_CHECKPOINT, CUSTOM_TYPE_WORKFLOW, PlanMode, build_context_entries,
        latest_plan_ref, latest_todo_checkpoint, latest_workflow, plan_docs,
    };

    let entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("post-compact state ensure: reload failed ({e})");
            return;
        }
    };
    let cut = build_context_entries(&entries);
    let has_custom = |custom_type: &str| {
        cut.iter().any(|entry| {
            matches!(entry, SessionEntry::Custom(custom) if custom.custom_type == custom_type)
        })
    };

    let mut snapshots: Vec<SessionEntry> = Vec::new();
    if !has_custom(CUSTOM_TYPE_PLAN_MODE) {
        let mode = entries_before.iter().rev().find_map(|entry| match entry {
            SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_MODE => {
                PlanMode::from_data_value(&custom.data)
            }
            _ => None,
        });
        if let Some(mode) = mode {
            snapshots.push(mode.to_custom_entry_shell());
        }
    }
    if !has_custom(CUSTOM_TYPE_PLAN_REF)
        && let Some(plan_ref) = latest_plan_ref(entries_before)
    {
        snapshots.push(plan_ref.to_custom_entry_shell());
    }
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

    let mut seen = HashSet::new();
    for doc in plan_docs(entries_before).into_iter().rev() {
        if !seen.insert(doc.plan_id.clone()) {
            continue;
        }
        let present = cut.iter().any(|entry| {
            matches!(
                entry,
                SessionEntry::Custom(custom)
                    if custom.custom_type == CUSTOM_TYPE_PLAN_DOC
                        && custom.data.get("plan_id").and_then(|id| id.as_str())
                            == Some(doc.plan_id.as_str())
            )
        });
        if !present {
            snapshots.push(doc.to_custom_entry_shell());
        }
    }

    for snapshot in snapshots {
        if let Err(e) = store.append_session_entry(session_id, &snapshot).await {
            log::warn!("post-compact state persist failed: {e}");
        }
    }
}

async fn ensure_session_env_after_compact(
    store: &dyn XySessionStore,
    session_id: &str,
    entries_before_reload: &[SessionEntry],
) {
    let entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("post-compact session_env ensure: reload failed ({e})");
            return;
        }
    };
    let cut = xylitol_plugin_sdk::session::build_context_entries(&entries);
    let mut history: Vec<_> = cut.iter().filter_map(|e| e.as_agent_message()).collect();
    let cwd = session_cwd_from_entries(&entries)
        .or_else(|| session_cwd_from_entries(entries_before_reload))
        .unwrap_or(".");
    let snap = xylitol_plugin_sdk::session_env::snapshot_for_cwd(cwd);
    if !xylitol_plugin_sdk::session_env::ensure_session_env_in_history(&mut history, &snap) {
        return;
    }
    let Some(msg) = history.last() else {
        return;
    };
    let Ok(message) = serde_json::to_value(msg) else {
        return;
    };
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message,
    });
    if let Err(e) = store.append_session_entry(session_id, &entry).await {
        log::warn!("post-compact session_env persist failed: {e}");
    }
}

async fn ensure_session_prepare_after_compact(
    store: &dyn XySessionStore,
    session_id: &str,
    entries_before_reload: &[SessionEntry],
) {
    let before: Vec<_> = entries_before_reload
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect();
    let Some(snap) = xylitol_plugin_sdk::last_session_prepare(&before) else {
        return;
    };
    let entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("post-compact session_prepare ensure: reload failed ({e})");
            return;
        }
    };
    let cut = xylitol_plugin_sdk::session::build_context_entries(&entries);
    let mut history: Vec<_> = cut.iter().filter_map(|e| e.as_agent_message()).collect();
    if !xylitol_plugin_sdk::ensure_session_prepare_in_history(&mut history, &snap) {
        return;
    }
    let Some(msg) = history.last() else {
        return;
    };
    let Ok(message) = serde_json::to_value(msg) else {
        return;
    };
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message,
    });
    if let Err(e) = store.append_session_entry(session_id, &entry).await {
        log::warn!("post-compact session_prepare persist failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use xylitol_plugin_sdk::session::{
        ForkPosition, PlanDoc, PlanDocStatus, PlanDocStep, PlanMode, PlanRef, PlanScope,
        SessionContext, SessionEntry, TodoCheckpoint, TodoCheckpointStep, TodoStatus, Workflow,
        WorkflowStep,
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

    fn doc(plan_id: &str, updated_at: u64) -> PlanDoc {
        PlanDoc {
            plan_id: plan_id.into(),
            title: format!("Plan {plan_id}"),
            status: PlanDocStatus::Draft,
            scope: PlanScope::Session,
            steps: vec![PlanDocStep {
                id: "s1".into(),
                content: "do it".into(),
                depends_on: Vec::new(),
                verification: None,
                status: TodoStatus::Pending,
            }],
            created_at: 1,
            updated_at,
        }
    }

    fn workflow() -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps: vec![WorkflowStep {
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

    #[tokio::test]
    async fn compact_restores_every_state_custom_type() {
        let before = vec![
            PlanMode::plan(Some("p1".into())).to_custom_entry_shell(),
            PlanRef {
                scope: PlanScope::Session,
                plan_id: "p1".into(),
                path: None,
            }
            .to_custom_entry_shell(),
            workflow().to_custom_entry_shell(),
            checkpoint().to_custom_entry_shell(),
            doc("p1", 1).to_custom_entry_shell(),
        ];
        let store = Arc::new(MemStore::default());
        ensure_state_entries_after_compact(store.as_ref(), "s1", &before).await;

        let appended = store.appended.lock().unwrap().clone();
        let types = custom_types(&appended);
        for expected in [
            xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_MODE,
            xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_REF,
            xylitol_plugin_sdk::session::CUSTOM_TYPE_WORKFLOW,
            xylitol_plugin_sdk::session::CUSTOM_TYPE_TODO_CHECKPOINT,
            xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_DOC,
        ] {
            assert!(types.contains(&expected.to_string()), "missing {expected}");
        }
        assert_eq!(appended.len(), 5);
    }

    #[tokio::test]
    async fn compact_skips_state_already_in_the_context() {
        let plan_mode = PlanMode::plan(Some("p1".into())).to_custom_entry_shell();
        let workflow = workflow().to_custom_entry_shell();
        let before = vec![plan_mode.clone(), workflow.clone()];
        let store = Arc::new(MemStore::default());
        store.branch.lock().unwrap().extend([plan_mode, workflow]);
        ensure_state_entries_after_compact(store.as_ref(), "s1", &before).await;
        assert!(
            store.appended.lock().unwrap().is_empty(),
            "state already present in the cut must not be duplicated"
        );
    }

    #[tokio::test]
    async fn compact_keeps_only_the_latest_plan_doc_per_id() {
        let before = vec![
            doc("p1", 1).to_custom_entry_shell(),
            doc("p1", 2).to_custom_entry_shell(),
            doc("p2", 3).to_custom_entry_shell(),
        ];
        let store = Arc::new(MemStore::default());
        ensure_state_entries_after_compact(store.as_ref(), "s1", &before).await;

        let appended = store.appended.lock().unwrap().clone();
        let docs: Vec<_> = appended
            .iter()
            .filter_map(|entry| match entry {
                SessionEntry::Custom(custom)
                    if custom.custom_type == xylitol_plugin_sdk::session::CUSTOM_TYPE_PLAN_DOC =>
                {
                    Some(custom.data.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(docs.len(), 2, "one snapshot per plan_id");
        let p1 = docs
            .iter()
            .find(|data| data["plan_id"] == "p1")
            .expect("p1");
        assert_eq!(p1["updated_at"], 2, "the latest p1 must win");
    }
}
