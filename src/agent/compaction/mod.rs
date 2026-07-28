//! Context compaction — LLM-based structured summarization + cut-point detection.
//!
//! Submodules:
//! - [`settings`] — compact
//! - [`token_estimator`] — token/heuristic counting
//! - [`cut_detector`] — cut-point detection
//! - [`file_ops`] — file operation tracking
//! - [`llm_summarizer`] — LLM-based summarization
//! - [`message_converter`] — SessionEntry → AgentMessage

pub mod cut_detector;
pub mod file_ops;
pub mod llm_summarizer;
pub mod message_converter;
pub mod orchestrator;
pub mod overflow;
pub mod settings;

pub use orchestrator::{CompactionOrchestrator, OverflowCompactOutcome, should_compact};
pub mod token_estimator;

pub use cut_detector::{
    CutPointResult, estimate_tokens_entry, find_cut_point, is_context_overflow,
};
pub use file_ops::{
    FileOps, compute_file_lists, extract_file_ops_from_messages, format_file_ops_xml,
};
pub use llm_summarizer::{generate_summary, generate_turn_prefix_summary, serialize_conversation};
pub use overflow::{
    assistant_same_model, error_message_is_context_overflow, is_context_overflow_assistant,
};
pub use settings::CompactionSettings;
pub use token_estimator::{
    EstimateOpts, calculate_context_tokens, estimate_context_tokens, estimate_from_session_entries,
};

use anyhow::Result;
use serde_json::json;

use crate::protocol::ports::{XyModel, XySessionStore};
use crate::protocol::session::{CompactionEntry, EntryBase, SessionEntry};

/// pi `prepareCompaction` gate: whether there is content worth summarizing.
///
/// Errors use pi-aligned English strings for the force path.
pub fn prepare_compaction(
    entries: &[SessionEntry],
    settings: &CompactionSettings,
) -> Result<(), String> {
    if entries.is_empty() {
        return Err("Nothing to compact (session too small)".into());
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
        settings.keep_recent_tokens,
    );
    let first_kept = &entries[cut.first_kept_entry_index];
    if first_kept.entry_id().is_none() {
        return Err("Nothing to compact (session too small)".into());
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
        return Err("Nothing to compact (session too small)".into());
    }
    Ok(())
}

