use serde_json::json;

use crate::message::{AgentMessage, EnvMessage};

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

pub const CUSTOM_TYPE_SESSION_ENV: &str = "session_env";

pub const SESSION_ENV_XML_ROOT: &str = "session_env";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEnvSnapshot {
    pub date: String,
    pub clock: String,
    pub cwd: String,
}

impl SessionEnvSnapshot {
    pub fn from_utc_now(cwd: impl Into<String>) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let clock = now
            .format(&time::format_description::well_known::Rfc3339)
            .expect("RFC3339 format is infallible for valid times");
        let date = now
            .format(&time::macros::format_description!("[year]-[month]-[day]"))
            .expect("calendar date format is infallible");
        Self {
            date,
            clock,
            cwd: cwd.into(),
        }
    }

    pub fn to_xml(&self) -> String {
        let root = SESSION_ENV_XML_ROOT;
        format!(
            "<{root}>\n  <date>{}</date>\n  <clock>{}</clock>\n  <cwd>{}</cwd>\n</{root}>",
            self.date,
            self.clock,
            xml_escape(&self.cwd),
        )
    }

    pub fn to_agent_message(&self) -> AgentMessage {
        AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: CUSTOM_TYPE_SESSION_ENV.into(),
            content: json!(self.to_xml()),
            display: json!(CUSTOM_TYPE_SESSION_ENV),
            details: json!({
                "date": self.date,
                "clock": self.clock,
                "cwd": self.cwd,
                "status_bar_kind": CUSTOM_TYPE_SESSION_ENV,
            }),
        })
    }
}

pub fn last_session_env(messages: &[AgentMessage]) -> Option<SessionEnvSnapshot> {
    for msg in messages.iter().rev() {
        if let Some(s) = session_env_from_message(msg) {
            return Some(s);
        }
    }
    None
}

pub fn session_env_from_message(msg: &AgentMessage) -> Option<SessionEnvSnapshot> {
    let AgentMessage::Env(EnvMessage::CustomMessage {
        custom_type,
        content,
        details,
        ..
    }) = msg
    else {
        return None;
    };
    if custom_type != CUSTOM_TYPE_SESSION_ENV {
        return None;
    }
    if let Some(obj) = details.as_object() {
        let date = obj.get("date")?.as_str()?.to_string();
        let clock = obj
            .get("clock")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cwd = obj.get("cwd")?.as_str()?.to_string();
        return Some(SessionEnvSnapshot { date, clock, cwd });
    }
    let text = match content {
        serde_json::Value::String(s) => s.as_str(),
        _ => return None,
    };
    parse_session_env_xml(text)
}

fn parse_session_env_xml(text: &str) -> Option<SessionEnvSnapshot> {
    let date = tag_value(text, "date")?;
    let cwd = tag_value(text, "cwd")?;
    let clock = tag_value(text, "clock").unwrap_or_default();
    Some(SessionEnvSnapshot {
        date: date.to_string(),
        clock: clock.to_string(),
        cwd: cwd.to_string(),
    })
}

fn tag_value<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim())
}

pub fn should_append_session_env(history: &[AgentMessage], next: &SessionEnvSnapshot) -> bool {
    match last_session_env(history) {
        None => true,
        Some(prev) => prev.date != next.date || prev.cwd != next.cwd,
    }
}

pub fn ensure_session_env_in_history(
    history: &mut Vec<AgentMessage>,
    next: &SessionEnvSnapshot,
) -> bool {
    if !should_append_session_env(history, next) {
        return false;
    }
    history.push(next.to_agent_message());
    true
}

pub fn snapshot_for_cwd(cwd: impl Into<String>) -> SessionEnvSnapshot {
    SessionEnvSnapshot::from_utc_now(cwd)
}
