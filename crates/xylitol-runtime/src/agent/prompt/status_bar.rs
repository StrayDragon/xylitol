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

#[allow(unused_imports)] // re-export for docs / call sites that name the XML root
pub use xylitol_plugin_sdk::status_bar::{AgentStatusBar, STATUS_BAR_XML_ROOT, project_outbound};

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::message::AgentMessage;
    use xylitol_plugin_sdk::session::{TodoItem, TodoList, TodoStatus};

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
