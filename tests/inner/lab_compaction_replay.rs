//! Lab replay harness — compaction pipeline invariants on a real session (c2810).
//!
//! 人跑维护 lab（`#[ignore]`，不进 `just qa`）：以真实 session JSONL 为 fixture
//! 回放压缩管线，断言——每次 compact 后投影单调下降、切点合法（c8：firstKept
//! 不落 toolResult）、摘要规模有界、固定新内容下 compact 次数有界（地板感知
//! 阈值的防回归实证）。fixture 不入库：env 指向用户本地文件，仓库零用户数据。
//!
//! Run:
//! ```text
//! XYLITOL_COMPACTION_REPLAY_FIXTURE=~/.xylitol/sessions/<id>.jsonl \
//!   cargo test -p xylitol --lib lab_compaction_replay -- --ignored --nocapture
//! ```

use std::sync::{Arc, Mutex};

use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::AgentMessage;
use crate::protocol::ports::{XyEventSink, XySessionStore};
use crate::protocol::session::SessionEntry;

const WINDOW: u64 = 32_768;
const RESERVE: u64 = 16_384;
const KEEP_RECENT: u64 = 20_000;
/// Synthetic turns appended between replay rounds (≈200 tokens each).
const APPEND_TURNS: usize = 120;

struct RecordingSink(Mutex<Vec<XyEvent>>);

#[async_trait::async_trait]
impl XyEventSink for RecordingSink {
    async fn emit(&self, event: &XyEvent) {
        self.0.lock().unwrap().push(event.clone());
    }
}

fn load_fixture_entries(path: &str) -> Result<Vec<SessionEntry>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"))?;
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<SessionEntry>(line)
                .map_err(|e| format!("fixture line {}: {e}", i + 1))
        })
        .collect()
}

/// c8: a firstKept target MUST be a leaf entry and MUST NOT be a toolResult.
fn first_kept_is_legal(entries: &[SessionEntry], first_kept_id: &str) -> bool {
    entries.iter().any(|e| {
        e.base().map(|b| b.id.as_str()) == Some(first_kept_id)
            && e.as_agent_message()
                .map(|m| m.role_name() != "toolResult")
                .unwrap_or(true)
    })
}

