//! ContextPolicy — request-layout hooks (c1890 / c1905).
//!
//! Code-first defaults only; no YAML / env overlay this wave.
//! Status bar injection is deferred (c1895) — not represented in the type.
//! Tool search is deferred (c1960); only the full-tool layout ships (`ToolsMode::Full`).
//! Calendar-day placement knob removed (c2730): session_env carries date/cwd.

mod defaults;

pub use defaults::{ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT, TOOLS_MODE_DEFAULT};

/// How tools are exposed on the provider request (full-schema is the only
/// shipped layout; discovery/search deferred — c1960).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolsMode {
    /// Full tool schema list (current product default).
    #[default]
    Full,
}

/// Where calendar-day text is placed **in the system prompt** (c1905).
///
/// Layout policy consumed by Assembler / ReAct hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPolicy {
    pub tools_mode: ToolsMode,
    pub allow_midturn_tools_rewrite: bool,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            tools_mode: TOOLS_MODE_DEFAULT,
            allow_midturn_tools_rewrite: ALLOW_MIDTURN_TOOLS_REWRITE_DEFAULT,
        }
    }
}

impl ContextPolicy {
    /// Whether provider `tools` may be rewritten mid-turn (flag-gated;
    /// `Search` layout that once forbade rewrites is deferred — c1960).
    pub fn allows_midturn_tools_rewrite(&self) -> bool {
        self.allow_midturn_tools_rewrite
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
        assert!(p.allow_midturn_tools_rewrite);
        assert!(p.allows_midturn_tools_rewrite());
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
