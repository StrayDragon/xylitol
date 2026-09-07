//! Session export/import — app-surface collaborator (HTML / JSONL).
//!
//! Held by [`XyInProcessDriver`](crate::app::core::driver::XyInProcessDriver).
//! Pure transformations over loaded session entries; file I/O via [`XyExportIo`].

use std::sync::Arc;

use serde_json::Value;

use crate::app::core::driver_error::XyDriverError;
use crate::protocol::error::XySessionStoreError;
use crate::protocol::ports::{XyExportIo, XySessionStore};
use crate::protocol::session::{MessageEntry, SessionEntry, message_role};
use crate::utils::xml_escape;

/// Format a unix-ms session timestamp as RFC3339 for display (v6: RFC3339 is
/// a display-side concern only; the on-disk format is unix-ms).
fn format_unix_ms(ms: u64) -> String {
    time::OffsetDateTime::from_unix_timestamp((ms / 1000) as i64)
        .map(|dt| dt.format(&time::format_description::well_known::Rfc3339))
        .ok()
        .and_then(|res| res.ok())
        .unwrap_or_else(|| ms.to_string())
}

/// Stateful export/import collaborator — owns the [`XyExportIo`] port.
///
/// Session store is borrowed per call (`&dyn XySessionStore` + session id).
pub struct SessionExporter {
    io: Option<Arc<dyn XyExportIo>>,
}

impl SessionExporter {
    /// Construct with an optional export I/O port (`None` ⇒ export unavailable).
    pub fn new(io: Option<Arc<dyn XyExportIo>>) -> Self {
        Self { io }
    }

