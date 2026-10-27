use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

pub const CUSTOM_TYPE_PLAN: &str = "plan";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanRisk {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub risk: PlanRisk,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<String>,
    #[serde(default)]
    pub requires_human: bool,
    #[serde(default)]
    pub status: PlanStepStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Plan {
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub steps: Vec<PlanStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanValidationError(pub String);

impl std::fmt::Display for PlanValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlanValidationError {}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn all_completed(&self) -> bool {
        !self.steps.is_empty()
            && self
                .steps
                .iter()
                .all(|step| step.status == PlanStepStatus::Completed)
    }

    pub fn completed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == PlanStepStatus::Completed)
            .count()
    }

    pub fn step(&self, id: &str) -> Option<&PlanStep> {
        self.steps.iter().find(|step| step.id == id)
    }

    pub fn to_data_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn from_data_value(data: &Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }

    pub fn to_custom_entry_shell(&self) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_PLAN.into(),
            data: self.to_data_value(),
        })
    }
}

pub fn latest_plan(entries: &[SessionEntry]) -> Option<Plan> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN => {
            Plan::from_data_value(&custom.data)
        }
        _ => None,
    })
}

pub fn validate_plan(plan: &Plan) -> Result<(), PlanValidationError> {
    if plan.steps.is_empty() {
        return Err(PlanValidationError(
            "plan must have at least one step".into(),
        ));
    }
    let mut ids = HashSet::new();
    for step in &plan.steps {
        if step.id.trim().is_empty() {
            return Err(PlanValidationError("plan step id must be non-empty".into()));
        }
        if !ids.insert(step.id.clone()) {
            return Err(PlanValidationError(format!(
                "duplicate plan step id: {}",
                step.id
            )));
        }
    }
    for step in &plan.steps {
        for dependency in &step.depends_on {
            if dependency == &step.id {
                return Err(PlanValidationError(format!(
                    "plan step {} depends on itself",
                    step.id
                )));
            }
            if !ids.contains(dependency) {
                return Err(PlanValidationError(format!(
                    "plan step {} depends on unknown step {}",
                    step.id, dependency
                )));
            }
        }
    }
    detect_cycle(plan)?;
    Ok(())
}

fn detect_cycle(plan: &Plan) -> Result<(), PlanValidationError> {
    fn visit(
        id: &str,
        plan: &Plan,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), PlanValidationError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.to_string()) {
            return Err(PlanValidationError(format!(
                "plan has a dependency cycle at {id}"
            )));
        }
        if let Some(step) = plan.step(id) {
            for dependency in &step.depends_on {
                visit(dependency, plan, visiting, visited)?;
            }
        }
        visiting.remove(id);
        visited.insert(id.to_string());
        Ok(())
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for step in &plan.steps {
        visit(&step.id, plan, &mut visiting, &mut visited)?;
    }
    Ok(())
}

pub fn runnable_step<'a>(plan: &'a Plan, approved: &HashSet<String>) -> Option<&'a PlanStep> {
    let completed: HashSet<&str> = plan
        .steps
        .iter()
        .filter(|step| step.status == PlanStepStatus::Completed)
        .map(|step| step.id.as_str())
        .collect();
    plan.steps.iter().find(|step| {
        step.status == PlanStepStatus::Pending
            && step
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && (!step.requires_human || approved.contains(&step.id))
    })
}

pub fn blocked_step<'a>(plan: &'a Plan, approved: &HashSet<String>) -> Option<&'a PlanStep> {
    let completed: HashSet<&str> = plan
        .steps
        .iter()
        .filter(|step| step.status == PlanStepStatus::Completed)
        .map(|step| step.id.as_str())
        .collect();
    plan.steps.iter().find(|step| {
        step.status == PlanStepStatus::Pending
            && step
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && step.requires_human
            && !approved.contains(&step.id)
    })
}

