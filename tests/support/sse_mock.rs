//! Minimal helpers for building wiremock SSE responses.

use serde_json::Value;

/// Build a single SSE frame.
///
/// Each line must end with `\n`; frames end with an extra `\n`.
pub(crate) fn sse_event(event: &str, data: &str) -> String {
    let mut out = String::new();
    if !event.trim().is_empty() {
        out.push_str("event: ");
        out.push_str(event.trim());
        out.push('\n');
    }

    // SSE "data" can be multi-line; each line is prefixed with `data:`.
    for line in data.split('\n') {
        out.push_str("data: ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out
}

pub(crate) fn sse_text_delta(text: &str) -> String {
    sse_json(vec![serde_json::json!({
        "type": "response.output_text.delta",
        "delta": text,
    })])
}

pub(crate) fn sse_tool_call(call_id: &str, name: &str, arguments: &str) -> String {
    sse_json(vec![serde_json::json!({
        "type": "response.output_item.done",
        "item": {
            "type": "function_call",
            "call_id": call_id,
            "name": name,
            "arguments": arguments,
        }
    })])
}

pub(crate) fn build_sse_body(events: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for e in events {
        out.push_str(&e);
    }
    out
}

/// Build an SSE stream body from a list of JSON events (Codex-style).
pub(crate) fn sse_json(events: Vec<Value>) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    for ev in events {
        let kind = ev.get("type").and_then(Value::as_str).unwrap_or("message");
        let _ = writeln!(&mut out, "event: {kind}");
        let is_minimal = ev.as_object().is_some_and(|o| o.len() == 1);
        if !is_minimal {
            let _ = writeln!(&mut out, "data: {ev}\n");
        } else {
            out.push('\n');
        }
    }
    out
}

pub(crate) fn build_sse_response(
    events: impl IntoIterator<Item = String>,
) -> wiremock::ResponseTemplate {
    let body = build_sse_body(events);
    wiremock::ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}
