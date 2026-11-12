use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use chrono::{DateTime, Duration, Local, TimeZone};
use croner::Cron;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

pub const CUSTOM_TYPE_AUTOMATION: &str = "automation";
pub const DEFAULT_MAX_TICKS: u64 = 256;
pub const DEFAULT_MAX_ITERATIONS: u64 = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cadence {
    Interval { seconds: u64 },
    Cron { expr: String },
}

impl Cadence {
    pub fn validate(&self) -> Result<(), AutomationError> {
        match self {
            Cadence::Interval { seconds } => {
                if *seconds == 0 {
                    return Err(AutomationError::InvalidInterval);
                }
            }
            Cadence::Cron { expr } => {
                if expr.split_whitespace().count() != 5 {
                    return Err(AutomationError::InvalidCron(
                        "cron must have exactly 5 fields".into(),
                    ));
                }
                Cron::from_str(expr).map_err(|e| AutomationError::InvalidCron(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn next_after(&self, now: DateTime<Local>) -> Result<DateTime<Local>, AutomationError> {
        self.validate()?;
        match self {
            Cadence::Interval { seconds } => Ok(now + Duration::seconds(*seconds as i64)),
            Cadence::Cron { expr } => {
                let cron = Cron::from_str(expr)
                    .map_err(|e| AutomationError::InvalidCron(e.to_string()))?;
                cron.find_next_occurrence(&now, false)
                    .map_err(|e| AutomationError::InvalidCron(e.to_string()))
            }
        }
    }

    pub fn next_after_unix(&self, timestamp: i64) -> Result<i64, AutomationError> {
        let now = Local
            .timestamp_opt(timestamp, 0)
            .single()
            .ok_or(AutomationError::InvalidTimestamp(timestamp))?;
        Ok(self.next_after(now)?.timestamp())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    #[default]
    Active,
    Paused,
    Exhausted,
    Cancelled,
    Failed,
}

pub fn goal_transition(from: GoalState, to: GoalState) -> Result<GoalState, String> {
    use GoalState::*;
    let allowed = matches!(
        (from, to),
        (Open, Running | Paused | Cancelled)
            | (Running, Done | Failed | Blocked | Paused | Cancelled)
            | (Blocked, Running | Done | Failed | Paused | Cancelled)
            | (Paused, Running | Cancelled)
    );
    if allowed {
        Ok(to)
    } else {
        Err(format!("illegal goal transition: {from:?} -> {to:?}"))
    }
}

pub fn schedule_transition(
    from: ScheduleState,
    to: ScheduleState,
) -> Result<ScheduleState, String> {
    use ScheduleState::*;
    let allowed = matches!(
        (from, to),
        (Active, Paused | Exhausted | Cancelled | Failed) | (Paused, Active | Cancelled)
    );
    if allowed {
        Ok(to)
    } else {
        Err(format!("illegal schedule transition: {from:?} -> {to:?}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Timer,
    Manual,
    #[serde(rename = "turn_end")]
    TurnCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Busy,
    Coalesced,
    GoalActive,
    Permission,
    Restart,
    Cancelled,
    Paused,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Success,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalState {
    #[default]
    Open,
    Running,
    Done,
    Failed,
    Blocked,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalVerdict {
    Met,
    NotMet,
    Impossible,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationFact {
    ScheduleDefined {
        schedule_id: String,
        branch_id: String,
        prompt: String,
        cadence: Cadence,
        max_ticks: u64,
        #[serde(default)]
        stop_condition: Option<String>,
        created_at: i64,
    },
    ScheduleStateChanged {
        schedule_id: String,
        state: ScheduleState,
        #[serde(default)]
        reason: Option<String>,
        actor: String,
        at: i64,
    },
    TriggerFired {
        trigger_id: String,
        schedule_id: String,
        branch_id: String,
        source: TriggerSource,
        fired_at: i64,
    },
    RunAdmitted {
        trigger_id: String,
        admitted_at: i64,
    },
    RunSkipped {
        trigger_id: String,
        reason: SkipReason,
        skipped_at: i64,
    },
    RunFinished {
        trigger_id: String,
        outcome: RunOutcome,
        finished_at: i64,
    },
    GoalDefined {
        goal_id: String,
        branch_id: String,
        objective: String,
        acceptance: String,
        max_iterations: u64,
        created_at: i64,
    },
    GoalStateChanged {
        goal_id: String,
        state: GoalState,
        #[serde(default)]
        reason: Option<String>,
        actor: String,
        at: i64,
    },
    GoalEvaluated {
        goal_id: String,
        verdict: GoalVerdict,
        evidence: String,
        iterations_used: u64,
        at: i64,
    },
}

impl AutomationFact {
    pub fn to_custom_entry(&self) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: json!(self),
        })
    }
}

pub fn parse_automation_fact(custom: &CustomEntry) -> Option<AutomationFact> {
    if custom.custom_type != CUSTOM_TYPE_AUTOMATION {
        return None;
    }
    match serde_json::from_value::<AutomationFact>(custom.data.clone()) {
        Ok(fact) => Some(fact),
        Err(error) => {
            log::warn!(
                target: "xylitol::session",
                "ignoring unknown automation fact: {error}"
            );
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleProjection {
    pub schedule_id: String,
    pub branch_id: String,
    pub prompt: String,
    pub cadence: Cadence,
    pub max_ticks: u64,
    pub stop_condition: Option<String>,
    pub state: ScheduleState,
    pub ticks_used: u64,
    pub next_fire: Option<i64>,
    pub pending_trigger: Option<String>,
    pub last_outcome: Option<RunOutcome>,
    pub last_fire_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalProjection {
    pub goal_id: String,
    pub branch_id: String,
    pub objective: String,
    pub acceptance: String,
    pub max_iterations: u64,
    pub state: GoalState,
    pub iterations_used: u64,
    pub last_verdict: Option<GoalVerdict>,
    pub last_evidence: Option<String>,
    pub consecutive_invalid: u32,
}

#[derive(Default)]
struct ScheduleBuilder {
    schedule_id: String,
    branch_id: String,
    prompt: String,
    cadence: Option<Cadence>,
    max_ticks: u64,
    stop_condition: Option<String>,
    created_at: i64,
    state: ScheduleState,
    ticks_used: u64,
    pending_trigger: Option<String>,
    last_outcome: Option<RunOutcome>,
    last_fire_at: Option<i64>,
}

impl ScheduleBuilder {
    fn into_projection(self) -> Option<ScheduleProjection> {
        let cadence = self.cadence?;
        let base = self.last_fire_at.unwrap_or(self.created_at);
        let next_fire = cadence.next_after_unix(base).ok();
        let state = if self.state == ScheduleState::Active && self.ticks_used >= self.max_ticks {
            ScheduleState::Exhausted
        } else {
            self.state
        };
        Some(ScheduleProjection {
            schedule_id: self.schedule_id,
            branch_id: self.branch_id,
            prompt: self.prompt,
            cadence,
            max_ticks: self.max_ticks,
            stop_condition: self.stop_condition,
            state,
            ticks_used: self.ticks_used,
            next_fire,
            pending_trigger: self.pending_trigger,
            last_outcome: self.last_outcome,
            last_fire_at: self.last_fire_at,
        })
    }
}

pub fn project_schedules(entries: &[SessionEntry], branch_id: &str) -> Vec<ScheduleProjection> {
    let mut builders: BTreeMap<String, ScheduleBuilder> = BTreeMap::new();
    let mut trigger_owner: HashMap<String, String> = HashMap::new();

    for entry in entries {
        let SessionEntry::Custom(custom) = entry else {
            continue;
        };
        let Some(fact) = parse_automation_fact(custom) else {
            continue;
        };
        match fact {
            AutomationFact::ScheduleDefined {
                schedule_id,
                branch_id: fact_branch,
                prompt,
                cadence,
                max_ticks,
                stop_condition,
                created_at,
                ..
            } if fact_branch == branch_id => {
                builders.insert(
                    schedule_id.clone(),
                    ScheduleBuilder {
                        schedule_id,
                        branch_id: fact_branch,
                        prompt,
                        cadence: Some(cadence),
                        max_ticks,
                        stop_condition,
                        created_at,
                        state: ScheduleState::Active,
                        ticks_used: 0,
                        pending_trigger: None,
                        last_outcome: None,
                        last_fire_at: None,
                    },
                );
            }
            AutomationFact::ScheduleStateChanged {
                schedule_id, state, ..
            } => {
                if let Some(builder) = builders.get_mut(&schedule_id) {
                    match schedule_transition(builder.state, state) {
                        Ok(next) => {
                            builder.state = next;
                            if next != ScheduleState::Active {
                                builder.pending_trigger = None;
                            }
                        }
                        Err(error) => log::warn!(
                            target: "xylitol::session",
                            "ignoring illegal schedule transition fact for schedule {schedule_id}: {error}"
                        ),
                    }
                }
            }
            AutomationFact::TriggerFired {
                trigger_id,
                schedule_id,
                branch_id: fact_branch,
                fired_at,
                ..
            } if fact_branch == branch_id => {
                trigger_owner.insert(trigger_id.clone(), schedule_id.clone());
                if let Some(builder) = builders.get_mut(&schedule_id) {
                    builder.pending_trigger = Some(trigger_id);
                    builder.last_fire_at = Some(fired_at);
                }
            }
            AutomationFact::RunAdmitted { trigger_id, .. } => {
                if let Some(schedule_id) = trigger_owner.get(&trigger_id)
                    && let Some(builder) = builders.get_mut(schedule_id)
                {
                    builder.ticks_used += 1;
                    builder.pending_trigger = None;
                }
            }
            AutomationFact::RunSkipped { trigger_id, .. } => {
                if let Some(schedule_id) = trigger_owner.get(&trigger_id)
                    && let Some(builder) = builders.get_mut(schedule_id)
                {
                    builder.pending_trigger = None;
                }
            }
            AutomationFact::RunFinished {
                trigger_id,
                outcome,
                ..
            } => {
                if let Some(schedule_id) = trigger_owner.get(&trigger_id)
                    && let Some(builder) = builders.get_mut(schedule_id)
                {
                    builder.pending_trigger = None;
                    builder.last_outcome = Some(outcome);
                }
            }
            _ => {}
        }
    }

    builders
        .into_values()
        .filter_map(ScheduleBuilder::into_projection)
        .collect()
}

#[derive(Default)]
struct GoalBuilder {
    goal_id: String,
    branch_id: String,
    objective: String,
    acceptance: String,
    max_iterations: u64,
    state: GoalState,
    iterations_used: u64,
    last_verdict: Option<GoalVerdict>,
    last_evidence: Option<String>,
    consecutive_invalid: u32,
}

pub fn project_goals(entries: &[SessionEntry], branch_id: &str) -> Vec<GoalProjection> {
    let mut builders: BTreeMap<String, GoalBuilder> = BTreeMap::new();
    for entry in entries {
        let SessionEntry::Custom(custom) = entry else {
            continue;
        };
        let Some(fact) = parse_automation_fact(custom) else {
            continue;
        };
        match fact {
            AutomationFact::GoalDefined {
                goal_id,
                branch_id: fact_branch,
                objective,
                acceptance,
                max_iterations,
                ..
            } if fact_branch == branch_id => {
                builders.insert(
                    goal_id.clone(),
                    GoalBuilder {
                        goal_id,
                        branch_id: fact_branch,
                        objective,
                        acceptance,
                        max_iterations,
                        state: GoalState::Open,
                        iterations_used: 0,
                        last_verdict: None,
                        last_evidence: None,
                        consecutive_invalid: 0,
                    },
                );
            }
            AutomationFact::GoalStateChanged { goal_id, state, .. } => {
                if let Some(builder) = builders.get_mut(&goal_id) {
                    match goal_transition(builder.state, state) {
                        Ok(next) => builder.state = next,
                        Err(error) => log::warn!(
                            target: "xylitol::session",
                            "ignoring illegal goal transition fact for goal {goal_id}: {error}"
                        ),
                    }
                }
            }
            AutomationFact::GoalEvaluated {
                goal_id,
                verdict,
                evidence,
                iterations_used,
                ..
            } => {
                if let Some(builder) = builders.get_mut(&goal_id) {
                    builder.iterations_used = iterations_used;
                    builder.last_verdict = Some(verdict);
                    builder.last_evidence = Some(evidence);
                    if verdict == GoalVerdict::Invalid {
                        builder.consecutive_invalid += 1;
                    } else {
                        builder.consecutive_invalid = 0;
                    }
                }
            }
            _ => {}
        }
    }
    builders
        .into_values()
        .map(|builder| GoalProjection {
            goal_id: builder.goal_id,
            branch_id: builder.branch_id,
            objective: builder.objective,
            acceptance: builder.acceptance,
            max_iterations: builder.max_iterations,
            state: builder.state,
            iterations_used: builder.iterations_used,
            last_verdict: builder.last_verdict,
            last_evidence: builder.last_evidence,
            consecutive_invalid: builder.consecutive_invalid,
        })
        .collect()
}

pub fn has_active_goal(entries: &[SessionEntry], branch_id: &str) -> bool {
    project_goals(entries, branch_id)
        .into_iter()
        .any(|goal| matches!(goal.state, GoalState::Open | GoalState::Running))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AutomationError {
    #[error("interval seconds must be greater than zero")]
    InvalidInterval,
    #[error("invalid cron expression: {0}")]
    InvalidCron(String),
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(i64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use serde_json::Value;

    fn custom(data: Value) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: "e1".into(),
                parent_id: None,
                timestamp: 1,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data,
        })
    }

    #[test]
    fn interval_next_after_is_exact() {
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let next = Cadence::Interval { seconds: 300 }
            .next_after(now)
            .expect("next");
        assert_eq!(next.timestamp() - now.timestamp(), 300);
    }

    #[test]
    fn cron_next_after_uses_five_fields() {
        let now = Local.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let next = Cadence::Cron {
            expr: "*/5 * * * *".into(),
        }
        .next_after(now)
        .expect("next");
        assert_eq!(next.minute(), 5);
        assert_eq!(next.hour(), 0);
    }

    #[test]
    fn cron_rejects_six_fields() {
        let cadence = Cadence::Cron {
            expr: "* * * * * *".into(),
        };
        assert!(cadence.validate().is_err());
    }

    #[test]
    fn schedule_projection_counts_admitted_and_filters_branch() {
        let entries = vec![
            custom(json!({
                "kind": "schedule_defined",
                "schedule_id": "s1",
                "branch_id": "b1",
                "prompt": "poll",
                "cadence": {"type": "interval", "seconds": 60},
                "max_ticks": 2,
                "created_at": 1
            })),
            custom(json!({
                "kind": "trigger_fired",
                "trigger_id": "t1",
                "schedule_id": "s1",
                "branch_id": "b1",
                "source": "timer",
                "fired_at": 10
            })),
            custom(json!({
                "kind": "run_admitted",
                "trigger_id": "t1",
                "admitted_at": 11
            })),
            custom(json!({
                "kind": "schedule_defined",
                "schedule_id": "s2",
                "branch_id": "b2",
                "prompt": "other",
                "cadence": {"type": "interval", "seconds": 60},
                "max_ticks": 2,
                "created_at": 1
            })),
        ];
        let projections = project_schedules(&entries, "b1");
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].schedule_id, "s1");
        assert_eq!(projections[0].ticks_used, 1);
    }

    #[test]
    fn unknown_automation_kind_is_ignored() {
        let entry = custom(json!({"kind": "future_fact", "x": 1}));
        let SessionEntry::Custom(custom) = entry else {
            panic!("custom")
        };
        assert!(parse_automation_fact(&custom).is_none());
    }

    #[test]
    fn goal_projection_tracks_state_and_invalid_count() {
        let entries = vec![
            custom(json!({
                "kind": "goal_defined",
                "goal_id": "g1",
                "branch_id": "b1",
                "objective": "tests pass",
                "acceptance": "cargo test exits 0",
                "max_iterations": 5,
                "created_at": 1
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "running",
                "actor": "test",
                "at": 2
            })),
            custom(json!({
                "kind": "goal_evaluated",
                "goal_id": "g1",
                "verdict": "invalid",
                "evidence": "bad json",
                "iterations_used": 1,
                "at": 3
            })),
            custom(json!({
                "kind": "goal_evaluated",
                "goal_id": "g1",
                "verdict": "invalid",
                "evidence": "bad json",
                "iterations_used": 2,
                "at": 4
            })),
        ];
        let goals = project_goals(&entries, "b1");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].state, GoalState::Running);
        assert_eq!(goals[0].max_iterations, 5);
        assert_eq!(goals[0].iterations_used, 2);
        assert_eq!(goals[0].consecutive_invalid, 2);
        assert!(has_active_goal(&entries, "b1"));
        assert!(!has_active_goal(&entries, "b2"));
    }

    #[test]
    fn goal_projection_and_active_check() {
        let entries = vec![custom(json!({
            "kind": "goal_defined",
            "goal_id": "g1",
            "branch_id": "b1",
            "objective": "tests pass",
            "acceptance": "cargo test exits 0",
            "max_iterations": 3,
            "created_at": 1
        }))];
        let goals = project_goals(&entries, "b1");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].state, GoalState::Open);
        assert!(has_active_goal(&entries, "b1"));
        assert!(!has_active_goal(&entries, "b2"));
    }

    #[test]
    fn goal_transition_matrix_is_closed() {
        use GoalState::*;
        let states = [Open, Running, Done, Failed, Blocked, Paused, Cancelled];
        let allowed = |from: GoalState, to: GoalState| {
            matches!(
                (from, to),
                (Open, Running | Paused | Cancelled)
                    | (Running, Done | Failed | Blocked | Paused | Cancelled)
                    | (Blocked, Running | Done | Failed | Paused | Cancelled)
                    | (Paused, Running | Cancelled)
            )
        };
        for from in states {
            for to in states {
                assert_eq!(
                    goal_transition(from, to).is_ok(),
                    allowed(from, to),
                    "goal transition {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn schedule_transition_matrix_is_closed() {
        use ScheduleState::*;
        let states = [Active, Paused, Exhausted, Cancelled, Failed];
        let allowed = |from: ScheduleState, to: ScheduleState| {
            matches!(
                (from, to),
                (Active, Paused | Exhausted | Cancelled | Failed) | (Paused, Active | Cancelled)
            )
        };
        for from in states {
            for to in states {
                assert_eq!(
                    schedule_transition(from, to).is_ok(),
                    allowed(from, to),
                    "schedule transition {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn goal_projection_keeps_last_legal_state_on_illegal_fact() {
        let entries = vec![
            custom(json!({
                "kind": "goal_defined",
                "goal_id": "g1",
                "branch_id": "b1",
                "objective": "tests pass",
                "acceptance": "cargo test exits 0",
                "max_iterations": 5,
                "created_at": 1
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "running",
                "actor": "legacy",
                "at": 2
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "done",
                "actor": "legacy",
                "at": 3
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "running",
                "actor": "legacy",
                "at": 4
            })),
        ];
        let goals = project_goals(&entries, "b1");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].state, GoalState::Done);
    }

    #[test]
    fn goal_projection_continues_legal_track_after_illegal_fact() {
        let entries = vec![
            custom(json!({
                "kind": "goal_defined",
                "goal_id": "g1",
                "branch_id": "b1",
                "objective": "tests pass",
                "acceptance": "cargo test exits 0",
                "max_iterations": 5,
                "created_at": 1
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "done",
                "actor": "legacy",
                "at": 2
            })),
            custom(json!({
                "kind": "goal_state_changed",
                "goal_id": "g1",
                "state": "running",
                "actor": "llm",
                "at": 3
            })),
        ];
        let goals = project_goals(&entries, "b1");
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].state, GoalState::Running);
    }

    #[test]
    fn schedule_projection_keeps_last_legal_state_on_illegal_fact() {
        let entries = vec![
            custom(json!({
                "kind": "schedule_defined",
                "schedule_id": "s1",
                "branch_id": "b1",
                "prompt": "poll",
                "cadence": {"type": "interval", "seconds": 60},
                "max_ticks": 4,
                "created_at": 1
            })),
            custom(json!({
                "kind": "schedule_state_changed",
                "schedule_id": "s1",
                "state": "paused",
                "actor": "legacy",
                "at": 2
            })),
            custom(json!({
                "kind": "schedule_state_changed",
                "schedule_id": "s1",
                "state": "active",
                "actor": "legacy",
                "at": 3
            })),
            custom(json!({
                "kind": "schedule_state_changed",
                "schedule_id": "s1",
                "state": "exhausted",
                "actor": "legacy",
                "at": 4
            })),
            custom(json!({
                "kind": "schedule_state_changed",
                "schedule_id": "s1",
                "state": "active",
                "actor": "legacy",
                "at": 5
            })),
        ];
        let schedules = project_schedules(&entries, "b1");
        assert_eq!(schedules.len(), 1);
        assert_eq!(schedules[0].state, ScheduleState::Exhausted);
    }
}