/// Replay the compaction pipeline over a real session and assert its invariants.
#[tokio::test(flavor = "current_thread")]
#[ignore = "lab: needs XYLITOL_COMPACTION_REPLAY_FIXTURE pointing at a local session JSONL"]
async fn lab_compaction_replay_floor_invariants() {
    let Some(fixture) = std::env::var("XYLITOL_COMPACTION_REPLAY_FIXTURE").ok() else {
        eprintln!("skip: set XYLITOL_COMPACTION_REPLAY_FIXTURE to a session JSONL");
        return;
    };
    let entries = load_fixture_entries(&fixture).expect("fixture loads");
    assert!(!entries.is_empty(), "fixture must contain entries");

    let mgr = crate::infra::session::SessionManager::in_memory();
    let sid = "lab-replay";
    mgr.create(sid, Some("."), None).await.unwrap();
    for entry in &entries {
        mgr.append(sid, entry).await.expect("append fixture entry");
    }

    let sink = Arc::new(RecordingSink(Mutex::new(Vec::new())));
    let orch = crate::agent::compaction::CompactionOrchestrator::new(
        crate::agent::compaction::CompactionSettings {
            enabled: true,
            reserve_tokens: RESERVE,
            keep_recent_tokens: KEEP_RECENT,
            ..Default::default()
        },
    );
    let fixed = crate::agent::compaction::FixedRequestContext {
        system_prompt: Some("S".repeat(36_000)), // ≈9k tokens, measured shape
        tool_schemas: Vec::new(),
    };
    let opts = crate::agent::compaction::EstimateOpts {
        fixed_context: Some(fixed.clone()),
        ..Default::default()
    };

    // Round 0: force compact the real history.
    let model = crate::infra::provider::fake_xy_model(
        "lab-replay",
        vec![crate::infra::provider::ScenarioStep::text(
            "## Goal\nreplayed summary",
        )],
    );
    let mut fallback_notice = false;
    orch.compact(
        &mgr,
        sid,
        &crate::agent::model::task_model::CompactionSummaryBinding::for_test(model, "lab-replay"),
        sink.as_ref(),
        None,
        WINDOW,
        Some(&fixed),
        &mut fallback_notice,
    )
    .await
    .expect("replay force compact");

    let leaf = mgr.load_leaf_branch(sid).await.unwrap();
    let compaction = leaf
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("replay wrote a CompactionEntry");

    // c8: the cut lands on a legal entry, never a toolResult.
    assert!(
        first_kept_is_legal(&leaf, &compaction.first_kept_entry_id),
        "firstKept must be a legal cut target: {}",
        compaction.first_kept_entry_id
    );
    // Summary bounded: heuristic ≈ placeholder scale, not runaway.
    assert!(
        compaction.summary.len() / 4 <= 8 * 2_048,
        "summary must stay bounded: {} chars",
        compaction.summary.len()
    );

    // Monotone: the AfterCompaction projection sits below the pre-compact size.
    let after_events = sink.0.lock().unwrap().clone();
    let settled = after_events
        .iter()
        .filter_map(|e| match e {
            XyEvent::ContextTokenSettlement {
                estimate, reason, ..
            } if reason == "after_compaction" => Some(estimate.tokens),
            _ => None,
        })
        .next()
        .expect("AfterCompaction settlement");
    assert!(
        settled < compaction.tokens_before,
        "post-compact projection {settled} must be below tokensBefore {}",
        compaction.tokens_before
    );

    // Convergence: APPEND_TURNS of fixed synthetic content must compact a bounded
    // number of times (floor-aware threshold), not once per turn-end.
    let mut flag = false;
    let mut fallback_notice = false;
    let mut rounds = 0usize;
    for i in 0..APPEND_TURNS {
        for (id, role, body) in [
            (format!("ru{i}"), "user", "x".repeat(400)),
            (format!("ra{i}"), "assistant", "y".repeat(400)),
        ] {
            let msg = if role == "user" {
                AgentMessage::user(body)
            } else {
                AgentMessage::assistant(body)
            };
            mgr.append(
                sid,
                &SessionEntry::Message(crate::protocol::session::MessageEntry {
                    base: crate::protocol::session::EntryBase {
                        entry_type: "message".into(),
                        id,
                        parent_id: None,
                        timestamp: 0,
                    },
                    message: serde_json::to_value(&msg).unwrap(),
                }),
            )
            .await
            .unwrap();
        }
        let model = crate::infra::provider::fake_xy_model(
            "lab-replay",
            vec![crate::infra::provider::ScenarioStep::text(
                "## Goal\nreplayed summary",
            )],
        );
        if orch
            .maybe_auto_compact(
                &mgr,
                sid,
                &crate::agent::model::task_model::CompactionSummaryBinding::for_test(
                    model,
                    "lab-replay",
                ),
                sink.as_ref(),
                WINDOW,
                &opts,
                None,
                None,
                Some(&fixed),
                None,
                &mut fallback_notice,
                Some(&mut flag),
            )
            .await
            .expect("replay auto compact")
        {
            rounds += 1;
        }
    }
    // Hysteresis band ≈ floor/4 ⇒ expected cycles ≈ APPEND_TURNS×200 / 4_600 ≪ bound.
    assert!(
        rounds <= APPEND_TURNS / 8,
        "floor threshold must bound churn: {rounds} compacts for {APPEND_TURNS} turns"
    );
    println!(
        "lab_compaction_replay: tokensBefore={} settled={settled} auto-rounds={rounds}/{APPEND_TURNS}",
        compaction.tokens_before
    );
}
