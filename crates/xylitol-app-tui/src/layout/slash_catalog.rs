use xylitol_tui::SlashCommand;

use crate::product_commands::product_slash_commands;

#[cfg(test)]
pub(crate) fn product_slash_commands_for_editor() -> Vec<SlashCommand> {
    slash_commands_for_editor(false, &[])
}

pub(crate) fn slash_commands_for_editor(
    designing: bool,
    plugin_commands: &[(String, String)],
) -> Vec<SlashCommand> {
    let mut cmds: Vec<SlashCommand> = product_slash_commands()
        .into_iter()
        .map(|c| SlashCommand {
            name: c.name.into(),
            description: Some(c.description.into()),
            argument_hint: c.argument_hint.map(str::to_string),
            get_argument_completions: None,
        })
        .collect();
    for (name, description) in plugin_commands {
        if !name.starts_with("plugin:") || cmds.iter().any(|c| c.name == *name) {
            continue;
        }
        cmds.push(SlashCommand {
            name: name.clone(),
            description: (!description.is_empty()).then(|| description.clone()),
            argument_hint: None,
            get_argument_completions: None,
        });
    }
    if designing {
        cmds.insert(
            0,
            SlashCommand {
                name: "designing".into(),
                description: Some("Open designing scene slot".into()),
                argument_hint: None,
                get_argument_completions: None,
            },
        );
    }
    cmds
}
