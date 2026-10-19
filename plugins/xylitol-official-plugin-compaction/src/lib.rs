//! Context compaction — LLM-based structured summarization + cut-point detection.
//!
//! Submodules:
//! - [`settings`] — compact
//! - [`token_estimator`] — token/heuristic counting
//! - [`cut_detector`] — cut-point detection
//! - [`file_ops`] — file operation tracking
//! - [`llm_summarizer`] — LLM-based summarization

pub mod binding;
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

pub use cut_detector::{estimate_tokens_entry, find_cut_point};
pub use file_ops::{compute_file_lists, extract_file_ops_from_messages, format_file_ops_xml};
pub use llm_summarizer::{generate_summary, generate_turn_prefix_summary};
pub use overflow::error_message_is_context_overflow;
pub use settings::CompactionSettings;
// Test-support re-exports (in-crate tests import via this facade).
pub use cut_detector::estimate_tokens_entry_for_cut;
pub use file_ops::FileOps;
pub use orchestrator::should_compact;
pub use overflow::{assistant_same_model, is_context_overflow_assistant};
pub use settlement::{
    ContextTokenSettlement, ContextTokenSettlementReason, settle_from_session_entries,
};
pub use token_estimator::{EstimateOpts, FixedRequestContext, estimate_from_session_entries};

use anyhow::Result;
use serde_json::json;

use xylitol_plugin_sdk::error::{XyError, XySessionError, XySessionStoreError};
use xylitol_plugin_sdk::ports::XySessionStore;
use xylitol_plugin_sdk::session::{CompactionEntry, EntryBase, MessageEntry, SessionEntry};

/// Compaction failures: store IO vs product-copy policy gates.
#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error(transparent)]
    Store(#[from] XySessionStoreError),
    #[error(transparent)]
    Session(#[from] XySessionError),
    /// Model construction / selection failed; keeps the typed `XyError` (kind/source)
    /// instead of being flattened into a `Policy` string (c2270 family).
    #[error(transparent)]
    Model(#[from] XyError),
    #[error("{0}")]
    Policy(String),
}

impl CompactionError {
    pub fn policy(message: impl Into<String>) -> Self {
        Self::Policy(message.into())
    }
}

impl From<String> for CompactionError {
    fn from(message: String) -> Self {
        Self::Policy(message)
    }
}

impl From<&str> for CompactionError {
    fn from(message: &str) -> Self {
        Self::Policy(message.to_string())
    }
}

/// Unix-ms timestamp for new compaction entries persisted to session storage.
fn timestamp_now() -> u64 {
    xylitol_plugin_sdk::message::now_ms()
}

/// Keep-window budget coordinated with the reserve gate (c8 / c25).
///
/// `min(keep_recent_tokens, window − reserve − fixed overhead)`; degenerates to
/// `keep_recent_tokens` when the window is unknown (0) or overhead not injected.
/// Without this clamp a keep budget ≥ the threshold makes every cut land on the
/// first valid entry (keep-everything) and compaction stops shrinking context.
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

/// Summary output budget assumed for the c2 floor when the leaf has no prior
/// CompactionEntry to measure from. ≈2k tokens of structured summary.
const SUMMARY_PLACEHOLDER_TOKENS: u64 = 2_048;

/// `/3` estimate of the latest summary on the leaf (c2 summary placeholder);
/// falls back to `SUMMARY_PLACEHOLDER_TOKENS` when there is no CompactionEntry
/// or the summary is empty.
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

/// Post-compaction floor projection (c2): fixed request overhead (c16 folding) +
/// effective keep tail (c8 clamp) + summary placeholder — what the next main
/// request will weigh right after a compact. The summary request itself carries
/// no tool schemas; this floor models the *conversation* request.
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

/// Effective threshold-auto trigger threshold (c2):
/// `max(window − reserve, floor + floor/4)`. `floor == 0` (overhead unknown /
/// not injected) degenerates to the pi reserve formula `window − reserve`, the
/// same degeneration discipline as the c8 clamp.
pub fn effective_trigger_threshold(
    context_window: u64,
    settings: &CompactionSettings,
    post_compact_floor: u64,
) -> u64 {
    context_window
        .saturating_sub(settings.reserve_tokens)
        .max(post_compact_floor + post_compact_floor / 4)
}

/// pi `prepareCompaction` gate: whether there is content worth summarizing.
///
/// Force-path error strings: `Already compacted` stays pi-aligned; empty /
/// no-summarizable-history strings diverge from pi `session too small`
/// (c1875 / `PI_DELTAS` A12) so footer-% misunderstandings are harder.
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

/// Compact a session by summarizing old entries and writing a CompactionEntry.
///
/// `custom_instructions` is only for the force/manual path (pi `customInstructions`);
/// auto callers MUST pass `None`.
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

    // Prefer session-context estimate (pi tokensBefore via buildSessionContext);
    // fall back to boundary heuristic when estimate cannot parse messages.
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

    // c1906: after cut, leaf context may lack session_env — ensure + persist.
    ensure_session_env_after_compact(store, session_id, &entries).await;
    // c1955: if cut dropped every agent_todo snapshot, re-append latest.
    ensure_agent_todo_after_compact(store, session_id, &entries).await;

    Ok(entry)
}

fn session_cwd_from_entries(entries: &[SessionEntry]) -> Option<&str> {
    entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.cwd.as_str()),
        _ => None,
    })
}

/// If compaction/cut leaves no `agent_todo` in the post-cut context window,
/// re-append the latest pre-cut snapshot so tip / resume / request-time inject stay aligned.
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