pub fn downstream_ids(plan: &Plan, step_id: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut queue = vec![step_id.to_string()];
    while let Some(id) = queue.pop() {
        for step in &plan.steps {
            if step.depends_on.iter().any(|dependency| dependency == &id)
                && out.insert(step.id.clone())
            {
                queue.push(step.id.clone());
            }
        }
    }
    out
}

pub fn validate_replan(previous: Option<&Plan>, next: &Plan) -> Result<(), PlanValidationError> {
    validate_plan(next)?;
    let Some(previous) = previous else {
        return Ok(());
    };
    let previous_by_id: HashMap<&str, &PlanStep> = previous
        .steps
        .iter()
        .map(|step| (step.id.as_str(), step))
        .collect();
    let next_by_id: HashMap<&str, &PlanStep> = next
        .steps
        .iter()
        .map(|step| (step.id.as_str(), step))
        .collect();

    for step in &previous.steps {
        if step.status != PlanStepStatus::Completed {
            continue;
        }
        let Some(next_step) = next_by_id.get(step.id.as_str()) else {
            return Err(PlanValidationError(format!(
                "completed step {} must not be removed",
                step.id
            )));
        };
        if next_step.status != PlanStepStatus::Completed {
            return Err(PlanValidationError(format!(
                "completed step {} must stay completed",
                step.id
            )));
        }
        if !same_structure(step, next_step) {
            return Err(PlanValidationError(format!(
                "completed step {} must not be rewritten",
                step.id
            )));
        }
    }

    let mut allowed = HashSet::new();
    for failed in previous
        .steps
        .iter()
        .filter(|step| step.status == PlanStepStatus::Failed)
    {
        allowed.insert(failed.id.clone());
        allowed.extend(downstream_ids(previous, &failed.id));
    }

    for step in &previous.steps {
        if step.status == PlanStepStatus::Completed || allowed.contains(&step.id) {
            continue;
        }
        let Some(next_step) = next_by_id.get(step.id.as_str()) else {
            return Err(PlanValidationError(format!(
                "step {} may only be removed after it fails",
                step.id
            )));
        };
        if !same_structure(step, next_step) {
            return Err(PlanValidationError(format!(
                "step {} is not downstream of a failure and must not be rewritten",
                step.id
            )));
        }
    }

    for next_step in &next.steps {
        if previous_by_id.contains_key(next_step.id.as_str()) || allowed.contains(&next_step.id) {
            continue;
        }
        let dependencies_ok = next_step.depends_on.iter().all(|dependency| {
            allowed.contains(dependency)
                || previous_by_id
                    .get(dependency.as_str())
                    .is_some_and(|step| step.status == PlanStepStatus::Completed)
        });
        if !dependencies_ok {
            return Err(PlanValidationError(format!(
                "new step {} must depend on completed or replanned steps",
                next_step.id
            )));
        }
    }

    Ok(())
}

