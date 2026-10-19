//! Session environment — **status-bar family** bootstrap (c1905).
//!
//! This is **not** a chat user turn. It is a sparse, append-only **special status
//! bar type** that bootstraps `date` / `clock` / `cwd` before the real user
//! message. Outbound `<agent_status_bar>` (Todo as `<todo>` child) is
//! `crate::agent::prompt::status_bar` — not persisted.
//!
//! SSOT: [`xylitol_plugin_sdk::session_env`].

pub use xylitol_plugin_sdk::session_env::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::AgentMessage;

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
