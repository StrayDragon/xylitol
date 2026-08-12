//! Visible line hard-caps for the append-only surface (atao4).

/// tool / bash / assistant body visible lines (MUST be < mainline 5).
pub const APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES: usize = 3;

/// write / diff body visible lines (MUST be < mainline 10).
pub const APPEND_ONLY_WRITE_DIFF_LINES: usize = 5;

/// Main-session product TUI reference caps (scrollback preview).
pub const MAINLINE_TOOL_PREVIEW_LINES: usize = 5;
pub const MAINLINE_WRITE_PREVIEW_LINES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    ToolBashAssistant,
    WriteDiff,
}

impl BlockKind {
    pub fn visible_cap(self) -> usize {
        match self {
            Self::ToolBashAssistant => APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES,
            Self::WriteDiff => APPEND_ONLY_WRITE_DIFF_LINES,
        }
    }

    pub fn from_tool_name(name: &str) -> Self {
        match name {
            "write" | "edit" | "patch" | "diff" => Self::WriteDiff,
            _ => Self::ToolBashAssistant,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atao4_caps_strictly_below_mainline() {
        assert_eq!(APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES, 3);
        assert_eq!(APPEND_ONLY_WRITE_DIFF_LINES, 5);
        assert!(APPEND_ONLY_TOOL_BASH_ASSISTANT_LINES < MAINLINE_TOOL_PREVIEW_LINES);
        assert!(APPEND_ONLY_WRITE_DIFF_LINES < MAINLINE_WRITE_PREVIEW_LINES);
    }

    #[test]
    fn write_family_uses_taller_cap() {
        assert_eq!(BlockKind::from_tool_name("write").visible_cap(), 5);
        assert_eq!(BlockKind::from_tool_name("bash").visible_cap(), 3);
        assert_eq!(BlockKind::ToolBashAssistant.visible_cap(), 3);
    }
}
