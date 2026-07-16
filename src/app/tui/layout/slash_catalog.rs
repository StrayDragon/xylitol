//! Product slash command catalog for editor completion (c1170 / c1175).
//!
//! Names and descriptions come from [`crate::app::core::product_commands`] SSOT.

use xylitol_tui::SlashCommand;

use crate::app::core::product_commands::product_slash_commands;

pub(crate) fn product_slash_commands_for_editor() -> Vec<SlashCommand> {
    product_slash_commands()
        .into_iter()
        .map(|c| SlashCommand {
            name: c.name.into(),
            description: Some(c.description.into()),
            argument_hint: c.argument_hint.map(str::to_string),
            get_argument_completions: None,
        })
        .collect()
}
