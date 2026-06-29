//! Slash commands — `/model`, `/compact`, `/export`, etc.
//!
//! Provides a full builtin command table, slash-command detection,
//! and slash-command info types for routing.
//!
//! ## Architecture
//! - **Builtins** are defined as simple `(&str, &str)` tuples (name, description).
//! - **Non-builtin commands** (from skills, prompts, extensions) carry a
//!   [`SlashCommandSource`] and optional `source_path` for provenance.
//! - AgentSession owns the dispatch logic (`dispatch_slash_command` in session.rs).

use crate::domain::source_info::SourceInfo;

/// Source of a registered (non-builtin) slash command.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SlashCommandSource {
    /// Registered from a prompt template (`/template:name`).
    Prompt,
    /// Registered from a discovered SKILL.md.
    Skill,
}

/// A slash command registered by a non-builtin source (skill, prompt, extension).
#[derive(Debug, Clone)]
pub(crate) struct SlashCommandInfo {
    /// Command name (without leading `/`).
    pub(crate) name: String,
    /// Human-readable description.
    pub(crate) description: String,
    /// Source of the command.
    #[allow(dead_code)]
    pub(crate) source: SlashCommandSource,
    /// Provenance info for the originating resource, if applicable.
    pub(crate) source_info: Option<SourceInfo>,
}

impl SlashCommandInfo {
    pub(crate) fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        source: SlashCommandSource,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            source,
            source_info: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_source_info(mut self, info: SourceInfo) -> Self {
        self.source_info = Some(info);
        self
    }
}

/// Full builtin slash command table.
///
/// Command descriptions use `[requires ...]` tags to indicate optional feature
/// requirements so callers can distinguish them without a separate flag.
pub(crate) const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("model", "Select model"),
    ("compact", "Manually compact the session context"),
    ("session", "Show session info and stats"),
    ("fork", "Fork session at a previous message"),
    ("new", "Start a new session"),
    ("export", "Export session (HTML/JSONL)"),
    ("import", "Import and resume a session from a JSONL file"),
    ("tree", "Navigate session tree (switch branches)"),
    ("resume", "Resume a different session"),
    ("quit", "Quit the agent"),
    ("settings", "Open settings menu"),
    ("scoped-models", "Enable/disable models for cycling"),
    ("share", "Share session as a gist"),
    (
        "copy",
        "Copy last agent message to clipboard [requires clipboard]",
    ),
    ("name", "Set session display name"),
    ("changelog", "Show changelog entries"),
    ("hotkeys", "Show all keyboard shortcuts"),
    ("clone", "Clone the current session"),
    ("trust", "Save project trust decision"),
    ("login", "Configure provider authentication"),
    ("logout", "Remove provider authentication"),
    ("reload", "Reload extensions, skills, and prompts"),
];

/// Get builtin commands as a Vec for iteration.
/// Merge builtin + extension/skill/prompt commands into a single list.
pub(crate) fn get_all_commands(extensions: &[SlashCommandInfo]) -> Vec<SlashCommandInfo> {
    let mut all: Vec<SlashCommandInfo> = BUILTIN_COMMANDS
        .iter()
        .map(|(n, d)| SlashCommandInfo::new(*n, *d, SlashCommandSource::Skill))
        .collect();
    all.extend(extensions.iter().cloned());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_22_commands() {
        assert_eq!(BUILTIN_COMMANDS.len(), 22);
        let names: Vec<&str> = BUILTIN_COMMANDS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"model"));
        assert!(names.contains(&"export"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"tree"));
        assert!(names.contains(&"fork"));
        assert!(names.contains(&"reload"));
        assert!(names.contains(&"quit"));
        assert!(!names.contains(&"stats"));
    }

    #[test]
    fn test_get_all_commands_includes_non_builtins() {
        let ext = vec![SlashCommandInfo::new(
            "analyze",
            "Analyze code",
            SlashCommandSource::Skill,
        )];
        let all = get_all_commands(&ext);
        assert!(all.iter().any(|c| c.name == "analyze"));
        assert!(all.iter().any(|c| c.name == "model"));
    }

    #[test]
    fn test_slash_command_info_with_source_info() {
        use crate::domain::source_info::{SourceInfo, SourceOrigin, SourceScope};
        let si = SourceInfo {
            path: std::path::PathBuf::from("/a/b/c.md"),
            source: "local".into(),
            scope: SourceScope::Temporary,
            origin: SourceOrigin::TopLevel,
            base_dir: None,
        };
        let cmd =
            SlashCommandInfo::new("x", "desc", SlashCommandSource::Skill).with_source_info(si);
        assert_eq!(
            cmd.source_info.as_ref().map(|s| s.path.as_path()),
            Some(std::path::Path::new("/a/b/c.md"))
        );
        assert_eq!(cmd.source, SlashCommandSource::Skill);
    }

    #[test]
    fn test_slash_command_source_variants() {
        let skill = SlashCommandSource::Skill;
        let prompt = SlashCommandSource::Prompt;
        assert_ne!(format!("{skill:?}"), format!("{prompt:?}"));
    }
}