    /// Export a session's entries to an HTML file. Returns the written path.
    pub async fn export_to_html(
        &self,
        store: &dyn XySessionStore,
        session_id: &str,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, XyDriverError> {
        let io = self
            .io
            .as_ref()
            .ok_or_else(|| XyDriverError::message("export io not configured"))?;
        let entries = store.load_entries(session_id).await?;
        let html = render_html(session_id, &entries);
        io.write_text(path, &html).await?;
        Ok(path.to_path_buf())
    }

    /// Export a session's entries as JSONL. Returns the written path.
    pub async fn export_to_jsonl(
        &self,
        store: &dyn XySessionStore,
        session_id: &str,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, XyDriverError> {
        let io = self
            .io
            .as_ref()
            .ok_or_else(|| XyDriverError::message("export io not configured"))?;
        let entries = store.load_entries(session_id).await?;
        let jsonl = render_jsonl(&entries)?;
        io.write_text(path, &jsonl).await?;
        Ok(path.to_path_buf())
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    ///
    /// The new session id is derived from the source header (re-used) to keep
    /// identities stable across export/import; the file lands without
    /// overwriting an existing session.
    pub async fn import_from_jsonl(
        &self,
        store: &dyn XySessionStore,
        path: &std::path::Path,
    ) -> Result<String, XyDriverError> {
        let io = self
            .io
            .as_ref()
            .ok_or_else(|| XyDriverError::message("export io not configured"))?;
        let bytes = io.read_bytes(path).await?;
        let entries = parse_jsonl(&bytes)?;
        let new_id = match entries.first() {
            Some(SessionEntry::Header(h)) => h.id.clone(),
            _ => {
                return Err(XyDriverError::invalid_input("import: missing header"));
            }
        };
        if store.exists(&new_id).await {
            return Err(XyDriverError::invalid_input(format!(
                "session already exists: {new_id}"
            )));
        }
        for entry in &entries {
            store.append_session_entry(&new_id, entry).await?;
        }
        Ok(new_id)
    }
}

/// Render a session's entries to a standalone HTML document.
///
/// Each entry is rendered to a readable block; messages and bash executions are
/// shown with their content, other entries get a one-line summary. ANSI escapes
/// are passed through (no color conversion in this minimal implementation).
pub fn render_html(session_id: &str, entries: &[SessionEntry]) -> String {
    let mut body = String::new();
    for entry in entries {
        body.push_str(&render_entry_html(entry));
    }
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
         <title>session {sid}</title>\
         <style>\
         body{{font-family:system-ui,sans-serif;margin:2em;}}\
         .entry{{margin:0 0 1.2em;padding:0.8em;border-left:3px solid #ccc;background:#fafafa;}}\
         .entry.message{{border-color:#3b82f6;}}\
         .entry.bash{{border-color:#f59e0b;}}\
         .entry.compaction{{border-color:#a855f7;}}\
         .kind{{font-size:0.75em;text-transform:uppercase;letter-spacing:0.05em;color:#666;margin-bottom:0.4em;}}\
         pre{{white-space:pre-wrap;word-break:break-word;margin:0;}}\
         </style></head><body>\n{body}\n</body></html>",
        sid = xml_escape(session_id),
        body = body,
    )
}

fn render_entry_html(entry: &SessionEntry) -> String {
    match entry {
        SessionEntry::Header(h) => block(
            "header",
            &format!(
                "session {} (v{}) @ {}",
                h.id,
                h.version,
                format_unix_ms(h.timestamp)
            ),
        ),
        SessionEntry::Message(m) if message_role(&m.message) == Some("bashExecution") => {
            let cmd = m
                .message
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let out = m
                .message
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            block("bash", &format!("$ {cmd}\n{out}"))
        }
        SessionEntry::Message(m) => block("message", &render_message(m)),
        SessionEntry::Compaction(c) => block(
            "compaction",
            &format!("[compaction summary]\n{}", c.summary),
        ),
        SessionEntry::BranchSummary(b) => block(
            "compaction",
            &format!("[branch summary from {}]\n{}", b.from_id, b.summary),
        ),
        SessionEntry::ModelChange(mc) => block(
            "header",
            &format!("model → {}:{}", mc.provider, mc.model_id),
        ),
        SessionEntry::ThinkingLevelChange(tc) => {
            block("header", &format!("thinking → {}", tc.thinking_level))
        }
        SessionEntry::CustomMessage(cm) => block(
            "message",
            &format!("[custom:{}]\n{}", cm.custom_type, cm.content),
        ),
        SessionEntry::Custom(c) => {
            if c.custom_type == crate::protocol::session::CUSTOM_TYPE_AGENT_TODO {
                block("header", &format!("[custom:agent_todo]\n{}", c.data))
            } else {
                block("header", &format!("[custom:{}]", c.custom_type))
            }
        }
        SessionEntry::Label(l) => block("header", &format!("[label → {}]", l.target_id)),
        SessionEntry::SessionInfo(si) => {
            block("header", &format!("[session info: name={:?}]", si.name))
        }
    }
}

fn block(kind: &str, content: &str) -> String {
    format!(
        "<div class=\"entry {kind}\"><div class=\"kind\">{kind}</div><pre>{content}</pre></div>\n",
        kind = kind,
        content = xml_escape(content),
    )
}

fn render_message(m: &MessageEntry) -> String {
    let role = m
        .message
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let text = message_text(&m.message);
    format!("{role}: {text}")
}

fn message_text(msg: &Value) -> String {
    crate::protocol::session::message_text(msg)
}

/// Render a session's entries as JSONL (one JSON object per line).
pub fn render_jsonl(entries: &[SessionEntry]) -> Result<String, XyDriverError> {
    let mut out = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(XySessionStoreError::from)?;
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

/// Parse JSONL bytes into session entries.
///
/// Validates that the first non-empty line is a session header carrying a
/// compatible `version`. Returns an error otherwise.
pub fn parse_jsonl(bytes: &[u8]) -> Result<Vec<SessionEntry>, XyDriverError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| XyDriverError::invalid_input(format!("jsonl is not utf-8: {e}")))?;
    let entries = crate::protocol::session::parse_session_jsonl(text)?;
    if entries.is_empty() {
        return Err(XyDriverError::invalid_input("jsonl contained no entries"));
    }
    // The first entry must be a Header.
    if !matches!(entries.first(), Some(SessionEntry::Header(_))) {
        return Err(XyDriverError::invalid_input(format!(
            "import: first entry must be a session header, got {:?}",
            entries.first().map(|e| e.entry_type()).unwrap_or("none")
        )));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{
        EntryBase, SESSION_VERSION, SessionHeader, bash_execution_message_entry,
    };

    fn header(id: &str) -> SessionEntry {
        SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: id.into(),
            timestamp: 1_781_827_200_000, // 2026-06-19T00:00:00Z (unix-ms)
            cwd: "/tmp".into(),
            parent_session: None,
            fork_at_entry_id: None,
        })
    }

    fn base() -> EntryBase {
        EntryBase {
            entry_type: "message".into(),
            id: "e1".into(),
            parent_id: None,
            timestamp: 1_781_827_200_000, // 2026-06-19T00:00:00Z (unix-ms)
        }
    }

    fn message(role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: base(),
            message: crate::protocol::session::fixture_message_json(role, text),
        })
    }

    #[test]
    fn html_renders_message_and_escapes() {
        let entries = vec![header("s1"), message("user", "hi <script>")];
        let html = render_html("s1", &entries);
        assert!(html.contains("user: hi &lt;script&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("<!doctype html>"));
    }

    #[test]
    fn html_renders_bash_entry() {
        let entries = vec![
            header("s1"),
            bash_execution_message_entry("echo hi", "hi", Some(0), false, false, None, false),
        ];
        let html = render_html("s1", &entries);
        assert!(html.contains("$ echo hi"));
        assert!(html.contains("hi"));
        assert!(html.contains("bash"));
    }

    #[test]
    fn jsonl_roundtrip_preserves_entries() {
        let entries = vec![
            header("s1"),
            message("user", "hello"),
            message("assistant", "world"),
        ];
        let jsonl = render_jsonl(&entries).unwrap();
        assert_eq!(jsonl.lines().count(), 3);

        let parsed = parse_jsonl(jsonl.as_bytes()).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], SessionEntry::Header(_)));
        assert!(matches!(parsed[1], SessionEntry::Message(_)));
    }

    #[test]
    fn jsonl_rejects_v5_disk_import() {
        // c2260: old v5 disk must not be imported. A real v5 file carries string
        // timestamps, so its header fails v6 deserialization → refused (no header).
        let v5 = r#"{"type":"session","version":5,"id":"old-s1","timestamp":"2024-01-01T00:00:00Z","cwd":"/tmp"}
{"type":"message","id":"m1","parentId":null,"timestamp":"2024-01-01T00:00:01Z","message":{"role":"user","content":[{"type":"text","text":"hi"}],"timestamp":1704067200000}}
"#;
        let err = parse_jsonl(v5.as_bytes()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no header"),
            "real v5 disk must be refused: {msg}"
        );

        // A version-tagged header that still parses (numeric ms) is refused with
        // `require 6` — the actionable message.
        let v5_numeric = r#"{"type":"session","version":5,"id":"old-s2","timestamp":1704067200000,"cwd":"/tmp"}
"#;
        let err = parse_jsonl(v5_numeric.as_bytes()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("require 6"),
            "must name current version: {msg}"
        );
    }

    #[test]
    fn jsonl_rejects_missing_header() {
        // A message entry alone (no header) must be rejected.
        let only_message = serde_json::to_string(&message("user", "x")).unwrap();
        let res = parse_jsonl(only_message.as_bytes());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("header"));
    }

    #[test]
    fn jsonl_rejects_empty() {
        assert!(parse_jsonl(b"   \n\n").is_err());
    }
}
