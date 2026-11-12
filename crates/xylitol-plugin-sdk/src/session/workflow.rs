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
pub enum WorkflowTaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowPhase {
    Contract,
    Executing,
    AwaitingApproval,
    Replanning,
    SubWorkflow,
    Summarizing,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowTaskEvent {
    Start,
    Finish,
    Fail,
    Block,
    Approve,
    Replan,
}

pub fn task_transition(
    from: WorkflowTaskStatus,
    event: WorkflowTaskEvent,
) -> Result<WorkflowTaskStatus, WorkflowValidationError> {
    let next = match (from, event) {
        (WorkflowTaskStatus::Pending, WorkflowTaskEvent::Start) => WorkflowTaskStatus::InProgress,
        (WorkflowTaskStatus::InProgress, WorkflowTaskEvent::Finish) => {
            WorkflowTaskStatus::Completed
        }
        (WorkflowTaskStatus::InProgress, WorkflowTaskEvent::Fail) => WorkflowTaskStatus::Failed,
        (WorkflowTaskStatus::InProgress, WorkflowTaskEvent::Block) => WorkflowTaskStatus::Blocked,
        (WorkflowTaskStatus::Blocked, WorkflowTaskEvent::Approve) => WorkflowTaskStatus::InProgress,
        (WorkflowTaskStatus::Failed, WorkflowTaskEvent::Replan) => WorkflowTaskStatus::InProgress,
        _ => {
            return Err(WorkflowValidationError(format!(
                "illegal workflow task transition: {from:?} --{event:?}-->"
            )));
        }
    };
    Ok(next)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTask {
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
    pub status: WorkflowTaskStatus,
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
    pub tasks: Vec<WorkflowTask>,
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
        self.tasks.is_empty()
    }

    pub fn all_completed(&self) -> bool {
        !self.tasks.is_empty()
            && self
                .tasks
                .iter()
                .all(|task| task.status == WorkflowTaskStatus::Completed)
    }

    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.status == WorkflowTaskStatus::Completed)
            .count()
    }

    pub fn task(&self, id: &str) -> Option<&WorkflowTask> {
        self.tasks.iter().find(|task| task.id == id)
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
    if plan.tasks.is_empty() {
        return Err(WorkflowValidationError(
            "workflow must have at least one task".into(),
        ));
    }
    *total += plan.tasks.len();
    if *total > max_total_steps {
        return Err(WorkflowValidationError(format!(
            "workflow has {total} total tasks, exceeds max_total_steps {max_total_steps}"
        )));
    }
    let mut ids = HashSet::new();
    for task in &plan.tasks {
        if task.id.trim().is_empty() {
            return Err(WorkflowValidationError(
                "workflow task id must be non-empty".into(),
            ));
        }
        if !ids.insert(task.id.clone()) {
            return Err(WorkflowValidationError(format!(
                "duplicate workflow task id: {}",
                task.id
            )));
        }
    }
    for task in &plan.tasks {
        for dependency in &task.depends_on {
            if dependency == &task.id {
                return Err(WorkflowValidationError(format!(
                    "workflow task {} depends on itself",
                    task.id
                )));
            }
            if !ids.contains(dependency) {
                return Err(WorkflowValidationError(format!(
                    "workflow task {} depends on unknown task {}",
                    task.id, dependency
                )));
            }
        }
    }
    detect_cycle(plan)?;
    for task in &plan.tasks {
        if let Some(sub) = task.sub_workflow.as_deref() {
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
        if let Some(task) = plan.task(id) {
            for dependency in &task.depends_on {
                visit(dependency, plan, visiting, visited)?;
            }
        }
        visiting.remove(id);
        visited.insert(id.to_string());
        Ok(())
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for task in &plan.tasks {
        visit(&task.id, plan, &mut visiting, &mut visited)?;
    }
    Ok(())
}

pub fn runnable_workflow_task<'a>(
    plan: &'a Workflow,
    approved: &HashSet<String>,
) -> Option<&'a WorkflowTask> {
    let completed: HashSet<&str> = plan
        .tasks
        .iter()
        .filter(|task| task.status == WorkflowTaskStatus::Completed)
        .map(|task| task.id.as_str())
        .collect();
    plan.tasks.iter().find(|task| {
        task.status == WorkflowTaskStatus::Pending
            && task
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && (!task.requires_human || approved.contains(&task.id))
    })
}

