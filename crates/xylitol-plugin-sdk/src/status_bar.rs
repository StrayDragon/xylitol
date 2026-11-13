use crate::llm_project::{project_for_llm, user_text};
use crate::message::{AgentMessage, XyLlmMessage};
use crate::session::{SessionEntry, TodoList, latest_agent_todo};

pub const STATUS_BAR_XML_ROOT: &str = "agent_status_bar";

#[derive(Debug, Clone, Default)]
pub struct AgentStatusBar {
    todo: Option<TodoList>,
}

impl AgentStatusBar {
    pub fn from_session_entries(entries: &[SessionEntry]) -> Self {
        Self {
            todo: latest_agent_todo(entries).filter(|list| !list.is_empty()),
        }
    }

    pub fn with_todo(list: TodoList) -> Self {
        Self {
            todo: (!list.is_empty()).then_some(list),
        }
    }

    pub fn todo(&self) -> Option<&TodoList> {
        self.todo.as_ref()
    }

    pub fn render(&self) -> Option<String> {
        let list = self.todo.as_ref()?;
        Some(format!(
            "<{root}>\n{todo_xml}</{root}>",
            root = STATUS_BAR_XML_ROOT,
            todo_xml = render_todo(list),
        ))
    }
}

fn xml_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
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

pub fn project_outbound(messages: &[AgentMessage], bar: &AgentStatusBar) -> Vec<XyLlmMessage> {
    let mut out = project_for_llm(messages);
    if let Some(body) = bar.render() {
        out.push(user_text(body));
    }
    out
}
