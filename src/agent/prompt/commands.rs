//! Slash command types and discovery for agent + XyDriver GetCommands (c1175).
//!
//! Builtin names/descriptions come from [`super::product_commands`] (product SSOT).
//! Extension/skill commands merge on top.

use crate::protocol::source_info::SourceInfo;

use super::product_commands::product_slash_commands;

/// Source of a registered (non-builtin) slash command.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SlashCommandSource {
    /// Registered from a discovered SKILL.md.
    Skill,
}

/// A slash command registered by a non-builtin source (skill, extension).
#[derive(Debug, Clone)]
pub(crate) struct SlashCommandInfo {
    /// Command name (without leading `/`).
    pub(crate) name: String,
    /// Human-readable description.
    pub(crate) description: String,
    /// Source of the command.
    #[allow(dead_code)] // retained for extension provenance; not yet read on product path
    pub(crate) source: SlashCommandSource,
    /// Provenance info for the originating resource, if applicable.
    #[allow(dead_code)] // retained for extension provenance; not yet read on product path
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

/// Merge product builtins + extension/skill commands into a single list.
pub(crate) fn get_all_commands(extensions: &[SlashCommandInfo]) -> Vec<SlashCommandInfo> {
    let mut all: Vec<SlashCommandInfo> = product_slash_commands()
        .into_iter()
        .map(|c| SlashCommandInfo::new(c.name, c.description, SlashCommandSource::Skill))
        .collect();
    all.extend(extensions.iter().cloned());
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::prompt::product_commands::{LEGACY_SHORT_NAMES, product_slash_commands};

    #[test]
    fn builtins_match_product_ssot() {
        let ssot: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
        let builtins: Vec<String> = get_all_commands(&[]).into_iter().map(|c| c.name).collect();
        assert_eq!(
            builtins,
            ssot.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()
        );
        for legacy in LEGACY_SHORT_NAMES {
            assert!(!builtins.iter().any(|n| n == legacy));
        }
    }

    #[test]
    fn get_all_commands_includes_non_builtins() {
        let ext = vec![SlashCommandInfo::new(
            "analyze",
            "Analyze code",
            SlashCommandSource::Skill,
        )];
        let all = get_all_commands(&ext);
        assert!(all.iter().any(|c| c.name == "analyze"));
        assert!(all.iter().any(|c| c.name == "model"));
        assert!(all.iter().any(|c| c.name == "session-tree"));
    }

    #[test]
    fn slash_command_info_with_source_info() {
        use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};
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
    fn slash_command_source_skill_variant() {
        let skill = SlashCommandSource::Skill;
        assert_eq!(format!("{skill:?}"), "Skill");
    }
}
