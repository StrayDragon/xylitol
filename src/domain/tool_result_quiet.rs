//! Quiet write/edit tool results for LLM history (c1310 / t22).
//!
//! Raw execute strings may still carry UI fields (`display_diff`, etc.). Session
//! history content becomes a short success sentence; full payload goes to
//! `details` (providers map content only).

use serde_json::Value;

use super::message::AgentPart;

/// Split a tool execute preview into LLM-facing content + optional UI details.
pub fn quiet_write_edit_for_history(
    name: &str,
    raw: &str,
    is_error: bool,
) -> (Vec<AgentPart>, Option<Value>) {
    if is_error {
        return (vec![AgentPart::text(raw.to_string())], None);
    }
    match name {
        "write" => quiet_write(raw),
        "edit" => quiet_edit(raw),
        _ => (vec![AgentPart::text(raw.to_string())], None),
    }
}

fn quiet_write(raw: &str) -> (Vec<AgentPart>, Option<Value>) {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return (vec![AgentPart::text(raw.to_string())], None);
    };
    if v.get("success").and_then(Value::as_bool) != Some(true) {
        return (vec![AgentPart::text(raw.to_string())], Some(v));
    }
    let path = v.get("path").and_then(Value::as_str).unwrap_or("");
    let msg = match v.get("bytes").and_then(Value::as_u64) {
        Some(n) => format!("Successfully wrote {n} bytes to {path}"),
        None => format!("Successfully wrote to {path}"),
    };
    (vec![AgentPart::text(msg)], Some(v))
}

fn quiet_edit(raw: &str) -> (Vec<AgentPart>, Option<Value>) {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return (vec![AgentPart::text(raw.to_string())], None);
    };
    if v.get("success").and_then(Value::as_bool) != Some(true) {
        return (vec![AgentPart::text(raw.to_string())], Some(v));
    }
    let path = v.get("path").and_then(Value::as_str).unwrap_or("");
    let n = v.get("edit_count").and_then(Value::as_u64).unwrap_or(1);
    let msg = format!("Successfully replaced {n} block(s) in {path}.");
    (vec![AgentPart::text(msg)], Some(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_success_short_keeps_details() {
        let raw = json!({"success": true, "path": "a.md", "bytes": 12}).to_string();
        let (parts, details) = quiet_write_edit_for_history("write", &raw, false);
        assert_eq!(
            parts[0].as_text().unwrap(),
            "Successfully wrote 12 bytes to a.md"
        );
        assert!(details.unwrap().get("success").is_some());
    }

    #[test]
    fn edit_success_short_keeps_display_diff_in_details() {
        let raw = json!({
            "success": true,
            "path": "a.rs",
            "edit_count": 2,
            "display_diff": "1 1 | x",
            "diff": "@@"
        })
        .to_string();
        let (parts, details) = quiet_write_edit_for_history("edit", &raw, false);
        let text = parts[0].as_text().unwrap();
        assert!(text.contains("Successfully replaced 2 block(s)"));
        assert!(!text.contains("display_diff"));
        assert_eq!(
            details.unwrap().get("display_diff").and_then(Value::as_str),
            Some("1 1 | x")
        );
    }

    #[test]
    fn error_passthrough() {
        let (parts, details) = quiet_write_edit_for_history("edit", "exact match failed", true);
        assert_eq!(parts[0].as_text().unwrap(), "exact match failed");
        assert!(details.is_none());
    }
}
