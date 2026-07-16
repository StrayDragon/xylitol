//! Product slash command catalog for editor completion (c1170).

use xylitol_tui::SlashCommand;

pub(super) fn product_slash_commands() -> Vec<SlashCommand> {
    let mut cmds = vec![
        SlashCommand {
            name: "exit".into(),
            description: Some("Quit TUI".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "model".into(),
            description: Some("Switch model: /model [id]".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-tree".into(),
            description: Some("Open session tree (same as double Esc)".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-fork".into(),
            description: Some("Fork session at current leaf".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-compact".into(),
            description: Some("Compact session context".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-export".into(),
            description: Some("Export session (default HTML; .jsonl → JSONL)".into()),
            argument_hint: Some("[path]".into()),
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-import".into(),
            description: Some("Import session from JSONL".into()),
            argument_hint: Some("<path>".into()),
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session".into(),
            description: Some("Show session info and stats".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-resume".into(),
            description: Some("Switch to another session".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-new".into(),
            description: Some("Start a new empty session".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-clone".into(),
            description: Some("Clone session at current leaf (fork at)".into()),
            argument_hint: None,
            get_argument_completions: None,
        },
        SlashCommand {
            name: "session-name".into(),
            description: Some("Show or set session display name".into()),
            argument_hint: Some("[name]".into()),
            get_argument_completions: None,
        },
    ];
    // Hand-test only — see `app::debug_fixtures` (delete that module to remove).
    #[cfg(debug_assertions)]
    cmds.push(SlashCommand {
        name: "debug".into(),
        description: Some("Load fixture: /debug <scene>".into()),
        argument_hint: Some("<scene>".into()),
        get_argument_completions: None,
    });
    cmds
}
