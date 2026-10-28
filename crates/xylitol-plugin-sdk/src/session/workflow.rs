use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};

pub const CUSTOM_TYPE_WORKFLOW: &str = "workflow";
pub(crate) const LEGACY_CUSTOM_TYPE_PLAN: &str = "plan";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRisk {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowStep {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub risk: WorkflowRisk,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<String>,
    #[serde(default)]
    pub requires_human: bool,
    #[serde(default)]
    pub status: WorkflowStepStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_workflow: Option<Box<Workflow>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Workflow {
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<String>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowValidationError(pub String);

impl std::fmt::Display for WorkflowValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WorkflowValidationError {}

impl Workflow {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn all_completed(&self) -> bool {
        !self.steps.is_empty()
            && self
                .steps
                .iter()
                .all(|step| step.status == WorkflowStepStatus::Completed)
    }

    pub fn completed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == WorkflowStepStatus::Completed)
            .count()
    }

    pub fn step(&self, id: &str) -> Option<&WorkflowStep> {
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
            custom_type: CUSTOM_TYPE_WORKFLOW.into(),
            data: self.to_data_value(),
        })
    }
}

pub fn latest_workflow(entries: &[SessionEntry]) -> Option<Workflow> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom)
            if custom.custom_type == CUSTOM_TYPE_WORKFLOW
                || custom.custom_type == LEGACY_CUSTOM_TYPE_PLAN =>
        {
            Workflow::from_data_value(&custom.data)
        }
        _ => None,
    })
}

pub const DEFAULT_MAX_DEPTH: u32 = 2;
pub const DEFAULT_MAX_TOTAL_STEPS: usize = 100;

pub fn validate_workflow(plan: &Workflow) -> Result<(), WorkflowValidationError> {
    validate_workflow_with_limits(plan, DEFAULT_MAX_DEPTH, DEFAULT_MAX_TOTAL_STEPS)
}

pub fn validate_workflow_with_limits(
    plan: &Workflow,
    max_depth: u32,
    max_total_steps: usize,
) -> Result<(), WorkflowValidationError> {
    let mut total = 0;
    validate_workflow_level(plan, 1, max_depth, max_total_steps, &mut total)
}

fn validate_workflow_level(
    plan: &Workflow,
    depth: u32,
    max_depth: u32,
    max_total_steps: usize,
    total: &mut usize,
) -> Result<(), WorkflowValidationError> {
    if depth > max_depth {
        return Err(WorkflowValidationError(format!(
            "workflow nesting depth {depth} exceeds max_depth {max_depth}"
        )));
    }
    if plan.steps.is_empty() {
        return Err(WorkflowValidationError(
            "workflow must have at least one step".into(),
        ));
    }
    *total += plan.steps.len();
    if *total > max_total_steps {
        return Err(WorkflowValidationError(format!(
            "workflow has {total} total steps, exceeds max_total_steps {max_total_steps}"
        )));
    }
    let mut ids = HashSet::new();
    for step in &plan.steps {
        if step.id.trim().is_empty() {
            return Err(WorkflowValidationError(
                "workflow step id must be non-empty".into(),
            ));
        }
        if !ids.insert(step.id.clone()) {
            return Err(WorkflowValidationError(format!(
                "duplicate workflow step id: {}",
                step.id
            )));
        }
    }
    for step in &plan.steps {
        for dependency in &step.depends_on {
            if dependency == &step.id {
                return Err(WorkflowValidationError(format!(
                    "workflow step {} depends on itself",
                    step.id
                )));
            }
            if !ids.contains(dependency) {
                return Err(WorkflowValidationError(format!(
                    "workflow step {} depends on unknown step {}",
                    step.id, dependency
                )));
            }
        }
    }
    detect_cycle(plan)?;
    for step in &plan.steps {
        if let Some(sub) = step.sub_workflow.as_deref() {
            validate_workflow_level(sub, depth + 1, max_depth, max_total_steps, total)?;
        }
    }
    Ok(())
}

