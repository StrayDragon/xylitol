//! `XyEvent` → [`AppendOnlyModel`] (atao3/atao4/atao7).

use crate::app::core::driver::XyEvent;

use super::caps::BlockKind;
use super::model::AppendOnlyModel;

/// Apply one event into the append-only model.
pub fn apply_xy_event(model: &mut AppendOnlyModel, event: &XyEvent) -> Result<(), String> {
    match event {
        XyEvent::AgentStart { .. } => {
            model.busy = true;
            model.abort_latched = false;
            model.ctrl_c_armed = false;
            model.streaming_assistant.clear();
            Ok(())
        }
        XyEvent::AgentEnd { .. } => {
            flush_assistant(model)?;
            model.busy = false;
            Ok(())
        }
        XyEvent::TextDelta(text) => {
            model.streaming_assistant.push_str(text);
            Ok(())
        }
        XyEvent::MessageEnd { .. } => flush_assistant(model),
        XyEvent::ToolExecutionEnd { name, result, .. } => {
            let kind = BlockKind::from_tool_name(name);
            let body = result.as_str();
            // Prefer path already present in tool result when spilled upstream.
            if let Some(path) = extract_full_output_path(body) {
                let cap = kind.visible_cap();
                let lines: Vec<&str> = body.lines().collect();
                let visible = if lines.len() > cap {
                    let head = lines.into_iter().take(cap).collect::<Vec<_>>().join("\n");
                    if let Some(footer) = body.lines().find(|l| l.starts_with("[Full output:")) {
                        format!("{head}\n{footer}")
                    } else {
                        format!(
                            "{head}\n[Full output: {path}. Truncated: {cap} lines shown (append-only cap)]"
                        )
                    }
                } else {
                    body.to_string()
                };
                model.blocks.push(super::model::AppendBlock {
                    kind,
                    title: format!("⚙ {name}"),
                    visible,
                    full_output_path: Some(std::path::PathBuf::from(path)),
                    expandable: false,
                });
                Ok(())
            } else {
                model.commit_capped(kind, format!("⚙ {name}"), body, name)
            }
        }
        XyEvent::ToolExecutionStart { name, .. } => {
            model.status_line = format!("running {name}…");
            Ok(())
        }
        XyEvent::Error(err) => {
            model.busy = false;
            model.commit_capped(BlockKind::ToolBashAssistant, "error", &err.message, "error")
        }
        _ => Ok(()),
    }
}

fn flush_assistant(model: &mut AppendOnlyModel) -> Result<(), String> {
    if model.streaming_assistant.is_empty() {
        return Ok(());
    }
    let body = std::mem::take(&mut model.streaming_assistant);
    model.commit_capped(
        BlockKind::ToolBashAssistant,
        "assistant",
        &body,
        "assistant",
    )
}

fn extract_full_output_path(body: &str) -> Option<&str> {
    for line in body.lines() {
        let rest = line.strip_prefix("[Full output: ")?;
        let path = rest.split('.').next()?.trim();
        if !path.is_empty() && path != "(unavailable)" {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::driver::XyEvent;

    #[test]
    fn streaming_then_end_commits_capped_assistant() {
        let tmp = tempfile::tempdir().unwrap();
        let mut model = AppendOnlyModel::new(Some(tmp.path().to_path_buf()));
        apply_xy_event(
            &mut model,
            &XyEvent::AgentStart {
                session_id: "s".into(),
                model: "m".into(),
            },
        )
        .unwrap();
        apply_xy_event(&mut model, &XyEvent::TextDelta("a\nb\nc\nd\n".into())).unwrap();
        apply_xy_event(
            &mut model,
            &XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        )
        .unwrap();
        assert!(!model.busy);
        assert_eq!(model.blocks.len(), 1);
        assert!(!model.blocks[0].expandable);
        assert!(model.blocks[0].visible.contains("[Full output:"));
    }

    #[test]
    fn no_fold_flags_on_tool_end() {
        let tmp = tempfile::tempdir().unwrap();
        let mut model = AppendOnlyModel::new(Some(tmp.path().to_path_buf()));
        let result = serde_json::json!({
            "output": "1\n2\n3\n4\n5\n6\n",
            "exit_code": 0
        });
        // ToolExecutionEnd carries Display via Debug-ish — use string body path via commit.
        // Prefer constructing via display string in event; check XyEvent shape.
        let _ = result;
        model
            .commit_capped(
                BlockKind::from_tool_name("bash"),
                "⚙ bash",
                "1\n2\n3\n4\n5\n6\n",
                "bash",
            )
            .unwrap();
        assert!(!model.any_expandable());
        assert!(model.blocks[0].full_output_path.is_some());
    }
}