/// Compact a session by summarizing old entries and writing a CompactionEntry.
pub async fn compact_session(
    store: &dyn XySessionStore,
    session_id: &str,
    model: &dyn XyModel,
    settings: &CompactionSettings,
) -> Result<CompactionEntry, String> {
    if !settings.enabled {
        return Err("compaction disabled".to_string());
    }

    let entries = store.load_entries(session_id).await?;

    if entries.is_empty() {
        return Err("empty session, nothing to compact".to_string());
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
        settings.keep_recent_tokens,
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

    let messages_to_summarize: Vec<crate::protocol::message::AgentMessage> = entries
        [boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    let turn_prefix_messages: Vec<crate::protocol::message::AgentMessage> = if cut.is_split_turn {
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

    let now = chrono::Utc::now().to_rfc3339();
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
    };

    store
        .append_session_entry(session_id, &SessionEntry::Compaction(entry.clone()))
        .await
        .map_err(|e| format!("write compaction entry: {e}"))?;

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionEntry;

    // ── Helpers for building test entries ──────────────────────────

    fn make_message_entry(id: &str, role: &str, content: &str) -> SessionEntry {
        use crate::protocol::message::AgentMessage;
        let now = chrono::Utc::now().to_rfc3339();
        let message = match role {
            "user" => serde_json::to_value(AgentMessage::user(content)).unwrap(),
            "assistant" => serde_json::to_value(AgentMessage::assistant(content)).unwrap(),
            "toolResult" => serde_json::to_value(AgentMessage::tool_result(
                format!("call-{id}"),
                "test_tool",
                vec![crate::protocol::message::AgentPart::text(content)],
                false,
            ))
            .unwrap(),
            other => serde_json::json!({
                "role": other,
                "content": [{ "type": "text", "text": content }],
                "timestamp": 0u64,
            }),
        };
        SessionEntry::Message(crate::infra::session::MessageEntry {
            base: crate::infra::session::EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: now,
            },
            message,
        })
    }

    fn make_model_change_entry(id: &str, provider: &str, model_id: &str) -> SessionEntry {
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::ModelChange(crate::infra::session::ModelChangeEntry {
            base: crate::infra::session::EntryBase {
                entry_type: "model_change".into(),
                id: id.into(),
                parent_id: None,
                timestamp: now.clone(),
            },
            provider: provider.into(),
            model_id: model_id.into(),
        })
    }

    fn make_thinking_entry(id: &str, level: &str) -> SessionEntry {
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::ThinkingLevelChange(crate::infra::session::ThinkingLevelChangeEntry {
            base: crate::infra::session::EntryBase {
                entry_type: "thinking_level_change".into(),
                id: id.into(),
                parent_id: None,
                timestamp: now.clone(),
            },
            thinking_level: level.into(),
        })
    }

    fn make_compaction_entry(id: &str, summary: &str) -> SessionEntry {
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::Compaction(crate::infra::session::CompactionEntry {
            base: crate::infra::session::EntryBase {
                entry_type: "compaction".into(),
                id: id.into(),
                parent_id: None,
                timestamp: now.clone(),
            },
            summary: summary.into(),
            first_kept_entry_id: String::new(),
            tokens_before: 0,
            details: None,
            from_hook: None,
        })
    }

    // ── find_cut_point tests ──────────────────────────────────────

    #[test]
    fn test_cut_point_empty_entries() {
        let result = find_cut_point(&[], 0, 0, 1000);
        assert_eq!(result.first_kept_entry_index, 0);
        assert!(!result.is_split_turn);
    }

    #[test]
    fn test_cut_point_no_valid_entry() {
        let entries = vec![make_model_change_entry("mc1", "openai", "gpt-4")];
        let result = find_cut_point(&entries, 0, entries.len(), 1000);
        assert_eq!(result.first_kept_entry_index, 0);
    }

    #[test]
    fn test_cut_point_single_message() {
        let entries = vec![make_message_entry("m1", "user", "hello")];
        let result = find_cut_point(&entries, 0, entries.len(), 1000);
        assert_eq!(result.first_kept_entry_index, 0);
        assert!(!result.is_split_turn);
    }

    #[test]
    fn test_cut_point_keeps_all_when_under_threshold() {
        let mut entries = Vec::new();
        for i in 0..5 {
            entries.push(make_message_entry(
                &format!("u{i}"),
                "user",
                &format!("msg {i}"),
            ));
            entries.push(make_message_entry(
                &format!("a{i}"),
                "assistant",
                &format!("resp {i}"),
            ));
        }
        // Very high keep_tokens should keep everything
        let result = find_cut_point(&entries, 0, entries.len(), 1_000_000);
        assert_eq!(result.first_kept_entry_index, 0);
    }

    #[test]
    fn test_cut_point_triggers_cut() {
        let mut entries = Vec::new();
        for i in 0..100 {
            entries.push(make_message_entry(
                &format!("u{i}"),
                "user",
                &format!("msg {i}"),
            ));
            entries.push(make_message_entry(
                &format!("a{i}"),
                "assistant",
                &format!("resp {i}"),
            ));
        }
        // Low keep_tokens should trigger a cut
        let result = find_cut_point(&entries, 0, entries.len(), 50);
        assert!(result.first_kept_entry_index > 0);
    }

    #[test]
    fn test_cut_point_with_start_index() {
        let mut entries = Vec::new();
        for i in 0..10 {
            entries.push(make_message_entry(
                &format!("u{i}"),
                "user",
                &format!("msg {i}"),
            ));
            entries.push(make_message_entry(
                &format!("a{i}"),
                "assistant",
                &format!("resp {i}"),
            ));
        }
        // Start at index 10 (skip first 10 entries)
        let result = find_cut_point(&entries, 10, entries.len(), 10);
        assert!(result.first_kept_entry_index >= 10);
    }

    #[test]
    fn test_cut_point_model_change_between_messages() {
        let entries = vec![
            make_message_entry("m1", "user", "first turn"),
            make_message_entry("m2", "assistant", "first response"),
            make_model_change_entry("mc", "openai", "gpt-4o"),
            make_message_entry("m3", "user", "second turn"),
            make_message_entry("m4", "assistant", "second response"),
        ];
        let result = find_cut_point(&entries, 3, entries.len(), 1);
        // With assistant as a valid cut, tiny keep budget lands on m4 (assistant).
        assert_eq!(result.first_kept_entry_index, 4);
        assert!(result.is_split_turn);
        assert_eq!(result.turn_start_index, 3); // m3 user
    }

    #[test]
    fn test_cut_point_preceding_non_messages() {
        let entries = vec![
            make_compaction_entry("c1", "previous summary"),
            make_message_entry("m1", "user", "first turn"),
            make_message_entry("m2", "assistant", "first response"),
            make_thinking_entry("t1", "high"),
            make_message_entry("m3", "user", "second turn"),
        ];
        let result = find_cut_point(&entries, 1, entries.len(), 1);
        // Should cut at m3 (user message) and include preceding non-messages
        assert!(!result.is_split_turn);
    }

    #[test]
    fn test_cut_point_assistant_is_valid_and_splits_turn() {
        // Large assistant mid-turn becomes the cut; user before it is turn start.
        let long = "x".repeat(400); // ~100 tokens
        let entries = vec![
            make_message_entry("u0", "user", "old"),
            make_message_entry("a0", "assistant", "old resp"),
            make_message_entry("u1", "user", "big turn"),
            make_message_entry("a1", "assistant", &long),
            make_message_entry("tr1", "toolResult", "tool out"),
            make_message_entry("a2", "assistant", &long),
        ];
        // Keep only the last assistant (~100) so cut lands at a2 (assistant).
        let result = find_cut_point(&entries, 0, entries.len(), 80);
        assert_eq!(result.first_kept_entry_index, 5); // a2
        assert!(result.is_split_turn);
        assert_eq!(result.turn_start_index, 2); // u1
        let role = match &entries[result.first_kept_entry_index] {
            SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
            _ => None,
        };
        assert_eq!(role, Some("assistant"));
    }

    #[test]
    fn test_cut_point_never_tool_result() {
        let long = "y".repeat(400);
        let entries = vec![
            make_message_entry("u0", "user", "start"),
            make_message_entry("a0", "assistant", "call tools"),
            make_message_entry("tr0", "toolResult", &long),
            make_message_entry("a1", "assistant", &long),
        ];
        let result = find_cut_point(&entries, 0, entries.len(), 50);
        let role = match &entries[result.first_kept_entry_index] {
            SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
            _ => None,
        };
        assert_ne!(role, Some("toolResult"));
        assert!(matches!(role, Some("assistant") | Some("user")));
    }

    #[test]
    fn test_cut_point_keep_budget_nearest_valid() {
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push(make_message_entry(
                &format!("u{i}"),
                "user",
                &format!("msg-{i}-{}", "z".repeat(40)),
            ));
            entries.push(make_message_entry(
                &format!("a{i}"),
                "assistant",
                &format!("resp-{i}-{}", "z".repeat(40)),
            ));
        }
        let keep = 80u64;
        let result = find_cut_point(&entries, 0, entries.len(), keep);
        let kept: u64 = entries[result.first_kept_entry_index..]
            .iter()
            .map(estimate_tokens_entry)
            .sum();
        assert!(
            kept >= keep.saturating_sub(keep / 2),
            "kept={kept} keep={keep}"
        );
        assert!(result.first_kept_entry_index > 0);
    }

    #[test]
    fn test_cut_point_user_boundary_not_split() {
        // Keep budget large enough that the nearest valid cut is the user turn-start.
        let entries = vec![
            make_message_entry("u0", "user", &"a".repeat(200)),
            make_message_entry("a0", "assistant", &"b".repeat(200)),
            make_message_entry("u1", "user", &"c".repeat(80)),
            make_message_entry("a1", "assistant", &"d".repeat(80)),
        ];
        // keep ≈ size of (u1+a1) so cut lands at u1 (turn-start), not mid-turn assistant.
        let keep: u64 = entries[2..].iter().map(estimate_tokens_entry).sum();
        let result = find_cut_point(&entries, 0, entries.len(), keep);
        assert_eq!(result.first_kept_entry_index, 2);
        assert!(!result.is_split_turn);
        assert_eq!(result.turn_start_index, -1);
    }

    // ── estimate_tokens_entry tests ───────────────────────────────

    #[test]
    fn test_estimate_tokens_entry_message() {
        let entry = make_message_entry("m1", "user", "hello world");
        let tokens = cut_detector::estimate_tokens_entry(&entry);
        assert!(tokens > 0, "expected non-zero tokens for message entry");
    }

    #[test]
    fn test_estimate_tokens_entry_compaction() {
        let entry = make_compaction_entry("c1", "summary text here");
        let tokens = cut_detector::estimate_tokens_entry(&entry);
        assert!(tokens > 0, "expected non-zero tokens for compaction entry");
    }

    #[test]
    fn test_estimate_tokens_entry_model_change() {
        let entry = make_model_change_entry("mc1", "openai", "gpt-4");
        let tokens = cut_detector::estimate_tokens_entry(&entry);
        assert_eq!(tokens, 0, "expected zero tokens for model change entry");
    }

    // ── is_context_overflow tests ─────────────────────────────────

    #[test]
    fn test_is_context_overflow_under_threshold() {
        assert!(!is_context_overflow(1000, 2000, 500));
    }

    #[test]
    fn test_is_context_overflow_at_threshold() {
        assert!(!is_context_overflow(1500, 2000, 500));
    }

    #[test]
    fn test_is_context_overflow_over_threshold() {
        assert!(is_context_overflow(1600, 2000, 500));
    }

    #[test]
    fn test_is_context_overflow_zero_window() {
        assert!(!is_context_overflow(1000, 0, 500));
    }

    // ── FileOps tests ─────────────────────────────────────────────

    #[test]
    fn test_file_ops_default_empty() {
        let ops = FileOps::default();
        assert!(ops.read.is_empty());
        assert!(ops.written.is_empty());
        assert!(ops.edited.is_empty());
    }

    #[test]
    fn test_format_file_ops_xml_empty() {
        let xml = format_file_ops_xml(&[], &[]);
        assert!(xml.is_empty());
    }

    #[test]
    fn test_format_file_ops_xml_read_only() {
        let xml = format_file_ops_xml(&["a.rs".into()], &[]);
        assert!(xml.contains("a.rs"));
        assert!(xml.contains("<read-files>"));
    }

    #[test]
    fn test_format_file_ops_xml_modified() {
        let xml = format_file_ops_xml(&[], &["b.rs".into()]);
        assert!(xml.contains("b.rs"));
        assert!(xml.contains("<modified-files>"));
    }

    #[test]
    fn test_format_file_ops_xml_both() {
        let xml = format_file_ops_xml(&["a.rs".into()], &["b.rs".into()]);
        assert!(xml.contains("a.rs"));
        assert!(xml.contains("b.rs"));
        assert!(xml.contains("<read-files>"));
        assert!(xml.contains("<modified-files>"));
    }

    // ── CompactionSettings tests ──────────────────────────────────

    #[test]
    fn test_compaction_settings_default() {
        let s = CompactionSettings::default();
        assert!(s.enabled);
        assert_eq!(s.reserve_tokens, 16384);
        assert_eq!(s.keep_recent_tokens, 20000);
    }

    #[test]
    fn test_should_compact_disabled() {
        let s = CompactionSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_compact(100_000, 200_000, &s));
    }

    #[test]
    fn test_should_compact_enabled_not_exceeded() {
        let s = CompactionSettings::default();
        // 50_000 tokens + 16384 reserve under 200K
        assert!(!should_compact(50_000, 200_000, &s));
    }

    #[test]
    fn test_should_compact_enabled_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        // threshold = 200_000 - 1000 = 199_000; 200_000 > 199_000
        assert!(should_compact(200_000, 200_000, &s));
    }

    #[test]
    fn test_should_compact_exact_threshold() {
        let s = CompactionSettings::default();
        // threshold = 200_000 - 16384 = 183_616; one over triggers
        assert!(should_compact(183_617, 200_000, &s));
    }

    #[test]
    fn test_should_compact_window_zero() {
        let s = CompactionSettings::default();
        assert!(!should_compact(100_000, 0, &s));
    }

    // ── XyUsage tests ───────────────────────────────────────────────

    #[test]
    fn test_calculate_context_tokens_uses_total() {
        let usage = crate::protocol::message::XyUsage {
            total_tokens: 500,
            input: 200,
            output: 300,
            cache_read: 100,
            cache_write: 50,
            cache_write_1h: 0,
            cost: None,
        };
        assert_eq!(token_estimator::calculate_context_tokens(&usage), 500);
    }

    #[test]
    fn test_tokens_before_prefers_session_estimate_over_len4_sum() {
        use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason, XyUsage};

        let usage = XyUsage {
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 9999,
            cost: None,
        };
        let asst = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("hi")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        });
        let now = chrono::Utc::now().to_rfc3339();
        let entries = vec![
            make_message_entry("u1", "user", "hello"),
            SessionEntry::Message(crate::infra::session::MessageEntry {
                base: crate::infra::session::EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: None,
                    timestamp: now,
                },
                message: serde_json::to_value(asst).unwrap(),
            }),
        ];
        let estimated = estimate_from_session_entries(&entries, &EstimateOpts::default());
        let len4: u64 = entries.iter().map(estimate_tokens_entry).sum();
        assert_eq!(
            estimated.provenance,
            crate::protocol::types::TokenProvenance::Api
        );
        assert!(estimated.tokens > 0);
        assert_ne!(
            estimated.tokens, len4,
            "tokens_before must not be boundary len/4 alone"
        );
    }

    #[tokio::test]
    async fn test_compact_session_split_dual_summary_merge() {
        use crate::infra::provider::{FakeProvider, ScenarioStep};
        use crate::infra::session::SessionManager;

        let long = "x".repeat(400);
        let mgr = SessionManager::in_memory();
        let sid = "split-dual";
        mgr.create(sid, Some("."), None).await.unwrap();
        for (id, role, content) in [
            ("u0", "user", "old".to_string()),
            ("a0", "assistant", "old resp".to_string()),
            ("u1", "user", "big turn".to_string()),
            ("a1", "assistant", long.clone()),
            ("tr1", "toolResult", "tool out".to_string()),
            ("a2", "assistant", long),
        ] {
            let e = make_message_entry(id, role, &content);
            mgr.append(sid, &e).await.unwrap();
        }

        let model = FakeProvider::new(
            "sum",
            vec![
                ScenarioStep::text("## Goal\nhistory-summary"),
                ScenarioStep::text("## Original Request\nturn-prefix"),
            ],
        );
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 1024,
            keep_recent_tokens: 80,
        };
        let entry = compact_session(&mgr, sid, &model, &settings)
            .await
            .expect("compact");
        assert!(
            entry.summary.contains("**Turn Context (split turn):**"),
            "summary={}",
            entry.summary
        );
        assert!(
            entry.summary.contains("history-summary")
                || entry.summary.contains("No prior history.")
        );
        assert!(entry.summary.contains("turn-prefix") || entry.summary.contains("Turn prefix"));
        assert!(entry.tokens_before > 0);
    }

    #[tokio::test]
    async fn test_generate_turn_prefix_summary_uses_prompt() {
        use crate::infra::provider::{FakeProvider, ScenarioStep};
        use crate::protocol::message::AgentMessage;

        let model = FakeProvider::new("tp", vec![ScenarioStep::text("prefix-ok")]);
        let msgs = vec![AgentMessage::user("do the thing")];
        let text = generate_turn_prefix_summary(&msgs, &model, 1024)
            .await
            .unwrap();
        assert_eq!(text, "prefix-ok");
    }
}