pub fn blocked_workflow_task<'a>(
    plan: &'a Workflow,
    approved: &HashSet<String>,
) -> Option<&'a WorkflowTask> {
    let completed: HashSet<&str> = plan
        .tasks
        .iter()
        .filter(|task| task.status == WorkflowTaskStatus::Completed)
        .map(|task| task.id.as_str())
        .collect();
    plan.tasks.iter().find(|task| {
        task.status == WorkflowTaskStatus::Pending
            && task
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
            && task.requires_human
            && !approved.contains(&task.id)
    })
}

pub fn workflow_downstream_ids(plan: &Workflow, task_id: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut queue = vec![task_id.to_string()];
    while let Some(id) = queue.pop() {
        for task in &plan.tasks {
            if task.depends_on.iter().any(|dependency| dependency == &id)
                && out.insert(task.id.clone())
            {
                queue.push(task.id.clone());
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
    let previous_by_id: HashMap<&str, &WorkflowTask> = previous
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    let next_by_id: HashMap<&str, &WorkflowTask> = next
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();

    for task in &previous.tasks {
        if task.status != WorkflowTaskStatus::Completed {
            continue;
        }
        let Some(next_step) = next_by_id.get(task.id.as_str()) else {
            return Err(WorkflowValidationError(format!(
                "completed task {} must not be removed",
                task.id
            )));
        };
        if next_step.status != WorkflowTaskStatus::Completed {
            return Err(WorkflowValidationError(format!(
                "completed task {} must stay completed",
                task.id
            )));
        }
        if !same_structure(task, next_step) {
            return Err(WorkflowValidationError(format!(
                "completed task {} must not be rewritten",
                task.id
            )));
        }
    }

    let allowed = replan_allowed_ids(previous);

    for task in &previous.tasks {
        if task.status == WorkflowTaskStatus::Completed || allowed.contains(&task.id) {
            continue;
        }
        let Some(next_step) = next_by_id.get(task.id.as_str()) else {
            return Err(WorkflowValidationError(format!(
                "task {} may only be removed after it fails",
                task.id
            )));
        };
        if !same_structure(task, next_step) {
            return Err(WorkflowValidationError(format!(
                "task {} is not downstream of a failure and must not be rewritten",
                task.id
            )));
        }
    }

    for next_step in &next.tasks {
        if previous_by_id.contains_key(next_step.id.as_str()) || allowed.contains(&next_step.id) {
            continue;
        }
        let dependencies_ok = next_step.depends_on.iter().all(|dependency| {
            allowed.contains(dependency)
                || previous_by_id
                    .get(dependency.as_str())
                    .is_some_and(|task| task.status == WorkflowTaskStatus::Completed)
        });
        if !dependencies_ok {
            return Err(WorkflowValidationError(format!(
                "new task {} must depend on completed or replanned tasks",
                next_step.id
            )));
        }
    }

    Ok(())
}

pub fn validate_workflow_status_delta(
    previous: Option<&Workflow>,
    next: &Workflow,
) -> Result<(), WorkflowValidationError> {
    let Some(previous) = previous else {
        return Ok(());
    };
    let allowed = replan_allowed_ids(previous);
    for task in &next.tasks {
        let Some(prev_task) = previous.task(&task.id) else {
            continue;
        };
        if task.status == prev_task.status || allowed.contains(&task.id) {
            continue;
        }
        let reachable = WORKFLOW_TASK_EVENTS.iter().any(|event| {
            task_transition(prev_task.status, *event).is_ok_and(|status| status == task.status)
        });
        if !reachable {
            return Err(WorkflowValidationError(format!(
                "illegal workflow task status delta: task {} {:?} -> {:?}",
                task.id, prev_task.status, task.status
            )));
        }
    }
    Ok(())
}

const WORKFLOW_TASK_EVENTS: [WorkflowTaskEvent; 6] = [
    WorkflowTaskEvent::Start,
    WorkflowTaskEvent::Finish,
    WorkflowTaskEvent::Fail,
    WorkflowTaskEvent::Block,
    WorkflowTaskEvent::Approve,
    WorkflowTaskEvent::Replan,
];

fn replan_allowed_ids(previous: &Workflow) -> HashSet<String> {
    let mut allowed = HashSet::new();
    for failed in previous
        .tasks
        .iter()
        .filter(|task| task.status == WorkflowTaskStatus::Failed)
    {
        allowed.insert(failed.id.clone());
        allowed.extend(workflow_downstream_ids(previous, &failed.id));
    }
    allowed
}

fn same_structure(left: &WorkflowTask, right: &WorkflowTask) -> bool {
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
                && left.tasks.len() == right.tasks.len()
                && left
                    .tasks
                    .iter()
                    .zip(right.tasks.iter())
                    .all(|(left, right)| same_structure(left, right))
        }
        _ => false,
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;

    #[test]
    fn legal_transitions() {
        use WorkflowTaskEvent as E;
        use WorkflowTaskStatus as S;
        assert_eq!(
            task_transition(S::Pending, E::Start).unwrap(),
            S::InProgress
        );
        assert_eq!(
            task_transition(S::InProgress, E::Finish).unwrap(),
            S::Completed
        );
        assert_eq!(task_transition(S::InProgress, E::Fail).unwrap(), S::Failed);
        assert_eq!(
            task_transition(S::InProgress, E::Block).unwrap(),
            S::Blocked
        );
        assert_eq!(
            task_transition(S::Blocked, E::Approve).unwrap(),
            S::InProgress
        );
        assert_eq!(
            task_transition(S::Failed, E::Replan).unwrap(),
            S::InProgress
        );
    }

    #[test]
    fn completed_is_locked() {
        use WorkflowTaskEvent as E;
        use WorkflowTaskStatus as S;
        for event in [
            E::Start,
            E::Finish,
            E::Fail,
            E::Block,
            E::Approve,
            E::Replan,
        ] {
            assert!(task_transition(S::Completed, event).is_err());
        }
    }

    #[test]
    fn pending_cannot_fail_directly() {
        assert!(task_transition(WorkflowTaskStatus::Pending, WorkflowTaskEvent::Fail).is_err());
    }

    #[test]
    fn blocked_requires_approval() {
        assert!(task_transition(WorkflowTaskStatus::Blocked, WorkflowTaskEvent::Start).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, fixture_message_json};

    fn task(id: &str, status: WorkflowTaskStatus, depends_on: &[&str]) -> WorkflowTask {
        WorkflowTask {
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

    fn plan(tasks: Vec<WorkflowTask>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            tasks,
            resume_cursor: None,
            updated_at: 1,
        }
    }

    #[test]
    fn step_tools_round_trip_and_default_to_empty() {
        let mut declared = task("s1", WorkflowTaskStatus::Pending, &[]);
        declared.tools = vec!["write".into(), "edit".into()];
        let wf = plan(vec![declared]);
        let back = Workflow::from_data_value(&wf.to_data_value()).expect("round trip");
        assert_eq!(back.tasks[0].tools, vec!["write", "edit"]);

        let legacy = serde_json::json!({
            "goal": "ship",
            "tasks": [{"id": "s1", "description": "do it"}]
        });
        let parsed = Workflow::from_data_value(&legacy).expect("legacy workflow");
        assert!(
            parsed.tasks[0].tools.is_empty(),
            "legacy workflows without tools must keep the coarse attribution path"
        );
    }

    #[test]
    fn same_workflow_structure_ignores_status_and_outputs() {
        let mut left = plan(vec![task("s1", WorkflowTaskStatus::Pending, &[])]);
        let mut right = left.clone();
        right.tasks[0].status = WorkflowTaskStatus::Completed;
        right.tasks[0].output_ref = Some("artifact".into());
        assert!(same_workflow_structure(Some(&left), Some(&right)));

        left.tasks[0].description = "changed".into();
        assert!(!same_workflow_structure(Some(&left), Some(&right)));
        assert!(!same_workflow_structure(None, Some(&right)));
        assert!(same_workflow_structure(None, None));
    }

    #[test]
    fn plan_round_trips_and_latest_wins() {
        let first = plan(vec![task("s1", WorkflowTaskStatus::Pending, &[])]);
        let second = plan(vec![task("s1", WorkflowTaskStatus::Completed, &[])]);
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
            plan(vec![task("s1", WorkflowTaskStatus::Pending, &[])]).to_custom_entry_shell();
        if let SessionEntry::Custom(custom) = &mut entry {
            custom.custom_type = "plan".into();
        }
        let restored = latest_workflow(&[entry]).expect("legacy workflow");
        assert_eq!(restored.goal, "ship");
    }

    #[test]
    fn new_workflow_writes_new_custom_type() {
        let entry =
            plan(vec![task("s1", WorkflowTaskStatus::Pending, &[])]).to_custom_entry_shell();
        let SessionEntry::Custom(custom) = entry else {
            panic!("workflow must persist as a custom entry");
        };
        assert_eq!(custom.custom_type, "workflow");
    }

    #[test]
    fn validate_rejects_duplicate_missing_and_cycles() {
        assert!(
            validate_workflow(&plan(vec![
                task("s1", WorkflowTaskStatus::Pending, &[]),
                task("s1", WorkflowTaskStatus::Pending, &[]),
            ]))
            .is_err()
        );
        assert!(
            validate_workflow(&plan(vec![task(
                "s1",
                WorkflowTaskStatus::Pending,
                &["nope"]
            )]))
            .is_err()
        );
        assert!(
            validate_workflow(&plan(vec![
                task("s1", WorkflowTaskStatus::Pending, &["s2"]),
                task("s2", WorkflowTaskStatus::Pending, &["s1"]),
            ]))
            .is_err()
        );
    }

    #[test]
    fn runnable_respects_dependencies_and_approval() {
        let mut human = task("s3", WorkflowTaskStatus::Pending, &["s1"]);
        human.requires_human = true;
        let sample = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Pending, &["s1"]),
            human,
        ]);
        let approved = HashSet::new();
        assert_eq!(
            runnable_workflow_task(&sample, &approved).map(|task| task.id.as_str()),
            Some("s2")
        );
        assert_eq!(
            blocked_workflow_task(&sample, &approved).map(|task| task.id.as_str()),
            Some("s3")
        );
        let mut approved = HashSet::new();
        approved.insert("s3".into());
        assert_eq!(
            runnable_workflow_task(&sample, &approved).map(|task| task.id.as_str()),
            Some("s2")
        );
        let sample = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Completed, &["s1"]),
            task("s3", WorkflowTaskStatus::Pending, &["s2"]),
        ]);
        assert_eq!(
            runnable_workflow_task(&sample, &HashSet::new()).map(|task| task.id.as_str()),
            Some("s3")
        );
    }

    #[test]
    fn downstream_closure_is_transitive() {
        let plan = plan(vec![
            task("s1", WorkflowTaskStatus::Failed, &[]),
            task("s2", WorkflowTaskStatus::Pending, &["s1"]),
            task("s3", WorkflowTaskStatus::Pending, &["s2"]),
            task("s4", WorkflowTaskStatus::Pending, &[]),
        ]);
        let downstream = workflow_downstream_ids(&plan, "s1");
        assert!(downstream.contains("s2"));
        assert!(downstream.contains("s3"));
        assert!(!downstream.contains("s4"));
    }

    #[test]
    fn replan_locks_completed_and_unrelated_steps() {
        let previous = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Failed, &["s1"]),
            task("s3", WorkflowTaskStatus::Pending, &["s2"]),
            task("s4", WorkflowTaskStatus::Pending, &[]),
        ]);

        let mut rewritten_completed = previous.clone();
        rewritten_completed.tasks[0].description = "changed".into();
        assert!(validate_workflow_replan(Some(&previous), &rewritten_completed).is_err());

        let mut rewritten_unrelated = previous.clone();
        rewritten_unrelated.tasks[3].description = "changed".into();
        assert!(validate_workflow_replan(Some(&previous), &rewritten_unrelated).is_err());

        let mut replanned = previous.clone();
        replanned.tasks[1].description = "do s2 differently".into();
        replanned.tasks[1].status = WorkflowTaskStatus::Pending;
        replanned.tasks[2].description = "do s3 after the fix".into();
        assert!(validate_workflow_replan(Some(&previous), &replanned).is_ok());
    }

    #[test]
    fn status_delta_first_write_has_no_previous() {
        let next = plan(vec![task("s1", WorkflowTaskStatus::Completed, &[])]);
        assert!(validate_workflow_status_delta(None, &next).is_ok());
    }

    #[test]
    fn status_delta_allows_table_reachable_changes() {
        let previous = plan(vec![
            task("s1", WorkflowTaskStatus::Pending, &[]),
            task("s2", WorkflowTaskStatus::InProgress, &[]),
            task("s3", WorkflowTaskStatus::Blocked, &[]),
            task("s4", WorkflowTaskStatus::Failed, &[]),
        ]);
        let mut next = previous.clone();
        next.tasks[0].status = WorkflowTaskStatus::InProgress;
        next.tasks[1].status = WorkflowTaskStatus::Completed;
        next.tasks[2].status = WorkflowTaskStatus::InProgress;
        next.tasks[3].status = WorkflowTaskStatus::InProgress;
        assert!(validate_workflow_status_delta(Some(&previous), &next).is_ok());
    }

    #[test]
    fn status_delta_rejects_table_bypass() {
        let previous = plan(vec![task("s1", WorkflowTaskStatus::Pending, &[])]);
        let mut next = previous.clone();
        next.tasks[0].status = WorkflowTaskStatus::Completed;
        let err = validate_workflow_status_delta(Some(&previous), &next).unwrap_err();
        assert!(err.0.contains("s1"), "{}", err.0);
        assert!(err.0.contains("Pending"), "{}", err.0);
        assert!(err.0.contains("Completed"), "{}", err.0);
    }

    #[test]
    fn status_delta_locks_completed_without_failure_downstream() {
        let previous = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Pending, &[]),
        ]);
        let unchanged = previous.clone();
        assert!(validate_workflow_status_delta(Some(&previous), &unchanged).is_ok());
        for status in [
            WorkflowTaskStatus::Pending,
            WorkflowTaskStatus::InProgress,
            WorkflowTaskStatus::Failed,
            WorkflowTaskStatus::Blocked,
        ] {
            let mut next = previous.clone();
            next.tasks[0].status = status;
            assert!(
                validate_workflow_status_delta(Some(&previous), &next).is_err(),
                "completed task must not move to {status:?}"
            );
        }
    }

    #[test]
    fn status_delta_replan_exception_covers_failed_closure() {
        let previous = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Failed, &["s1"]),
            task("s3", WorkflowTaskStatus::Pending, &["s2"]),
        ]);
        let mut next = previous.clone();
        next.tasks[1].status = WorkflowTaskStatus::Pending;
        next.tasks[2].status = WorkflowTaskStatus::Completed;
        assert!(validate_workflow_status_delta(Some(&previous), &next).is_ok());
    }

    #[test]
    fn status_changes_are_allowed_without_failure() {
        let previous = plan(vec![
            task("s1", WorkflowTaskStatus::Completed, &[]),
            task("s2", WorkflowTaskStatus::Pending, &["s1"]),
        ]);
        let mut next = previous.clone();
        next.tasks[1].status = WorkflowTaskStatus::InProgress;
        next.tasks[1].output_ref = Some("artifact:1".into());
        assert!(validate_workflow_replan(Some(&previous), &next).is_ok());
    }
}

#[cfg(test)]
mod nested_tests {
    use super::*;

    fn leaf(id: &str) -> WorkflowTask {
        WorkflowTask {
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
            status: WorkflowTaskStatus::Pending,
            output_ref: None,
            sub_workflow: None,
        }
    }

    fn container(id: &str, child: Workflow) -> WorkflowTask {
        WorkflowTask {
            sub_workflow: Some(Box::new(child)),
            ..leaf(id)
        }
    }

    fn root(tasks: Vec<WorkflowTask>) -> Workflow {
        Workflow {
            goal: "ship".into(),
            tasks,
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
