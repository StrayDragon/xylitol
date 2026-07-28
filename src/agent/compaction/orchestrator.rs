//! CompactionOrchestrator — reserve-formula checks and compaction triggering.
//!
//! Trigger formula matches pi `shouldCompact` (coding-agent compaction.ts):
//! `enabled && contextTokens > contextWindow - reserveTokens`.
//!
//! Manual `compact` = force (pi `AgentSession.compact`); auto = threshold only
//! (pi `_checkCompaction` Case2). Overflow = c1660.

use crate::agent::compaction::token_estimator::{EstimateOpts, estimate_from_session_entries};
use crate::agent::compaction::{CompactionSettings, compact_session, prepare_compaction};
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason};
use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore};
use crate::protocol::session::SessionEntry;

/// Orchestrates session compaction — threshold checks and execution.
pub struct CompactionOrchestrator {
    settings: CompactionSettings,
}

impl CompactionOrchestrator {
    pub fn new(settings: CompactionSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &CompactionSettings {
        &self.settings
    }

    /// Manual force compact (pi `compact()`). Does **not** apply the reserve gate.
    ///
    /// Emits `CompactionStart { reason: "manual" }` then prepare; on prepare failure
    /// returns pi-aligned errors (`Already compacted` / `Nothing to compact …`).
    pub async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
    ) -> Result<(), String> {
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: "manual".to_string(),
            })
            .await;

        let entries = store.load_entries(sid).await?;
        if let Some(err) = prepare_compaction(&entries, &self.settings).err() {
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: None,
                    aborted: false,
                })
                .await;
            return Err(err);
        }

        // Manual always runs even if settings.enabled == false (pi).
        let mut force_settings = self.settings.clone();
        force_settings.enabled = true;

        let result = compact_session(store, sid, model, &force_settings)
            .await
            .map_err(|e| format!("compaction failed: {e}"));

        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: result.as_ref().ok().map(|_| "ok".to_string()),
                aborted: false,
            })
            .await;

        result?;
        Ok(())
    }

    /// Threshold auto-compact (pi `_checkCompaction` Case2 / `_runAutoCompaction("threshold")`).
    ///
    /// Returns `Ok(true)` if compaction ran. Prepare failure → silent `Ok(false)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn maybe_auto_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
        last_assistant: Option<&AgentMessage>,
    ) -> Result<bool, String> {
        if !self.settings.enabled {
            return Ok(false);
        }

        if assistant_is_aborted(last_assistant) {
            return Ok(false);
        }

        let entries = store.load_entries(sid).await?;

        if assistant_is_stale_vs_compaction(last_assistant, &entries) {
            return Ok(false);
        }

        let estimate = estimate_from_session_entries(&entries, estimate_opts);
        // No usable estimate → skip (pi: no lastUsageIndex).
        if estimate.tokens == 0 {
            return Ok(false);
        }

        if usage_anchor_stale_vs_compaction(&entries) {
            return Ok(false);
        }

        if !should_compact(estimate.tokens, context_window, &self.settings) {
            return Ok(false);
        }

        // Auto: prepare before Start (pi _runAutoCompaction).
        if prepare_compaction(&entries, &self.settings).is_err() {
            return Ok(false);
        }

        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: format!(
                    "threshold: {} tokens over reserve of {}k window",
                    estimate.tokens,
                    context_window / 1000,
                ),
            })
            .await;

        let result = compact_session(store, sid, model, &self.settings)
            .await
            .map_err(|e| format!("auto-compaction: {e}"));

        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: result.as_ref().ok().map(|_| "ok".to_string()),
                aborted: false,
            })
            .await;

        result?;
        Ok(true)
    }
}

/// Check if compaction should trigger (pi-aligned reserve formula).
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool {
    if !settings.enabled || context_window == 0 {
        return false;
    }
    context_tokens > context_window.saturating_sub(settings.reserve_tokens)
}

fn assistant_is_aborted(assistant: Option<&AgentMessage>) -> bool {
    matches!(
        assistant,
        Some(AgentMessage::Llm(LlmMessage::AssistantMessage {
            stop_reason: Some(XyStopReason::Aborted),
            ..
        }))
    )
}

fn assistant_timestamp_ms(assistant: &AgentMessage) -> Option<u64> {
    match assistant {
        AgentMessage::Llm(LlmMessage::AssistantMessage { timestamp, .. }) if *timestamp > 0 => {
            Some(*timestamp)
        }
        _ => None,
    }
}

