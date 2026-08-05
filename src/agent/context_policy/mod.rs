//! ContextPolicy — request-layout hooks (c1890).
//!
//! Code-first defaults only; no YAML / env overlay this wave.
//! Status bar / tool_search behavior is deferred (`c1895` / `c1900`).

mod defaults;

pub use defaults::{
    ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT, DATE_PLACEMENT_DEFAULT, STATUS_BAR_MODE_DEFAULT,
    TOOLS_MODE_DEFAULT,
};

/// How tools are exposed on the provider request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsMode {
    /// Full tool schema list (current product default).
    Full,
    /// Deferred discovery / search (implemented in `c1900`).
    Search,
}

/// Agent status-bar injection mode (implemented in `c1895`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarMode {
    Off,
    Replace,
    Append,
}

/// Where calendar-day / date text is placed (final algorithm in `c1905` / `c1895`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatePlacement {
    /// Keep today's `build_system_prompt` date behavior.
    SystemAsToday,
}

/// Layout policy consumed by Assembler / ReAct hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPolicy {
    pub tools_mode: ToolsMode,
    pub status_bar_mode: StatusBarMode,
    pub allow_midturn_tools_rewrite: bool,
    pub date_placement: DatePlacement,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            tools_mode: TOOLS_MODE_DEFAULT,
            status_bar_mode: STATUS_BAR_MODE_DEFAULT,
            allow_midturn_tools_rewrite: ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT,
            date_placement: DATE_PLACEMENT_DEFAULT,
        }
    }
}

impl ContextPolicy {
    /// Whether provider `tools` may be rewritten mid-turn (Search defaults false).
    pub fn allows_midturn_tools_rewrite(&self) -> bool {
        match self.tools_mode {
            ToolsMode::Full => self.allow_midturn_tools_rewrite,
            ToolsMode::Search => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hooks_match_design() {
        let p = ContextPolicy::default();
        assert_eq!(p.tools_mode, ToolsMode::Full);
        assert_eq!(p.status_bar_mode, StatusBarMode::Off);
        assert_eq!(p.date_placement, DatePlacement::SystemAsToday);
        assert!(p.allow_midturn_tools_rewrite);
        assert!(p.allows_midturn_tools_rewrite());
    }

    #[test]
    fn search_mode_forbids_midturn_tools_rewrite() {
        let p = ContextPolicy {
            tools_mode: ToolsMode::Search,
            allow_midturn_tools_rewrite: true,
            ..Default::default()
        };
        assert!(!p.allows_midturn_tools_rewrite());
    }
}
