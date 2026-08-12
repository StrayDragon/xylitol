//! Append-only UI model — committed blocks with hard caps (atao3/atao4).

use std::path::PathBuf;

use super::caps::BlockKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendBlock {
    pub kind: BlockKind,
    pub title: String,
    /// Visible body (already capped; may include Full output footer).
    pub visible: String,
    pub full_output_path: Option<PathBuf>,
    /// Fold/expand MUST stay false — no expand interaction on this surface.
    pub expandable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AppendOnlyModel {
    pub blocks: Vec<AppendBlock>,
    /// Streaming assistant buffer (not yet committed).
    pub streaming_assistant: String,
    pub busy: bool,
    pub quit: bool,
    pub abort_latched: bool,
    /// Idle Ctrl+C arm for double-tap exit.
    pub ctrl_c_armed: bool,
    pub spill_dir: Option<PathBuf>,
    pub status_line: String,
}

impl AppendOnlyModel {
    pub fn new(spill_dir: Option<PathBuf>) -> Self {
        Self {
            spill_dir,
            status_line: "append-only surface · /exit to quit".into(),
            ..Default::default()
        }
    }

    pub fn transcript_text(&self) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            if !block.title.is_empty() {
                out.push_str(&block.title);
                out.push('\n');
            }
            if !block.visible.is_empty() {
                out.push_str(&block.visible);
                out.push('\n');
            }
            out.push('\n');
        }
        if !self.streaming_assistant.is_empty() {
            out.push_str(&self.streaming_assistant);
        }
        out
    }

    /// Commit a block with hard cap + optional spill (never expandable).
    pub fn commit_capped(
        &mut self,
        kind: BlockKind,
        title: impl Into<String>,
        full_body: &str,
        stem: &str,
    ) -> Result<(), String> {
        let cap = kind.visible_cap();
        let (visible, path) = if let Some(dir) = self.spill_dir.as_ref() {
            super::spill::cap_with_spill(full_body, cap, dir, stem)?
        } else {
            // No spill dir: hard-truncate without path (tests / degraded).
            let lines: Vec<&str> = full_body.lines().collect();
            if lines.len() <= cap {
                (full_body.to_string(), None)
            } else {
                let body = lines.into_iter().take(cap).collect::<Vec<_>>().join("\n");
                (
                    format!(
                        "{body}\n[Full output: (unavailable). Truncated: {cap} lines shown (append-only cap)]"
                    ),
                    None,
                )
            }
        };
        self.blocks.push(AppendBlock {
            kind,
            title: title.into(),
            visible,
            full_output_path: path,
            expandable: false,
        });
        Ok(())
    }

    pub fn any_expandable(&self) -> bool {
        self.blocks.iter().any(|b| b.expandable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::append_only::caps::APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES;

    #[test]
    fn blocks_never_expandable() {
        let tmp = tempfile::tempdir().unwrap();
        let mut model = AppendOnlyModel::new(Some(tmp.path().to_path_buf()));
        let body = "1\n2\n3\n4\n5\n";
        model
            .commit_capped(BlockKind::ToolBashAssistant, "tool", body, "t")
            .unwrap();
        assert!(!model.any_expandable());
        let visible_lines = model.blocks[0].visible.lines().count();
        // cap lines + Full output footer
        assert!(visible_lines > APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES);
        assert!(model.blocks[0].full_output_path.is_some());
        assert!(model.blocks[0].visible.contains("[Full output:"));
    }
}