fn same_structure(left: &PlanStep, right: &PlanStep) -> bool {
    left.description == right.description
        && left.depends_on == right.depends_on
        && left.inputs == right.inputs
        && left.outputs == right.outputs
        && left.risk == right.risk
        && left.verification == right.verification
        && left.rollback == right.rollback
        && left.requires_human == right.requires_human
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, fixture_message_json};

    fn step(id: &str, status: PlanStepStatus, depends_on: &[&str]) -> PlanStep {
        PlanStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: depends_on.iter().map(|dep| dep.to_string()).collect(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            risk: PlanRisk::Low,
            verification: Some(format!("verify {id}")),
            rollback: None,
            requires_human: false,
            status,
            output_ref: None,
        }
    }

    fn plan(steps: Vec<PlanStep>) -> Plan {
        Plan {
            goal: "ship".into(),
            steps,
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[test]
    fn plan_round_trips_and_latest_wins() {
        let first = plan(vec![step("s1", PlanStepStatus::Pending, &[])]);
        let second = plan(vec![step("s1", PlanStepStatus::Completed, &[])]);
        let entries = vec![
            first.to_custom_entry_shell(),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "m1".into(),
                    parent_id: None,
                    timestamp: 1,
                },
                message: fixture_message_json("user", "hi"),
            }),
            second.to_custom_entry_shell(),
        ];
        let restored = latest_plan(&entries).expect("plan");
        assert_eq!(restored, second);
    }

    #[test]
    fn validate_rejects_duplicate_missing_and_cycles() {
        assert!(
            validate_plan(&plan(vec![
                step("s1", PlanStepStatus::Pending, &[]),
                step("s1", PlanStepStatus::Pending, &[]),
            ]))
            .is_err()
        );
        assert!(
            validate_plan(&plan(vec![step("s1", PlanStepStatus::Pending, &["nope"])])).is_err()
        );
        assert!(
            validate_plan(&plan(vec![
                step("s1", PlanStepStatus::Pending, &["s2"]),
                step("s2", PlanStepStatus::Pending, &["s1"]),
            ]))
            .is_err()
        );
    }

    #[test]
    fn runnable_respects_dependencies_and_approval() {
        let mut human = step("s3", PlanStepStatus::Pending, &["s1"]);
        human.requires_human = true;
        let sample = plan(vec![
            step("s1", PlanStepStatus::Completed, &[]),
            step("s2", PlanStepStatus::Pending, &["s1"]),
            human,
        ]);
        let approved = HashSet::new();
        assert_eq!(
            runnable_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s2")
        );
        assert_eq!(
            blocked_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s3")
        );
        let mut approved = HashSet::new();
        approved.insert("s3".into());
        assert_eq!(
            runnable_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s2")
        );
        let sample = plan(vec![
            step("s1", PlanStepStatus::Completed, &[]),
            step("s2", PlanStepStatus::Completed, &["s1"]),
            step("s3", PlanStepStatus::Pending, &["s2"]),
        ]);
        assert_eq!(
            runnable_step(&sample, &HashSet::new()).map(|step| step.id.as_str()),
            Some("s3")
        );
    }

    #[test]
    fn downstream_closure_is_transitive() {
        let plan = plan(vec![
            step("s1", PlanStepStatus::Failed, &[]),
            step("s2", PlanStepStatus::Pending, &["s1"]),
            step("s3", PlanStepStatus::Pending, &["s2"]),
            step("s4", PlanStepStatus::Pending, &[]),
        ]);
        let downstream = downstream_ids(&plan, "s1");
        assert!(downstream.contains("s2"));
        assert!(downstream.contains("s3"));
        assert!(!downstream.contains("s4"));
    }

    #[test]
    fn replan_locks_completed_and_unrelated_steps() {
        let previous = plan(vec![
            step("s1", PlanStepStatus::Completed, &[]),
            step("s2", PlanStepStatus::Failed, &["s1"]),
            step("s3", PlanStepStatus::Pending, &["s2"]),
            step("s4", PlanStepStatus::Pending, &[]),
        ]);

        let mut rewritten_completed = previous.clone();
        rewritten_completed.steps[0].description = "changed".into();
        assert!(validate_replan(Some(&previous), &rewritten_completed).is_err());

        let mut rewritten_unrelated = previous.clone();
        rewritten_unrelated.steps[3].description = "changed".into();
        assert!(validate_replan(Some(&previous), &rewritten_unrelated).is_err());

        let mut replanned = previous.clone();
        replanned.steps[1].description = "do s2 differently".into();
        replanned.steps[1].status = PlanStepStatus::Pending;
        replanned.steps[2].description = "do s3 after the fix".into();
        assert!(validate_replan(Some(&previous), &replanned).is_ok());
    }

    #[test]
    fn status_changes_are_allowed_without_failure() {
        let previous = plan(vec![
            step("s1", PlanStepStatus::Completed, &[]),
            step("s2", PlanStepStatus::Pending, &["s1"]),
        ]);
        let mut next = previous.clone();
        next.steps[1].status = PlanStepStatus::InProgress;
        next.steps[1].output_ref = Some("artifact:1".into());
        assert!(validate_replan(Some(&previous), &next).is_ok());
    }
}
