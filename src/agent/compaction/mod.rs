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
pub mod settings;

pub use orchestrator::{CompactionOrchestrator, should_compact};
pub mod token_estimator;

pub use cut_detector::{
    CutPointResult, estimate_tokens_entry, find_cut_point, is_context_overflow,
};
pub use file_ops::{
    FileOps, compute_file_lists, extract_file_ops_from_messages, format_file_ops_xml,
};
pub use llm_summarizer::{generate_summary, serialize_conversation};
pub use settings::CompactionSettings;
pub use token_estimator::{XyUsage, calculate_context_tokens, estimate_context_tokens};

use anyhow::Result;
use serde_json::json;

use crate::core::ports::XyModel;
use crate::infra::session::manager::SessionManager;
use crate::infra::session::types::{CompactionEntry, EntryBase, SessionEntry};

/// Compact a session by summarizing old entries and writing a CompactionEntry.
pub async fn compact_session(
    mgr: &SessionManager,
    session_id: &str,
    model: &dyn XyModel,
    settings: &CompactionSettings,
) -> Result<CompactionEntry, String> {
    if !settings.enabled {
        return Err("compaction disabled".to_string());
    }

    let entries = mgr
        .load(session_id)
        .await
        .map_err(|e| format!("load session: {e}"))?;

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

    let tokens_before: u64 = entries[boundary_start..boundary_end]
        .iter()
        .map(estimate_tokens_entry)
        .sum();

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

    let messages_to_summarize: Vec<crate::core::message::AgentMessage> = entries
        [boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    let file_ops = extract_file_ops_from_messages(&messages_to_summarize, prev_compaction);

    let previous_summary = prev_compaction.map(|c| c.summary.as_str());
    let summary = match generate_summary(
        &messages_to_summarize,
        model,
        settings.reserve_tokens,
        previous_summary,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("LLM summarization failed, using fallback: {e}");
            format!(
                "[Compacted: {} entries, ~{tokens_before} tokens]",
                messages_to_summarize.len()
            )
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

    mgr.append(session_id, &SessionEntry::Compaction(entry.clone()))
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
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::Message(crate::infra::session::MessageEntry {
            base: crate::infra::session::EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: now.clone(),
            },
            message: serde_json::json!({
                "role": role,
                "parts": [{"type": "text", "text": content}]
            }),
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
        // Should cut at m3 (user message)
        assert!(!result.is_split_turn);
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
        assert!(!token_estimator::should_compact_by_reserve(
            100_000, 200_000, &s
        ));
    }

    #[test]
    fn test_should_compact_enabled_not_exceeded() {
        let s = CompactionSettings::default();
        // 50_000 tokens + 16384 reserve = 66384, under 200K
        assert!(!token_estimator::should_compact_by_reserve(
            50_000, 200_000, &s
        ));
    }

    #[test]
    fn test_should_compact_enabled_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        // 190_000 tokens + 1000 reserve = 191_000, exceeds 200K
        // threshold = 200_000 - 1000 = 199_000, context_tokens=190_000 < 199_000
        // Actually still not exceeded... let me fix the logic check
        // threshold = 200_000 - 1000 = 199_000, context_tokens=190_000 < 199_000
        // Wait the function checks context_tokens > threshold
        // context_tokens = 190_000, threshold = 199_000, so 190_000 > 199_000 is false
        // Hmm the test says "exceeded" but the math doesn't work.
        // Let me use 200_000 tokens: 200_000 > 199_000 = true
        assert!(token_estimator::should_compact_by_reserve(
            200_000, 200_000, &s
        ));
    }

    #[test]
    fn test_should_compact_exact_threshold() {
        let s = CompactionSettings::default();
        // threshold = 200_000 - 16384 = 183_616
        // 183_617 > 183_616 = true
        assert!(token_estimator::should_compact_by_reserve(
            183_617, 200_000, &s
        ));
    }

    // ── XyUsage tests ─────────────────────────────────────────────

    #[test]
    fn test_calculate_context_tokens_uses_total() {
        let usage = XyUsage {
            total_tokens: 500,
            input_tokens: 200,
            output_tokens: 300,
            cache_read_tokens: 100,
            cache_write_tokens: 50,
        };
        assert_eq!(token_estimator::calculate_context_tokens(&usage), 500);
    }

    #[test]
    fn test_calculate_context_tokens_fallback() {
        let usage = XyUsage {
            total_tokens: 0,
            input_tokens: 200,
            output_tokens: 300,
            cache_read_tokens: 100,
            cache_write_tokens: 50,
        };
        assert_eq!(token_estimator::calculate_context_tokens(&usage), 650);
    }
}
