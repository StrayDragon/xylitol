use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::entries::{CustomEntry, EntryBase, SessionEntry};
use super::todo::TodoStatus;

pub const CUSTOM_TYPE_PLAN_DOC: &str = "plan_doc";
pub const CUSTOM_TYPE_PLAN_REF: &str = "plan_ref";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanScope {
    #[default]
    Session,
    Project,
    Global,
}

impl PlanScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRef {
    pub scope: PlanScope,
    pub plan_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl PlanRef {
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
            custom_type: CUSTOM_TYPE_PLAN_REF.into(),
            data: self.to_data_value(),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanDocStatus {
    #[default]
    Draft,
    AwaitingApproval,
    Approved,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDocStep {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default)]
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDoc {
    pub plan_id: String,
    pub title: String,
    #[serde(default)]
    pub status: PlanDocStatus,
    #[serde(default)]
    pub scope: PlanScope,
    #[serde(default)]
    pub steps: Vec<PlanDocStep>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanDocValidationError(pub String);

impl std::fmt::Display for PlanDocValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PlanDocValidationError {}

impl PlanDoc {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn step(&self, id: &str) -> Option<&PlanDocStep> {
        self.steps.iter().find(|step| step.id == id)
    }

    pub fn to_data_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn from_data_value(data: &Value) -> Option<Self> {
        serde_json::from_value(data.clone()).ok()
    }

    pub fn validate(&self) -> Result<(), PlanDocValidationError> {
        if self.plan_id.trim().is_empty() {
            return Err(PlanDocValidationError("plan_id must be non-empty".into()));
        }
        if self.steps.is_empty() {
            return Err(PlanDocValidationError(
                "plan must have at least one step".into(),
            ));
        }
        let mut ids = HashSet::new();
        for step in &self.steps {
            if step.id.trim().is_empty() {
                return Err(PlanDocValidationError(
                    "plan step id must be non-empty".into(),
                ));
            }
            if !ids.insert(step.id.clone()) {
                return Err(PlanDocValidationError(format!(
                    "duplicate plan step id: {}",
                    step.id
                )));
            }
        }
        for step in &self.steps {
            for dependency in &step.depends_on {
                if dependency == &step.id {
                    return Err(PlanDocValidationError(format!(
                        "plan step {} depends on itself",
                        step.id
                    )));
                }
                if !ids.contains(dependency) {
                    return Err(PlanDocValidationError(format!(
                        "plan step {} depends on unknown step {}",
                        step.id, dependency
                    )));
                }
            }
        }
        detect_cycle(self)?;
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        let mut out = format!("# {}\n\n", self.title.trim());
        out.push_str(&format!("- plan_id: `{}`\n", self.plan_id));
        out.push_str(&format!("- scope: `{}`\n", self.scope.label()));
        out.push_str(&format!("- status: `{}`\n", status_label(self.status)));
        out.push_str(&format!("- updated_at: `{}`\n\n", self.updated_at));
        for step in &self.steps {
            let mark = match step.status {
                TodoStatus::Completed => "x",
                TodoStatus::InProgress => "~",
                TodoStatus::Pending => " ",
            };
            out.push_str(&format!("- [{mark}] `{}` {}\n", step.id, step.content));
            if !step.depends_on.is_empty() {
                out.push_str(&format!("  - depends on: {}\n", step.depends_on.join(", ")));
            }
            if let Some(verification) = &step.verification {
                out.push_str(&format!("  - verify: {verification}\n"));
            }
        }
        out
    }

    pub fn to_file_markdown(&self) -> Result<String, String> {
        let frontmatter = yaml_serde::to_string(self).map_err(|e| e.to_string())?;
        Ok(format!("---\n{}---\n\n{}", frontmatter, self.to_markdown()))
    }

    pub fn from_file_markdown(content: &str) -> Result<Self, String> {
        let rest = content
            .strip_prefix("---\n")
            .ok_or_else(|| "plan file is missing frontmatter".to_string())?;
        let (frontmatter, _body) = rest
            .split_once("\n---\n")
            .ok_or_else(|| "plan file frontmatter is not closed".to_string())?;
        let doc: Self = yaml_serde::from_str(frontmatter).map_err(|e| e.to_string())?;
        if doc.plan_id.trim().is_empty() {
            return Err("plan_id must be non-empty".into());
        }
        doc.validate().map_err(|e| e.0)?;
        Ok(doc)
    }

    pub fn to_custom_entry_shell(&self) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_PLAN_DOC.into(),
            data: self.to_data_value(),
        })
    }
}

fn status_label(status: PlanDocStatus) -> &'static str {
    match status {
        PlanDocStatus::Draft => "draft",
        PlanDocStatus::AwaitingApproval => "awaiting_approval",
        PlanDocStatus::Approved => "approved",
        PlanDocStatus::Abandoned => "abandoned",
    }
}

fn detect_cycle(plan: &PlanDoc) -> Result<(), PlanDocValidationError> {
    fn visit(
        id: &str,
        plan: &PlanDoc,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), PlanDocValidationError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.to_string()) {
            return Err(PlanDocValidationError(format!(
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

pub fn latest_plan_doc(entries: &[SessionEntry]) -> Option<PlanDoc> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_DOC => {
            PlanDoc::from_data_value(&custom.data)
        }
        _ => None,
    })
}

pub fn latest_plan_doc_by_id(entries: &[SessionEntry], plan_id: &str) -> Option<PlanDoc> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_DOC => {
            let doc = PlanDoc::from_data_value(&custom.data)?;
            (doc.plan_id == plan_id).then_some(doc)
        }
        _ => None,
    })
}

