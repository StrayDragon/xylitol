//! ContextPolicy — request-layout hooks (c1890 / c1905).
//!
//! Code-first defaults only; no YAML / env overlay this wave.
//! Status bar is deferred (`c1895`). Tool search / `ToolsMode::Search` is `c1960`.
//! Track-A freeze (c1900) keeps `ToolsMode::Full` as the open-box default.
//! Calendar-day placement knob removed (c2730): session_env carries date/cwd.

mod defaults;

pub use defaults::{
    ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT, STATUS_BAR_MODE_DEFAULT, TOOLS_MODE_DEFAULT,
};

/// How tools are exposed on the provider request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // c2750 dead-code purge candidate
pub enum ToolsMode {
    /// Full tool schema list (current product default).
    #[default]
    Full,
    /// Deferred discovery / search (c1960; not delivered by c1900 freeze track).
    Search,
}

/// Agent status-bar injection mode (full Lane Runtime in `c1895`).
///
/// Product already has one **special status-bar type** shipped by c1905:
/// [`crate::agent::prompt::CUSTOM_TYPE_SESSION_ENV`] (`session_env` bootstrap).
/// c1895 SHOULD scan / merge that type when enabling `Replace` / `Append`;
/// default remains [`StatusBarMode::Off`] (bootstrap-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // c2750 dead-code purge candidate
pub enum StatusBarMode {
    #[default]
    Off,
    Replace,
    Append,
}

/// Where calendar-day text is placed **in the system prompt** (c1905).
///
/// Layout policy consumed by Assembler / ReAct hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPolicy {
    pub tools_mode: ToolsMode,
    pub status_bar_mode: StatusBarMode,
    pub allow_midturn_tools_rewrite: bool,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            tools_mode: TOOLS_MODE_DEFAULT,
            status_bar_mode: STATUS_BAR_MODE_DEFAULT,
            allow_midturn_tools_rewrite: ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT,
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

    /// Ablation helper (was public restore scaffolding; product uses session_env).
    fn calendar_date_from_header_timestamp(timestamp: &str) -> Option<String> {
        let t = timestamp.trim();
        if t.len() >= 10 && t.as_bytes().get(4) == Some(&b'-') && t.as_bytes().get(7) == Some(&b'-')
        {
            let d = &t[..10];
            if d.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
                return Some(d.to_string());
            }
        }
        None
    }

    #[test]
    fn default_hooks_match_design() {
        let p = ContextPolicy::default();
        assert_eq!(p.tools_mode, ToolsMode::Full);
        assert_eq!(p.status_bar_mode, StatusBarMode::Off);
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

    #[test]
    fn calendar_date_from_rfc3339_header() {
        assert_eq!(
            calendar_date_from_header_timestamp("2026-08-05T14:25:00+08:00").as_deref(),
            Some("2026-08-05")
        );
        assert_eq!(
            calendar_date_from_header_timestamp("2026-08-06").as_deref(),
            Some("2026-08-06")
        );
        assert_eq!(calendar_date_from_header_timestamp("bogus"), None);
    }
}