fn latest_compaction_ms(entries: &[SessionEntry]) -> Option<u64> {
    entries.iter().rev().find_map(|e| match e {
        SessionEntry::Compaction(c) => parse_rfc3339_ms(&c.base.timestamp),
        _ => None,
    })
}

fn parse_rfc3339_ms(ts: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis().max(0) as u64)
}

fn assistant_is_stale_vs_compaction(
    assistant: Option<&AgentMessage>,
    entries: &[SessionEntry],
) -> bool {
    let (Some(asst), Some(comp_ms)) = (assistant, latest_compaction_ms(entries)) else {
        return false;
    };
    match assistant_timestamp_ms(asst) {
        Some(ts) => ts <= comp_ms,
        None => false,
    }
}

/// If the leaf Api usage assistant is older than the latest compaction, skip.
fn usage_anchor_stale_vs_compaction(entries: &[SessionEntry]) -> bool {
    let Some(comp_ms) = latest_compaction_ms(entries) else {
        return false;
    };
    for entry in entries.iter().rev() {
        let Some(msg) = entry.as_agent_message() else {
            continue;
        };
        if let AgentMessage::Llm(LlmMessage::AssistantMessage {
            usage: Some(_),
            timestamp,
            ..
        }) = &msg
        {
            if *timestamp > 0 {
                return *timestamp <= comp_ms;
            }
            // Fall back to entry timestamp when message timestamp is unset.
            if let Some(entry_ms) = entry.base().and_then(|b| parse_rfc3339_ms(&b.timestamp)) {
                return entry_ms <= comp_ms;
            }
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_compact_disabled() {
        let s = CompactionSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_compact(100_000, 200_000, &s));
    }

    #[test]
    fn should_compact_window_zero() {
        let s = CompactionSettings::default();
        assert!(!should_compact(100_000, 0, &s));
    }

    #[test]
    fn should_compact_not_exceeded() {
        let s = CompactionSettings::default();
        assert!(!should_compact(50_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        assert!(should_compact(200_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exact_boundary_not_trigger() {
        let s = CompactionSettings::default();
        assert!(!should_compact(183_616, 200_000, &s));
    }

    #[test]
    fn should_compact_one_over_boundary() {
        let s = CompactionSettings::default();
        assert!(should_compact(183_617, 200_000, &s));
    }

    #[test]
    fn should_compact_pi_examples() {
        let s = CompactionSettings {
            enabled: true,
            reserve_tokens: 10_000,
            keep_recent_tokens: 20_000,
        };
        assert!(should_compact(95_000, 100_000, &s));
        assert!(!should_compact(89_000, 100_000, &s));
        assert!(!should_compact(
            95_000,
            100_000,
            &CompactionSettings {
                enabled: false,
                ..s
            }
        ));
    }

    #[test]
    fn aborted_assistant_detected() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("x")],
            stop_reason: Some(XyStopReason::Aborted),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        });
        assert!(assistant_is_aborted(Some(&msg)));
    }

    #[test]
    fn prepare_already_compacted() {
        use crate::protocol::session::{CompactionEntry, EntryBase};
        let entries = vec![SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
            summary: "s".into(),
            first_kept_entry_id: "x".into(),
            tokens_before: 1,
            details: None,
            from_hook: None,
        })];
        let err =
            crate::agent::compaction::prepare_compaction(&entries, &CompactionSettings::default())
                .unwrap_err();
        assert_eq!(err, "Already compacted");
    }

    #[test]
    fn stale_assistant_vs_compaction() {
        use crate::protocol::session::{CompactionEntry, EntryBase};
        let comp_ms = 2_000u64;
        let comp_ts = chrono::DateTime::from_timestamp_millis(comp_ms as i64)
            .unwrap()
            .to_rfc3339();
        let entries = vec![SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: comp_ts,
            },
            summary: "s".into(),
            first_kept_entry_id: "x".into(),
            tokens_before: 1,
            details: None,
            from_hook: None,
        })];
        let asst = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("old")],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 1_000, // before compaction
            diagnostics: Vec::new(),
        });
        assert!(assistant_is_stale_vs_compaction(Some(&asst), &entries));
        let fresh = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("new")],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 3_000,
            diagnostics: Vec::new(),
        });
        assert!(!assistant_is_stale_vs_compaction(Some(&fresh), &entries));
    }
}
