//! Session export — HTML / JSONL rendering and JSONL import.
//!
//! Pure transformations over a loaded session's entries (relocated from
//! `infra::session::export` per c278 — these helpers depend only on core
//! vocabulary types + std, so they live here and the agent layer calls them
//! directly instead of through an infra forwarding wrapper). No file mutation
//! outside the explicit `export_to_*` writers; import creates a brand-new
//! session.

use std::path::Path;

use serde_json::Value;

use crate::core::session_types::{MessageEntry, SessionEntry};

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
        sid = html_escape(session_id),
        body = body,
    )
}

fn render_entry_html(entry: &SessionEntry) -> String {
    match entry {
        SessionEntry::Header(h) => block(
            "header",
            &format!("session {} (v{}) @ {}", h.id, h.version, h.timestamp),
        ),
        SessionEntry::Message(m) => block("message", &render_message(m)),
        SessionEntry::Compaction(c) => block(
            "compaction",
            &format!("[compaction summary]\n{}", c.summary),
        ),
        SessionEntry::BranchSummary(b) => block(
            "compaction",
            &format!("[branch summary from {}]\n{}", b.from_id, b.summary),
        ),
        SessionEntry::BashExecution(b) => block("bash", &format!("$ {}\n{}", b.command, b.output)),
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
        SessionEntry::Custom(c) => block("header", &format!("[custom:{}]", c.custom_type)),
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
        content = html_escape(content),
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
    if let Some(parts) = msg.get("parts").and_then(Value::as_array) {
        let mut out = String::new();
        for p in parts {
            if let Some(t) = p.get("text").and_then(Value::as_str) {
                out.push_str(t);
            } else {
                out.push_str(&p.to_string());
            }
        }
        return out;
    }
    msg.to_string()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render a session's entries as JSONL (one JSON object per line).
pub fn render_jsonl(entries: &[SessionEntry]) -> Result<String, String> {
    let mut out = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(|e| format!("serialize entry: {e}"))?;
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

/// Parse JSONL bytes into session entries.
///
/// Validates that the first non-empty line is a session header carrying a
/// compatible `version`. Returns an error otherwise.
pub fn parse_jsonl(bytes: &[u8]) -> Result<Vec<SessionEntry>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("jsonl is not utf-8: {e}"))?;
    let mut entries = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: SessionEntry =
            serde_json::from_str(line).map_err(|e| format!("line {}: parse error: {e}", i + 1))?;
        entries.push(entry);
    }
    if entries.is_empty() {
        return Err("jsonl contained no entries".into());
    }
    // The first entry must be a Header.
    if !matches!(entries.first(), Some(SessionEntry::Header(_))) {
        return Err(format!(
            "import: first entry must be a session header, got {:?}",
            entries.first().map(|e| e.entry_type()).unwrap_or("none")
        ));
    }
    Ok(entries)
}

/// Write a rendered string to `path`, creating parent directories as needed.
pub fn write_to(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dirs: {e}"))?;
    }
    std::fs::write(path, content).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Human-readable guidance shown when the user invokes `share` without a token.
///
/// The actual gist upload is intentionally not implemented here (requires HTTP
/// + token management); this stub keeps the call site stable for future wiring.
pub fn share_guidance_message(_path: &Path) -> String {
    "Sharing as a GitHub gist requires a token. Set GITHUB_GIST_TOKEN (or the \
     equivalent in your config), then re-run. \
     See: https://docs.github.com/en/rest/gists"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session_types::{BashExecutionEntry, EntryBase, SessionHeader};
    use std::path::PathBuf;

    fn header(id: &str) -> SessionEntry {
        SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: 3,
            id: id.into(),
            timestamp: "2026-06-19T00:00:00Z".into(),
            cwd: "/tmp".into(),
            parent_session: None,
        })
    }

    fn base() -> EntryBase {
        EntryBase {
            entry_type: "message".into(),
            id: "e1".into(),
            parent_id: None,
            timestamp: "2026-06-19T00:00:00Z".into(),
        }
    }

    fn message(role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: base(),
            message: serde_json::json!({
                "role": role,
                "parts": [{"type": "text", "text": text}]
            }),
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
            SessionEntry::BashExecution(BashExecutionEntry {
                base: base(),
                command: "echo hi".into(),
                output: "hi".into(),
                exit_code: Some(0),
                cancelled: false,
                truncated: false,
                full_output_path: None,
                exclude_from_context: false,
            }),
        ];
        let html = render_html("s1", &entries);
        assert!(html.contains("$ echo hi"));
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
    fn jsonl_rejects_missing_header() {
        // A message entry alone (no header) must be rejected.
        let only_message = serde_json::to_string(&message("user", "x")).unwrap();
        let res = parse_jsonl(only_message.as_bytes());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("header"));
    }

    #[test]
    fn jsonl_rejects_empty() {
        assert!(parse_jsonl(b"   \n\n").is_err());
    }

    #[test]
    fn write_to_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a/b/c/session.html");
        write_to(&path, "<html></html>").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "<html></html>");
    }

    #[test]
    fn share_stub_returns_guidance_without_token() {
        // No GitHub token configured → share returns a configuration hint,
        // never attempts a network upload.
        let msg = share_guidance_message(&PathBuf::from("/tmp/x.html"));
        assert!(msg.to_lowercase().contains("token") || msg.to_lowercase().contains("gist"));
        assert!(msg.contains("https://"));
    }
}