fn detect_cycle(plan: &Workflow) -> Result<(), WorkflowValidationError> {
    fn visit(
        id: &str,
        plan: &Workflow,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), WorkflowValidationError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.to_string()) {
            return Err(WorkflowValidationError(format!(
                "workflow has a dependency cycle at {id}"
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

pub fn runnable_workflow_step<'a>(
    plan: &'a Workflow,
    approved: &HashSet<String>,
) -> Option<&'a WorkflowStep> {
    let completed: HashSet<&str> = plan
        .steps
        .iter()
        .filter(|step| step.status == WorkflowStepStatus::Completed)
        .map(|step| step.id.as_str())
        .collect();
    plan.steps.iter().find(|step| {
        step.status == WorkflowStepStatus::Pending
            && step
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && (!step.requires_human || approved.contains(&step.id))
    })
}

pub fn blocked_workflow_step<'a>(
    plan: &'a Workflow,
    approved: &HashSet<String>,
) -> Option<&'a WorkflowStep> {
    let completed: HashSet<&str> = plan
        .steps
        .iter()
        .filter(|step| step.status == WorkflowStepStatus::Completed)
        .map(|step| step.id.as_str())
        .collect();
    plan.steps.iter().find(|step| {
        step.status == WorkflowStepStatus::Pending
            && step
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && step.requires_human
            && !approved.contains(&step.id)
    })
}