pub fn latest_plan_ref(entries: &[SessionEntry]) -> Option<PlanRef> {
    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Custom(custom) if custom.custom_type == CUSTOM_TYPE_PLAN_REF => {
            PlanRef::from_data_value(&custom.data)
        }
        _ => None,
    })
}

pub fn plan_docs(entries: &[SessionEntry]) -> Vec<PlanDoc> {
    let mut by_id: HashMap<String, PlanDoc> = HashMap::new();
    for entry in entries {
        let SessionEntry::Custom(custom) = entry else {
            continue;
        };
        if custom.custom_type != CUSTOM_TYPE_PLAN_DOC {
            continue;
        }
        let Some(doc) = PlanDoc::from_data_value(&custom.data) else {
            continue;
        };
        by_id.insert(doc.plan_id.clone(), doc);
    }
    let mut docs: Vec<PlanDoc> = by_id.into_values().collect();
    docs.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then(a.plan_id.cmp(&b.plan_id))
    });
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, fixture_message_json};

    fn step(id: &str, status: TodoStatus, depends_on: &[&str]) -> PlanDocStep {
        PlanDocStep {
            id: id.into(),
            content: format!("do {id}"),
            depends_on: depends_on.iter().map(|d| d.to_string()).collect(),
            verification: Some(format!("verify {id}")),
            status,
        }
    }

    fn doc(plan_id: &str, steps: Vec<PlanDocStep>) -> PlanDoc {
        PlanDoc {
            plan_id: plan_id.into(),
            title: format!("Plan {plan_id}"),
            status: PlanDocStatus::Draft,
            scope: PlanScope::Session,
            steps,
            created_at: 1,
            updated_at: 2,
        }
    }

    fn message(id: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            message: fixture_message_json("user", id),
        })
    }

    #[test]
    fn round_trip_and_latest_per_id() {
        let first = doc("p1", vec![step("s1", TodoStatus::Pending, &[])]);
        let mut second = doc("p1", vec![step("s1", TodoStatus::Completed, &[])]);
        second.updated_at = 3;
        let other = doc("p2", vec![step("a", TodoStatus::Pending, &[])]);
        let entries = vec![
            first.to_custom_entry_shell(),
            message("m1"),
            second.to_custom_entry_shell(),
            other.to_custom_entry_shell(),
        ];
        assert_eq!(
            latest_plan_doc(&entries).map(|d| d.plan_id),
            Some("p2".into())
        );
        assert_eq!(
            latest_plan_doc_by_id(&entries, "p1").map(|d| d.updated_at),
            Some(3)
        );
        let listed = plan_docs(&entries);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].plan_id, "p1");
    }

    #[test]
    fn validation_rejects_duplicate_missing_and_cycles() {
        assert!(
            doc(
                "p1",
                vec![
                    step("s1", TodoStatus::Pending, &[]),
                    step("s1", TodoStatus::Pending, &[]),
                ]
            )
            .validate()
            .is_err()
        );
        assert!(
            doc("p1", vec![step("s1", TodoStatus::Pending, &["nope"])])
                .validate()
                .is_err()
        );
        assert!(
            doc(
                "p1",
                vec![
                    step("s1", TodoStatus::Pending, &["s2"]),
                    step("s2", TodoStatus::Pending, &["s1"]),
                ]
            )
            .validate()
            .is_err()
        );
        assert!(
            doc("", vec![step("s1", TodoStatus::Pending, &[])])
                .validate()
                .is_err()
        );
        assert!(doc("p1", Vec::new()).validate().is_err());
        assert!(
            doc("p1", vec![step("s1", TodoStatus::Pending, &[])])
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn markdown_renders_steps_and_metadata() {
        let md = doc(
            "p1",
            vec![
                step("s1", TodoStatus::Completed, &[]),
                step("s2", TodoStatus::InProgress, &["s1"]),
            ],
        )
        .to_markdown();
        assert!(md.contains("# Plan p1"));
        assert!(md.contains("- plan_id: `p1`"));
        assert!(md.contains("- [x] `s1` do s1"));
        assert!(md.contains("- [~] `s2` do s2"));
        assert!(md.contains("depends on: s1"));
    }

    #[test]
    fn plan_ref_round_trip_and_latest() {
        let first = PlanRef {
            scope: PlanScope::Project,
            plan_id: "p1".into(),
            path: Some("/repo/.xylitol/plans/p1.md".into()),
        };
        let second = PlanRef {
            scope: PlanScope::Global,
            plan_id: "p2".into(),
            path: None,
        };
        let entries = vec![
            first.to_custom_entry_shell(),
            message("m1"),
            second.to_custom_entry_shell(),
        ];
        assert_eq!(
            latest_plan_ref(&entries).map(|r| (r.scope, r.plan_id)),
            Some((PlanScope::Global, "p2".into()))
        );
    }

    #[test]
    fn file_markdown_round_trip_and_rejects_bad_frontmatter() {
        let doc = doc("p1", vec![step("s1", TodoStatus::Pending, &[])]);
        let content = doc.to_file_markdown().expect("render");
        assert!(content.starts_with("---\n"));
        assert!(content.contains("plan_id: p1"));
        let parsed = PlanDoc::from_file_markdown(&content).expect("parse");
        assert_eq!(parsed, doc);
        assert!(PlanDoc::from_file_markdown("# no frontmatter").is_err());
        assert!(PlanDoc::from_file_markdown("---\ntitle: x\n---\n").is_err());
    }
}
