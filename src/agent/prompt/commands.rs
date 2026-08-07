//! Slash command types for skill/extension registration (agent-internal).
//!
//! Product builtin names live in [`crate::app::product_commands`] and are
//! assembled by [`XyDriver::get_commands`](crate::app::core::driver::XyDriver) —
//! agent must not own the product catalog.

use crate::protocol::source_info::SourceInfo;

/// Source of a registered (non-builtin) slash command.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // skill registration path not yet wired on product turn
pub(crate) enum SlashCommandSource {
    /// Registered from a discovered SKILL.md.
    Skill,
}

/// A slash command registered by a non-builtin source (skill, extension).
#[derive(Debug, Clone)]
#[allow(dead_code)] // retained until skill slash registration lands
pub(crate) struct SlashCommandInfo {
    /// Command name (without leading `/`).
    pub(crate) name: String,
    /// Human-readable description.
    pub(crate) description: String,
    /// Source of the command.
    pub(crate) source: SlashCommandSource,
    /// Provenance info for the originating resource, if applicable.
    pub(crate) source_info: Option<SourceInfo>,
}

impl SlashCommandInfo {
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
