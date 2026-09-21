//! Session environment — **status-bar family** bootstrap (c1905).
//!
//! This is **not** a chat user turn. It is a sparse, append-only **special status
//! bar type** that bootstraps `date` / `clock` / `cwd` before the real user
//! message. Outbound `<agent_status_bar>` (Todo as `<todo>` child) is
//! [`crate::agent::prompt::status_bar`] — not persisted.
//!
//! # Discovery for `c1895` (do not reinvent)
//!
//! | Need | Use |
//! |---|---|
//! | Discriminator | [`CUSTOM_TYPE_SESSION_ENV`] (`Env` `CustomMessage.custom_type`) |
//! | Wire / body root | [`SESSION_ENV_XML_ROOT`] → `<session_env>…</session_env>` |
//! | Latest snapshot in history | [`last_session_env`] / [`session_env_from_message`] |
//! | Whether to append another row | [`should_append_session_env`] (date **or** cwd change; clock alone ≠ append) |
//! | Mutating ensure | [`ensure_session_env_in_history`] — append when missing/stale (c1906) |
//! | Inject seams | ReAct before user persist; overflow reload; post-`compact_session` |
//! | Outbound status bar | [`crate::agent::prompt::project_outbound`] — `<agent_status_bar>`; Todo is `<todo>` child |
//! | TUI hide | [`crate::protocol::session::is_env_custom_message`] + tree kind `meta` |
//! | Planned persist kind | still park (`c1895`); outbound root is already [`crate::agent::prompt::status_bar`] |
//!
//! Persisted as [`EnvMessage::CustomMessage`], folded to a **user** row by
//! [`crate::agent::llm_project::project_for_llm`]. Stable XML body: edit only via
//! explicit change (prefix / cache sensitive).
//!
//! Design SSOT: `llmanspec/changes/archive/2026-08-10-c1905-update-system-prompt-stable-volatile-split/design.md`
//! Downstream bar: `…/c1895-add-agent-status-bar-subsystem/proposal.md`
//! Downstream compact: `…/c1906-ensure-session-env-after-compaction/`

use serde_json::json;

use crate::protocol::message::{AgentMessage, EnvMessage};
use crate::utils::xml_escape;

/// `CustomMessage.custom_type` for the session-env status-bar bootstrap.
///
/// Same string as the XML root ([`SESSION_ENV_XML_ROOT`]).
pub const CUSTOM_TYPE_SESSION_ENV: &str = "session_env";

/// XML root element for session-env body (sparse persist; c1905).
///
/// Outbound bar root is [`crate::agent::prompt::status_bar::STATUS_BAR_XML_ROOT`].
pub const SESSION_ENV_XML_ROOT: &str = "session_env";

/// Snapshot used to build or compare session_env messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEnvSnapshot {
    /// Calendar day `YYYY-MM-DD` (UTC unless caller overrides).
    pub date: String,
    /// RFC3339 / ISO-ish clock for model time sense (not written into system).
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

    /// XML body projected to the model (and stored in CustomMessage content string).
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
                // Hint for c1895 scanners: this row is status-bar family bootstrap.
                "status_bar_kind": CUSTOM_TYPE_SESSION_ENV,
            }),
        })
    }
}

/// Best-effort parse of the latest session_env snapshot from history (scan reverse).
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

/// Whether a new session_env row should be appended before the next user turn.
///
/// Compares **date** and **cwd** only (clock may refresh without forcing a new row
/// when day+cwd unchanged — avoids per-turn prefix noise; live clock → c1895).
pub fn should_append_session_env(history: &[AgentMessage], next: &SessionEnvSnapshot) -> bool {
    match last_session_env(history) {
        None => true,
        Some(prev) => prev.date != next.date || prev.cwd != next.cwd,
    }
}

/// Append a session_env row when history lacks one aligned to `next` (date+cwd).
///
/// Returns `true` if a row was pushed. Callers that own a store SHOULD persist
/// the new last message when this returns true (ReAct / overflow / compact).
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

/// Build a fresh snapshot for `cwd` (UTC now).
pub fn snapshot_for_cwd(cwd: impl Into<String>) -> SessionEnvSnapshot {
    SessionEnvSnapshot::from_utc_now(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_roundtrip_via_details_and_parse() {
        let s = SessionEnvSnapshot {
            date: "2026-08-10".into(),
            clock: "2026-08-10T09:00:00Z".into(),
            cwd: "/tmp/x".into(),
        };
        let msg = s.to_agent_message();
        let parsed = session_env_from_message(&msg).expect("parse");
        assert_eq!(parsed.date, s.date);
        assert_eq!(parsed.cwd, s.cwd);
        assert_eq!(parsed.clock, s.clock);
        assert!(s.to_xml().contains(&format!("<{SESSION_ENV_XML_ROOT}>")));
    }

    #[test]
    fn append_when_missing_or_day_or_cwd_changes() {
        let a = SessionEnvSnapshot {
            date: "2026-08-10".into(),
            clock: "2026-08-10T01:00:00Z".into(),
            cwd: "/a".into(),
        };
        assert!(should_append_session_env(&[], &a));
        let hist = vec![a.to_agent_message()];
        let same_day = SessionEnvSnapshot {
            clock: "2026-08-10T23:00:00Z".into(),
            ..a.clone()
        };
        assert!(!should_append_session_env(&hist, &same_day));
        let next_day = SessionEnvSnapshot {
            date: "2026-08-11".into(),
            clock: "2026-08-11T00:01:00Z".into(),
            cwd: "/a".into(),
        };
        assert!(should_append_session_env(&hist, &next_day));
        let new_cwd = SessionEnvSnapshot {
            cwd: "/b".into(),
            ..a.clone()
        };
        assert!(should_append_session_env(&hist, &new_cwd));
    }

    #[test]
    fn ensure_appends_when_missing_or_stale() {
        let next = SessionEnvSnapshot {
            date: "2026-08-10".into(),
            clock: "2026-08-10T12:00:00Z".into(),
            cwd: "/proj".into(),
        };
        let mut hist = Vec::new();
        assert!(ensure_session_env_in_history(&mut hist, &next));
        assert_eq!(hist.len(), 1);
        assert!(!ensure_session_env_in_history(&mut hist, &next));
        assert_eq!(hist.len(), 1);

        // Simulate compact cut dropping early env.
        hist.clear();
        hist.push(AgentMessage::user("kept after compact"));
        assert!(ensure_session_env_in_history(&mut hist, &next));
        assert!(last_session_env(&hist).is_some());

        let stale_cwd = SessionEnvSnapshot {
            cwd: "/other".into(),
            ..next.clone()
        };
        assert!(ensure_session_env_in_history(&mut hist, &stale_cwd));
        assert_eq!(last_session_env(&hist).unwrap().cwd, "/other");
    }
}
