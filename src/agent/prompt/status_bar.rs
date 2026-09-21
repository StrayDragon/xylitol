//! Agent status bar — code-maintained readings projected after history fold.
//!
//! Sibling of [`super::session_env`]: that module **persists** a sparse
//! `<session_env>` Env row into the cache prefix. This module **does not
//! persist**. It appends at most one user row whose root is
//! [`STATUS_BAR_XML_ROOT`].
//!
//! Todo is a **child** of that root (`<todo>`), not a standalone fake user
//! turn. SSOT stays `agent_todo` Custom. Parked c1895 persist / registry /
//! independent session kind are not this module.
//!
//! New readings: new fields on [`AgentStatusBar`] + a new arm in [`Self::render`].
//! Generate and token estimate share [`project_outbound`]; compact summarizer
//! stays on [`crate::agent::llm_project::project_for_llm`].

use crate::agent::llm_project::{project_for_llm, user_text};
use crate::protocol::message::{AgentMessage, LlmMessage};
use crate::protocol::session::{SessionEntry, TodoList, latest_agent_todo};
use crate::utils::xml_escape;

/// XML root for the outbound status-bar row.
pub const STATUS_BAR_XML_ROOT: &str = "agent_status_bar";

/// Code-maintained readings for one outbound generate / estimate.
///
/// Add a field when a new reading exists. Do not add `project_for_llm_with_*`.
/// Do not put these on `EstimateOpts`.
#[derive(Debug, Clone, Default)]
pub struct AgentStatusBar {
    pub todo: Option<TodoList>,
}

impl AgentStatusBar {
    pub fn from_session_entries(entries: &[SessionEntry]) -> Self {
        Self {
            todo: latest_agent_todo(entries).filter(|list| !list.is_empty()),
        }
    }

    /// Empty bar omits the row. Exhaustive on fields so a second reading cannot
    /// silently become another user turn.
    pub fn render(&self) -> Option<String> {
        let todo_xml = match &self.todo {
            None => return None,
            Some(list) if list.is_empty() => return None,
            Some(list) => render_todo(list),
        };
        Some(format!(
            "<{root}>\n{todo_xml}</{root}>",
            root = STATUS_BAR_XML_ROOT
        ))
    }
}

fn render_todo(list: &TodoList) -> String {
    let mut body = String::from("<todo>\n");
    for item in &list.items {
        body.push_str(&format!(
            "<item id=\"{}\" status=\"{}\">{}</item>\n",
            xml_escape(&item.id),
            item.status.as_str(),
            xml_escape(&item.content),
        ));
    }
    body.push_str("</todo>\n");
    body
}

/// History fold + status bar (generate and token estimate).
pub fn project_outbound(messages: &[AgentMessage], bar: &AgentStatusBar) -> Vec<LlmMessage> {
    let mut out = project_for_llm(messages);
    if let Some(body) = bar.render() {
        out.push(user_text(body));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{TodoItem, TodoStatus};

    #[test]
    fn empty_todo_adds_no_row() {
        let history = vec![AgentMessage::user("hi")];
        let projected = project_outbound(&history, &AgentStatusBar::default());
        assert_eq!(projected.len(), 1);
    }

    #[test]
    fn non_empty_todo_is_child_of_status_bar() {
        let history = vec![AgentMessage::user("hi")];
        let bar = AgentStatusBar {
            todo: Some(TodoList::new(vec![TodoItem {
                id: "t_abcd1234".into(),
                content: "a < b".into(),
                status: TodoStatus::InProgress,
            }])),
        };
        let projected = project_outbound(&history, &bar);
        let text = projected.last().expect("tail").text();
        assert!(text.contains("<agent_status_bar>"), "{text}");
        assert!(text.contains("<todo>"), "{text}");
        assert!(!text.contains("<agent_todo>"), "{text}");
        assert!(text.contains("id=\"t_abcd1234\""), "{text}");
        assert!(text.contains("a &lt; b"), "{text}");
        assert_eq!(projected.len(), 2);
    }
}
