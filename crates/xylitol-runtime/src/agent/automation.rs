use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Local};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::session::{
    AutomationFact, GoalState, RunOutcome, ScheduleState, SkipReason, TriggerSource, project_goals,
    project_schedules,
};
use xylitol_plugin_sdk::{XySessionStore, XySessionStoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomationAction {
    Run {
        trigger_id: String,
        schedule_id: String,
        prompt: String,
    },
    Skipped {
        schedule_id: String,
        reason: SkipReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingTrigger {
    trigger_id: String,
    schedule_id: String,
    prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AutomationEngineError {
    #[error("session store: {0}")]
    Store(String),
}

pub struct AutomationIndex {
    pub schedules: Vec<xylitol_plugin_sdk::session::ScheduleProjection>,
    pub goal_active: bool,
    pub goal_done: bool,
}

impl AutomationIndex {
    pub fn load(entries: &[xylitol_plugin_sdk::session::SessionEntry], branch_id: &str) -> Self {
        let goals = project_goals(entries, branch_id);
        let goal_active = goals
            .iter()
            .any(|goal| matches!(goal.state, GoalState::Open | GoalState::Running));
        Self {
            schedules: project_schedules(entries, branch_id),
            goal_active,
            goal_done: !goal_active && goals.iter().any(|goal| goal.state == GoalState::Done),
        }
    }
}

pub struct AutomationEngine {
    store: Arc<dyn XySessionStore>,
    session_id: String,
    branch_id: String,
    pending: Option<PendingTrigger>,
    started_at: Option<i64>,
    exhausted_factored: std::collections::HashSet<String>,
}

impl AutomationEngine {
    pub fn new(store: Arc<dyn XySessionStore>, session_id: impl Into<String>) -> Self {
        Self::new_at(store, session_id, Local::now())
    }

    pub fn new_at(
        store: Arc<dyn XySessionStore>,
        session_id: impl Into<String>,
        started_at: DateTime<Local>,
    ) -> Self {
        let session_id = session_id.into();
        Self {
            store,
            branch_id: session_id.clone(),
            session_id,
            pending: None,
            started_at: Some(started_at.timestamp()),
            exhausted_factored: std::collections::HashSet::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub async fn tick(
        &mut self,
        now: DateTime<Local>,
        busy: bool,
        permission_required: bool,
    ) -> Result<Vec<AutomationAction>, AutomationEngineError> {
        let entries = match self.store.load_leaf_branch(&self.session_id).await {
            Ok(entries) => entries,
            Err(XySessionStoreError::NotFound { .. }) => Vec::new(),
            Err(e) => return Err(AutomationEngineError::Store(e.to_string())),
        };
        let index = AutomationIndex::load(&entries, &self.branch_id);
        let mut actions = Vec::new();

        if let Some(pending) = self.pending.clone() {
            let state = index
                .schedules
                .iter()
                .find(|schedule| schedule.schedule_id == pending.schedule_id)
                .map(|schedule| schedule.state);
            if !matches!(state, Some(ScheduleState::Active)) {
                let reason = match state {
                    Some(ScheduleState::Paused) => SkipReason::Paused,
                    Some(ScheduleState::Exhausted) => SkipReason::Exhausted,
                    _ => SkipReason::Cancelled,
                };
                self.append(&AutomationFact::RunSkipped {
                    trigger_id: pending.trigger_id,
                    reason,
                    skipped_at: now.timestamp(),
                })
                .await?;
                self.pending = None;
                if state == Some(ScheduleState::Exhausted)
                    && self.exhausted_factored.insert(pending.schedule_id.clone())
                {
                    self.append(&AutomationFact::ScheduleStateChanged {
                        schedule_id: pending.schedule_id.clone(),
                        state: ScheduleState::Exhausted,
                        reason: Some("max_ticks exhausted".into()),
                        actor: "engine".into(),
                        at: now.timestamp(),
                    })
                    .await?;
                }
                actions.push(AutomationAction::Skipped {
                    schedule_id: pending.schedule_id,
                    reason,
                });
            }
        }

        let restart_cutoff = self.started_at.unwrap_or(now.timestamp());
        let mut paused_by_goal = std::collections::HashSet::new();
        if index.goal_done {
            for schedule in &index.schedules {
                if schedule.state != ScheduleState::Active {
                    continue;
                }
                self.append(&AutomationFact::ScheduleStateChanged {
                    schedule_id: schedule.schedule_id.clone(),
                    state: ScheduleState::Paused,
                    reason: Some("goal_done".into()),
                    actor: "engine".into(),
                    at: now.timestamp(),
                })
                .await?;
                paused_by_goal.insert(schedule.schedule_id.clone());
            }
        }

        for schedule in index.schedules {
            if schedule.state != ScheduleState::Active
                || paused_by_goal.contains(&schedule.schedule_id)
            {
                continue;
            }
            let Some(next_fire) = schedule.next_fire else {
                continue;
            };
            if next_fire > now.timestamp() {
                continue;
            }
            let trigger_id = uuid::Uuid::new_v4().to_string();
            if next_fire < restart_cutoff {
                self.append(&AutomationFact::TriggerFired {
                    trigger_id: trigger_id.clone(),
                    schedule_id: schedule.schedule_id.clone(),
                    branch_id: self.branch_id.clone(),
                    source: TriggerSource::Timer,
                    fired_at: now.timestamp(),
                })
                .await?;
                self.append(&AutomationFact::RunSkipped {
                    trigger_id,
                    reason: SkipReason::Restart,
                    skipped_at: now.timestamp(),
                })
                .await?;
                actions.push(AutomationAction::Skipped {
                    schedule_id: schedule.schedule_id,
                    reason: SkipReason::Restart,
                });
                continue;
            }
            self.append(&AutomationFact::TriggerFired {
                trigger_id: trigger_id.clone(),
                schedule_id: schedule.schedule_id.clone(),
                branch_id: self.branch_id.clone(),
                source: TriggerSource::Timer,
                fired_at: now.timestamp(),
            })
            .await?;

            if index.goal_active {
                self.append(&AutomationFact::RunSkipped {
                    trigger_id,
                    reason: SkipReason::GoalActive,
                    skipped_at: now.timestamp(),
                })
                .await?;
                actions.push(AutomationAction::Skipped {
                    schedule_id: schedule.schedule_id,
                    reason: SkipReason::GoalActive,
                });
                continue;
            }

            if permission_required {
                self.append(&AutomationFact::RunSkipped {
                    trigger_id,
                    reason: SkipReason::Permission,
                    skipped_at: now.timestamp(),
                })
                .await?;
                actions.push(AutomationAction::Skipped {
                    schedule_id: schedule.schedule_id,
                    reason: SkipReason::Permission,
                });
                continue;
            }

            if busy {
                if self.pending.is_some() {
                    self.append(&AutomationFact::RunSkipped {
                        trigger_id,
                        reason: SkipReason::Coalesced,
                        skipped_at: now.timestamp(),
                    })
                    .await?;
                    actions.push(AutomationAction::Skipped {
                        schedule_id: schedule.schedule_id,
                        reason: SkipReason::Coalesced,
                    });
                } else {
                    self.pending = Some(PendingTrigger {
                        trigger_id,
                        schedule_id: schedule.schedule_id,
                        prompt: schedule.prompt,
                    });
                }
                continue;
            }

            self.append(&AutomationFact::RunAdmitted {
                trigger_id: trigger_id.clone(),
                admitted_at: now.timestamp(),
            })
            .await?;
            actions.push(AutomationAction::Run {
                trigger_id,
                schedule_id: schedule.schedule_id,
                prompt: schedule.prompt,
            });
        }

        Ok(actions)
    }

    pub async fn admit(
        &mut self,
        now: DateTime<Local>,
        busy: bool,
        permission_required: bool,
    ) -> Result<Vec<AutomationAction>, AutomationEngineError> {
        let mut actions = self.tick(now, busy, permission_required).await?;
        if !busy && let Some(action) = self.drain_pending(now).await? {
            actions.push(action);
        }
        Ok(actions)
    }

    pub async fn drain_pending(
        &mut self,
        now: DateTime<Local>,
    ) -> Result<Option<AutomationAction>, AutomationEngineError> {
        let Some(pending) = self.pending.take() else {
            return Ok(None);
        };
        self.append(&AutomationFact::RunAdmitted {
            trigger_id: pending.trigger_id.clone(),
            admitted_at: now.timestamp(),
        })
        .await?;
        Ok(Some(AutomationAction::Run {
            trigger_id: pending.trigger_id,
            schedule_id: pending.schedule_id,
            prompt: pending.prompt,
        }))
    }

    pub async fn finish(
        &mut self,
        trigger_id: &str,
        outcome: RunOutcome,
        now: DateTime<Local>,
    ) -> Result<(), AutomationEngineError> {
        self.append(&AutomationFact::RunFinished {
            trigger_id: trigger_id.to_string(),
            outcome,
            finished_at: now.timestamp(),
        })
        .await
    }

    async fn append(&self, fact: &AutomationFact) -> Result<(), AutomationEngineError> {
        self.store
            .append_session_entry(&self.session_id, &fact.to_custom_entry())
            .await
            .map_err(|e| AutomationEngineError::Store(e.to_string()))
    }
}

pub fn spawn_timer(
    engine: Arc<Mutex<AutomationEngine>>,
    busy: Arc<AtomicBool>,
    permission_required: Arc<AtomicBool>,
    tx: tokio::sync::mpsc::UnboundedSender<AutomationAction>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = interval.tick() => {
                    let actions = {
                        let mut guard = engine.lock().await;
                        guard
                            .tick(
                                Local::now(),
                                busy.load(Ordering::SeqCst),
                                permission_required.load(Ordering::SeqCst),
                            )
                            .await
                            .unwrap_or_default()
                    };
                    for action in actions {
                        if tx.send(action).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::TimeZone;
    use serde_json::json;
    use xylitol_plugin_sdk::error::XySessionError;
    use xylitol_plugin_sdk::session::{
        CUSTOM_TYPE_AUTOMATION, CustomEntry, EntryBase, ForkPosition, SessionContext, SessionEntry,
    };

    use crate::test_support::MemoryStore;

    fn schedule_entry(schedule_id: &str, branch_id: &str, max_ticks: u64) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "schedule_defined",
                "schedule_id": schedule_id,
                "branch_id": branch_id,
                "prompt": "poll",
                "cadence": {"type": "interval", "seconds": 60},
                "max_ticks": max_ticks,
                "created_at": Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap().timestamp()
            }),
        })
    }

    fn goal_entry(goal_id: &str, branch_id: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "goal_defined",
                "goal_id": goal_id,
                "branch_id": branch_id,
                "objective": "tests pass",
                "acceptance": "cargo test",
                "max_iterations": 3,
                "created_at": 0
            }),
        })
    }

    #[tokio::test]
    async fn idle_tick_admits_run() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert!(matches!(actions.as_slice(), [AutomationAction::Run { .. }]));
        assert_eq!(engine.pending, None);
    }

    #[tokio::test]
    async fn busy_tick_queues_one_then_drains() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, true, false).await.unwrap();
        assert!(actions.is_empty());
        assert!(engine.has_pending());
        let drained = engine.drain_pending(now).await.unwrap();
        assert!(matches!(drained, Some(AutomationAction::Run { .. })));
        assert!(!engine.has_pending());
    }

    #[tokio::test]
    async fn unpersisted_session_ticks_as_empty_branch() {
        let store = Arc::new(MemoryStore::new());
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            "never-persisted",
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert!(
            actions.is_empty(),
            "unpersisted session MUST tick as an empty branch, not an error"
        );
    }

    #[tokio::test]
    async fn persisted_session_ticks_still_read_facts() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert!(
            matches!(actions.as_slice(), [AutomationAction::Run { .. }]),
            "persisted schedules must keep firing"
        );
    }

    #[tokio::test]
    async fn admit_merges_tick_and_pending() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();

        let busy_actions = engine.admit(now, true, false).await.unwrap();
        assert!(busy_actions.is_empty(), "busy admit must not run");
        assert!(engine.has_pending());

        let idle_actions = engine.admit(now, false, false).await.unwrap();
        assert!(matches!(
            idle_actions.as_slice(),
            [AutomationAction::Run { .. }]
        ));
        assert!(!engine.has_pending());
    }

    #[tokio::test]
    async fn goal_active_skips_tick() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        store
            .append_session_entry(session, &goal_entry("goal", session))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert_eq!(
            actions,
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::GoalActive
            }]
        );
    }

    #[tokio::test]
    async fn permission_required_skips_tick() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, true).await.unwrap();
        assert_eq!(
            actions,
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Permission
            }]
        );
    }

    #[tokio::test]
    async fn branch_filter_ignores_parent_schedule() {
        let store = Arc::new(MemoryStore::new());
        let session = "child";
        store
            .append_session_entry(session, &schedule_entry("parent-sched", "parent", 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn max_ticks_exhausts_schedule() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 1))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(matches!(
            engine.tick(first, false, false).await.unwrap().as_slice(),
            [AutomationAction::Run { .. }]
        ));
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        assert!(engine.tick(second, false, false).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn busy_coalesces_second_due_tick() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 3))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(engine.tick(first, true, false).await.unwrap().is_empty());
        assert!(engine.has_pending());
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        let actions = engine.tick(second, true, false).await.unwrap();
        assert_eq!(
            actions,
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Coalesced
            }]
        );
    }

    #[tokio::test]
    async fn unknown_fact_does_not_block_tick() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(
                session,
                &SessionEntry::Custom(CustomEntry {
                    base: EntryBase {
                        entry_type: "custom".into(),
                        id: "u1".into(),
                        parent_id: None,
                        timestamp: 0,
                    },
                    custom_type: CUSTOM_TYPE_AUTOMATION.into(),
                    data: json!({"kind": "future_fact"}),
                }),
            )
            .await
            .unwrap();
        store
            .append_session_entry(session, &schedule_entry("sched", session, 1))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(matches!(
            engine.tick(now, false, false).await.unwrap().as_slice(),
            [AutomationAction::Run { .. }]
        ));
    }

    fn state_entry(schedule_id: &str, state: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "schedule_state_changed",
                "schedule_id": schedule_id,
                "state": state,
                "actor": "test",
                "at": 0
            }),
        })
    }

    #[tokio::test]
    async fn restart_skips_missed_schedule() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
        assert_eq!(
            engine.tick(now, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Restart
            }]
        );
    }

    fn goal_running_entry(goal_id: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "goal_state_changed",
                "goal_id": goal_id,
                "state": "running",
                "actor": "test",
                "at": 0
            }),
        })
    }

    fn goal_done_entry(goal_id: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!({
                "kind": "goal_state_changed",
                "goal_id": goal_id,
                "state": "done",
                "actor": "test",
                "at": 0
            }),
        })
    }

    #[tokio::test]
    async fn goal_done_pauses_schedule() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        store
            .append_session_entry(session, &goal_entry("goal", session))
            .await
            .unwrap();
        store
            .append_session_entry(session, &goal_running_entry("goal"))
            .await
            .unwrap();
        store
            .append_session_entry(session, &goal_done_entry("goal"))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let actions = engine.tick(now, false, false).await.unwrap();
        assert!(actions.is_empty());
        let entries = store.load_entries(session).await.unwrap();
        let schedules = project_schedules(&entries, session);
        assert_eq!(schedules[0].state, ScheduleState::Paused);
    }

    #[tokio::test]
    async fn cancel_drops_pending_trigger() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 2))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(engine.tick(first, true, false).await.unwrap().is_empty());
        assert!(engine.has_pending());
        store
            .append_session_entry(session, &state_entry("sched", "cancelled"))
            .await
            .unwrap();
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        assert_eq!(
            engine.tick(second, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Cancelled
            }]
        );
        assert!(!engine.has_pending());
    }

    struct FailingLeafStore {
        inner: MemoryStore,
    }

    #[async_trait]
    impl XySessionStore for FailingLeafStore {
        async fn exists(&self, session_id: &str) -> bool {
            XySessionStore::exists(&self.inner, session_id).await
        }

        async fn load_entries(
            &self,
            session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            XySessionStore::load_entries(&self.inner, session_id).await
        }

        async fn load_leaf_branch(
            &self,
            _session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Err(XySessionStoreError::io(
                "load_leaf_branch",
                std::io::Error::other("store offline"),
            ))
        }

        async fn append_session_entry(
            &self,
            session_id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            XySessionStore::append_session_entry(&self.inner, session_id, entry).await
        }

        async fn build_session_context(
            &self,
            session_id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            XySessionStore::build_session_context(&self.inner, session_id).await
        }

        async fn create(
            &self,
            id: &str,
            cwd: Option<&str>,
            parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            XySessionStore::create(&self.inner, id, cwd, parent).await
        }

        async fn fork(
            &self,
            parent_id: &str,
            child_id: &str,
            at_entry_id: &str,
            position: ForkPosition,
        ) -> Result<(), XySessionError> {
            XySessionStore::fork(&self.inner, parent_id, child_id, at_entry_id, position).await
        }

        fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
            XySessionStore::set_leaf(&self.inner, session_id, entry_id);
        }

        fn leaf_id(&self, session_id: &str) -> Option<String> {
            XySessionStore::leaf_id(&self.inner, session_id)
        }
    }

    #[tokio::test]
    async fn store_error_surfaces_from_tick() {
        let store = Arc::new(FailingLeafStore {
            inner: MemoryStore::new(),
        });
        let mut engine = AutomationEngine::new_at(
            store,
            "s1",
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        let err = engine.tick(now, false, false).await.unwrap_err();
        assert!(matches!(err, AutomationEngineError::Store(_)));
    }

    #[tokio::test]
    async fn paused_schedule_skips_pending_trigger() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 3))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(engine.tick(first, true, false).await.unwrap().is_empty());
        assert!(engine.has_pending());
        store
            .append_session_entry(session, &state_entry("sched", "paused"))
            .await
            .unwrap();
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        assert_eq!(
            engine.tick(second, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Paused
            }]
        );
        assert!(!engine.has_pending());
        let facts: Vec<serde_json::Value> = store
            .load_entries(session)
            .await
            .unwrap()
            .iter()
            .filter_map(|entry| match entry {
                SessionEntry::Custom(custom) => Some(custom.data.clone()),
                _ => None,
            })
            .collect();
        assert!(!facts.iter().any(|data| {
            data.get("kind").and_then(|kind| kind.as_str()) == Some("schedule_state_changed")
                && data.get("actor").and_then(|actor| actor.as_str()) == Some("engine")
        }));
    }

    #[tokio::test]
    async fn exhausted_schedule_skips_pending_and_records_engine_fact() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 3))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(engine.tick(first, true, false).await.unwrap().is_empty());
        assert!(engine.has_pending());
        store
            .append_session_entry(session, &state_entry("sched", "exhausted"))
            .await
            .unwrap();
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        assert_eq!(
            engine.tick(second, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Exhausted
            }]
        );
        assert!(!engine.has_pending());
        let facts: Vec<serde_json::Value> = store
            .load_entries(session)
            .await
            .unwrap()
            .iter()
            .filter_map(|entry| match entry {
                SessionEntry::Custom(custom) => Some(custom.data.clone()),
                _ => None,
            })
            .collect();
        assert!(facts.iter().any(|data| {
            data.get("kind").and_then(|kind| kind.as_str()) == Some("schedule_state_changed")
                && data.get("schedule_id").and_then(|id| id.as_str()) == Some("sched")
                && data.get("state").and_then(|state| state.as_str()) == Some("exhausted")
                && data.get("reason").and_then(|reason| reason.as_str())
                    == Some("max_ticks exhausted")
                && data.get("actor").and_then(|actor| actor.as_str()) == Some("engine")
        }));
    }

    #[tokio::test]
    async fn exhausted_state_fact_deduped_across_rearming() {
        let store = Arc::new(MemoryStore::new());
        let session = "s1";
        store
            .append_session_entry(session, &schedule_entry("sched", session, 5))
            .await
            .unwrap();
        let mut engine = AutomationEngine::new_at(
            store.clone(),
            session,
            Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        );
        let first = Local.with_ymd_and_hms(2026, 1, 1, 0, 1, 0).unwrap();
        assert!(engine.tick(first, true, false).await.unwrap().is_empty());
        store
            .append_session_entry(session, &state_entry("sched", "exhausted"))
            .await
            .unwrap();
        let second = Local.with_ymd_and_hms(2026, 1, 1, 0, 2, 0).unwrap();
        assert_eq!(
            engine.tick(second, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Exhausted
            }]
        );
        store
            .append_session_entry(session, &schedule_entry("sched", session, 5))
            .await
            .unwrap();
        let third = Local.with_ymd_and_hms(2026, 1, 1, 0, 3, 0).unwrap();
        assert!(engine.tick(third, true, false).await.unwrap().is_empty());
        assert!(engine.has_pending());
        store
            .append_session_entry(session, &state_entry("sched", "exhausted"))
            .await
            .unwrap();
        let fourth = Local.with_ymd_and_hms(2026, 1, 1, 0, 4, 0).unwrap();
        assert_eq!(
            engine.tick(fourth, false, false).await.unwrap(),
            vec![AutomationAction::Skipped {
                schedule_id: "sched".into(),
                reason: SkipReason::Exhausted
            }]
        );
        let facts: Vec<serde_json::Value> = store
            .load_entries(session)
            .await
            .unwrap()
            .iter()
            .filter_map(|entry| match entry {
                SessionEntry::Custom(custom) => Some(custom.data.clone()),
                _ => None,
            })
            .collect();
        let engine_state_changes = facts
            .iter()
            .filter(|data| {
                data.get("kind").and_then(|kind| kind.as_str()) == Some("schedule_state_changed")
                    && data.get("actor").and_then(|actor| actor.as_str()) == Some("engine")
            })
            .count();
        assert_eq!(engine_state_changes, 1);
    }
}