pub fn workflow_downstream_ids(plan: &Workflow, step_id: &str) -> HashSet<String> {
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

pub fn validate_workflow_replan(
    previous: Option<&Workflow>,
    next: &Workflow,
) -> Result<(), WorkflowValidationError> {
    validate_workflow(next)?;
    let Some(previous) = previous else {
        return Ok(());
    };
    let previous_by_id: HashMap<&str, &WorkflowStep> = previous
        .steps
        .iter()
        .map(|step| (step.id.as_str(), step))
        .collect();
    let next_by_id: HashMap<&str, &WorkflowStep> = next
        .steps
        .iter()
        .map(|step| (step.id.as_str(), step))
        .collect();

    for step in &previous.steps {
        if step.status != WorkflowStepStatus::Completed {
            continue;
        }
        let Some(next_step) = next_by_id.get(step.id.as_str()) else {
            return Err(WorkflowValidationError(format!(
                "completed step {} must not be removed",
                step.id
            )));
        };
        if next_step.status != WorkflowStepStatus::Completed {
            return Err(WorkflowValidationError(format!(
                "completed step {} must stay completed",
                step.id
            )));
        }
        if !same_structure(step, next_step) {
            return Err(WorkflowValidationError(format!(
                "completed step {} must not be rewritten",
                step.id
            )));
        }
    }

    let mut allowed = HashSet::new();
    for failed in previous
        .steps
        .iter()
        .filter(|step| step.status == WorkflowStepStatus::Failed)
    {
        allowed.insert(failed.id.clone());
        allowed.extend(workflow_downstream_ids(previous, &failed.id));
    }

    for step in &previous.steps {
        if step.status == WorkflowStepStatus::Completed || allowed.contains(&step.id) {
            continue;
        }
        let Some(next_step) = next_by_id.get(step.id.as_str()) else {
            return Err(WorkflowValidationError(format!(
                "step {} may only be removed after it fails",
                step.id
            )));
        };
        if !same_structure(step, next_step) {
            return Err(WorkflowValidationError(format!(
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
                    .is_some_and(|step| step.status == WorkflowStepStatus::Completed)
        });
        if !dependencies_ok {
            return Err(WorkflowValidationError(format!(
                "new step {} must depend on completed or replanned steps",
                next_step.id
            )));
        }
    }

    Ok(())
}

fn same_structure(left: &WorkflowStep, right: &WorkflowStep) -> bool {
    left.description == right.description
        && left.depends_on == right.depends_on
        && left.inputs == right.inputs
        && left.outputs == right.outputs
        && left.risk == right.risk
        && left.verification == right.verification
        && left.rollback == right.rollback
        && left.requires_human == right.requires_human
        && same_workflow_structure(left.sub_workflow.as_deref(), right.sub_workflow.as_deref())
}

fn same_workflow_structure(left: Option<&Workflow>, right: Option<&Workflow>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.goal == right.goal
                && left.steps.len() == right.steps.len()
                && left
                    .steps
                    .iter()
                    .zip(right.steps.iter())
                    .all(|(left, right)| same_structure(left, right))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, fixture_message_json};

    fn step(id: &str, status: WorkflowStepStatus, depends_on: &[&str]) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: depends_on.iter().map(|dep| dep.to_string()).collect(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            tools: Vec::new(),
            risk: WorkflowRisk::Low,
            verification: Some(format!("verify {id}")),
            rollback: None,
            requires_human: false,
            status,
            output_ref: None,
            sub_workflow: None,
        }
    }

    fn plan(steps: Vec<WorkflowStep>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps,
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[test]
    fn step_tools_round_trip_and_default_to_empty() {
        let mut declared = step("s1", WorkflowStepStatus::Pending, &[]);
        declared.tools = vec!["write".into(), "edit".into()];
        let wf = plan(vec![declared]);
        let back = Workflow::from_data_value(&wf.to_data_value()).expect("round trip");
        assert_eq!(back.steps[0].tools, vec!["write", "edit"]);

        let legacy = serde_json::json!({
            "goal": "ship",
            "steps": [{"id": "s1", "description": "do it"}]
        });
        let parsed = Workflow::from_data_value(&legacy).expect("legacy workflow");
        assert!(
            parsed.steps[0].tools.is_empty(),
            "legacy workflows without tools must keep the coarse attribution path"
        );
    }

    #[test]
    fn plan_round_trips_and_latest_wins() {
        let first = plan(vec![step("s1", WorkflowStepStatus::Pending, &[])]);
        let second = plan(vec![step("s1", WorkflowStepStatus::Completed, &[])]);
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
        let restored = latest_workflow(&entries).expect("workflow");
        assert_eq!(restored, second);
    }

    #[test]
    fn legacy_plan_entry_is_read_as_workflow() {
        let mut entry =
            plan(vec![step("s1", WorkflowStepStatus::Pending, &[])]).to_custom_entry_shell();
        if let SessionEntry::Custom(custom) = &mut entry {
            custom.custom_type = "plan".into();
        }
        let restored = latest_workflow(&[entry]).expect("legacy workflow");
        assert_eq!(restored.goal, "ship");
    }

    #[test]
    fn new_workflow_writes_new_custom_type() {
        let entry =
            plan(vec![step("s1", WorkflowStepStatus::Pending, &[])]).to_custom_entry_shell();
        let SessionEntry::Custom(custom) = entry else {
            panic!("workflow must persist as a custom entry");
        };
        assert_eq!(custom.custom_type, "workflow");
    }

    #[test]
    fn validate_rejects_duplicate_missing_and_cycles() {
        assert!(
            validate_workflow(&plan(vec![
                step("s1", WorkflowStepStatus::Pending, &[]),
                step("s1", WorkflowStepStatus::Pending, &[]),
            ]))
            .is_err()
        );
        assert!(
            validate_workflow(&plan(vec![step(
                "s1",
                WorkflowStepStatus::Pending,
                &["nope"]
            )]))
            .is_err()
        );
        assert!(
            validate_workflow(&plan(vec![
                step("s1", WorkflowStepStatus::Pending, &["s2"]),
                step("s2", WorkflowStepStatus::Pending, &["s1"]),
            ]))
            .is_err()
        );
    }

    #[test]
    fn runnable_respects_dependencies_and_approval() {
        let mut human = step("s3", WorkflowStepStatus::Pending, &["s1"]);
        human.requires_human = true;
        let sample = plan(vec![
            step("s1", WorkflowStepStatus::Completed, &[]),
            step("s2", WorkflowStepStatus::Pending, &["s1"]),
            human,
        ]);
        let approved = HashSet::new();
        assert_eq!(
            runnable_workflow_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s2")
        );
        assert_eq!(
            blocked_workflow_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s3")
        );
        let mut approved = HashSet::new();
        approved.insert("s3".into());
        assert_eq!(
            runnable_workflow_step(&sample, &approved).map(|step| step.id.as_str()),
            Some("s2")
        );
        let sample = plan(vec![
            step("s1", WorkflowStepStatus::Completed, &[]),
            step("s2", WorkflowStepStatus::Completed, &["s1"]),
            step("s3", WorkflowStepStatus::Pending, &["s2"]),
        ]);
        assert_eq!(
            runnable_workflow_step(&sample, &HashSet::new()).map(|step| step.id.as_str()),
            Some("s3")
        );
    }

    #[test]
    fn downstream_closure_is_transitive() {
        let plan = plan(vec![
            step("s1", WorkflowStepStatus::Failed, &[]),
            step("s2", WorkflowStepStatus::Pending, &["s1"]),
            step("s3", WorkflowStepStatus::Pending, &["s2"]),
            step("s4", WorkflowStepStatus::Pending, &[]),
        ]);
        let downstream = workflow_downstream_ids(&plan, "s1");
        assert!(downstream.contains("s2"));
        assert!(downstream.contains("s3"));
        assert!(!downstream.contains("s4"));
    }

    #[test]
    fn replan_locks_completed_and_unrelated_steps() {
        let previous = plan(vec![
            step("s1", WorkflowStepStatus::Completed, &[]),
            step("s2", WorkflowStepStatus::Failed, &["s1"]),
            step("s3", WorkflowStepStatus::Pending, &["s2"]),
            step("s4", WorkflowStepStatus::Pending, &[]),
        ]);

        let mut rewritten_completed = previous.clone();
        rewritten_completed.steps[0].description = "changed".into();
        assert!(validate_workflow_replan(Some(&previous), &rewritten_completed).is_err());

        let mut rewritten_unrelated = previous.clone();
        rewritten_unrelated.steps[3].description = "changed".into();
        assert!(validate_workflow_replan(Some(&previous), &rewritten_unrelated).is_err());

        let mut replanned = previous.clone();
        replanned.steps[1].description = "do s2 differently".into();
        replanned.steps[1].status = WorkflowStepStatus::Pending;
        replanned.steps[2].description = "do s3 after the fix".into();
        assert!(validate_workflow_replan(Some(&previous), &replanned).is_ok());
    }

    #[test]
    fn status_changes_are_allowed_without_failure() {
        let previous = plan(vec![
            step("s1", WorkflowStepStatus::Completed, &[]),
            step("s2", WorkflowStepStatus::Pending, &["s1"]),
        ]);
        let mut next = previous.clone();
        next.steps[1].status = WorkflowStepStatus::InProgress;
        next.steps[1].output_ref = Some("artifact:1".into());
        assert!(validate_workflow_replan(Some(&previous), &next).is_ok());
    }
}

#[cfg(test)]
mod nested_tests {
    use super::*;

    fn leaf(id: &str) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            description: format!("do {id}"),
            depends_on: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            tools: Vec::new(),
            risk: WorkflowRisk::Low,
            verification: None,
            rollback: None,
            requires_human: false,
            status: WorkflowStepStatus::Pending,
            output_ref: None,
            sub_workflow: None,
        }
    }

    fn container(id: &str, child: Workflow) -> WorkflowStep {
        WorkflowStep {
            sub_workflow: Some(Box::new(child)),
            ..leaf(id)
        }
    }

    fn root(steps: Vec<WorkflowStep>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            steps,
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[test]
    fn nested_workflow_round_trips() {
        let workflow = root(vec![container("s1", root(vec![leaf("s1.1")]))]);
        let entries = vec![workflow.to_custom_entry_shell()];
        assert_eq!(latest_workflow(&entries), Some(workflow));
    }

    #[test]
    fn depth_and_total_step_bounds_are_enforced() {
        let deep = root(vec![container(
            "s1",
            root(vec![container("s1.1", root(vec![leaf("s1.1.1")]))]),
        )]);
        assert!(validate_workflow_with_limits(&deep, 2, 100).is_err());
        assert!(validate_workflow_with_limits(&deep, 3, 100).is_ok());
        let wide = root(vec![leaf("a"), leaf("b")]);
        assert!(validate_workflow_with_limits(&wide, 2, 1).is_err());
    }

    #[test]
    fn nested_duplicate_and_cycle_are_rejected() {
        let duplicate = root(vec![container("s1", root(vec![leaf("x"), leaf("x")]))]);
        assert!(validate_workflow(&duplicate).is_err());
        let mut a = leaf("a");
        let mut b = leaf("b");
        a.depends_on = vec!["b".into()];
        b.depends_on = vec!["a".into()];
        let cycle = root(vec![container("s1", root(vec![a, b]))]);
        assert!(validate_workflow(&cycle).is_err());
    }
}
