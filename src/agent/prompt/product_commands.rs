//! Product slash-command SSOT (c1175).
//!
//! Single catalog for XyDriver/GetCommands and the product TUI `SlashCommandSource`.
//! Names follow PI_DELTAS A03 (`session-*`); legacy short names are intentionally absent.

/// One product-facing slash command (no leading `/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductSlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub argument_hint: Option<&'static str>,
}

/// Built-in product commands. Debug-only entries appear only under `debug_assertions`.
pub fn product_slash_commands() -> Vec<ProductSlashCommand> {
    let mut cmds = vec![
        ProductSlashCommand {
            name: "exit",
            description: "Quit TUI",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "model",
            description: "Switch model: /model [id]",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-tree",
            description: "Open session tree (same as double Esc)",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-fork",
            description: "Fork session at current leaf",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-compact",
            description: "Compact session context",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-export",
            description: "Export session (default HTML; .jsonl → JSONL)",
            argument_hint: Some("[path]"),
        },
        ProductSlashCommand {
            name: "session-import",
            description: "Import session from JSONL",
            argument_hint: Some("<path>"),
        },
        ProductSlashCommand {
            name: "session",
            description: "Show session info and stats",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-resume",
            description: "Switch to another session",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-new",
            description: "Start a new empty session",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-clone",
            description: "Clone session at current leaf (fork at)",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "session-name",
            description: "Show or set session display name",
            argument_hint: Some("[name]"),
        },
        ProductSlashCommand {
            name: "reload",
            description: "Hot-reload keybindings, skills, MCP, themes, context",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "trust",
            description: "Persist project trust: /trust [self|parent|deny]",
            argument_hint: Some("[self|parent|deny]"),
        },
        ProductSlashCommand {
            name: "history-copy-last",
            description: "Copy last assistant message to clipboard",
            argument_hint: None,
        },
        ProductSlashCommand {
            name: "theme",
            description: "Switch theme: /theme [dark|light|toggle]",
            argument_hint: Some("[dark|light|toggle]"),
        },
        ProductSlashCommand {
            name: "mcp",
            description: "Show MCP servers and tools armed status",
            argument_hint: None,
        },
    ];
    #[cfg(debug_assertions)]
    cmds.push(ProductSlashCommand {
        name: "debug",
        description: "Load fixture: /debug <scene>",
        argument_hint: Some("<scene>"),
    });
    cmds
}

/// Names that MUST NOT appear as product builtin primary names (A03).
pub const LEGACY_SHORT_NAMES: &[&str] = &[
    "tree", "fork", "export", "import", "compact", "resume", "new", "clone", "name", "quit",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_catalog_has_session_tree_not_legacy_tree() {
        let names: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
        assert!(names.contains(&"session-tree"));
        assert!(names.contains(&"model"));
        assert!(names.contains(&"exit"));
        for legacy in LEGACY_SHORT_NAMES {
            assert!(
                !names.contains(legacy),
                "legacy short name /{legacy} must not be a product builtin primary"
            );
        }
    }
}
