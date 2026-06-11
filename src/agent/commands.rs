//! Slash commands — `/model`, `/compact`, `/help`, etc.
//!
//! Aligns with pi's slash-commands.ts. Provides:
//! - Builtin command table with descriptions
//! - Command dispatch interception in session.prompt()

#![allow(dead_code)]
#[allow(dead_code)]
/// Information about a registered slash command.
#[derive(Debug, Clone)]
pub(crate) struct SlashCommandInfo {
    /// Command name (without leading `/`).
    pub(crate) name: String,
    /// Human-readable description.
    pub(crate) description: String,
    /// Optional argument hint (e.g., "<model-id>").
    pub(crate) argument_hint: Option<String>,
}

impl SlashCommandInfo {
    pub(crate) fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            argument_hint: None,
        }
    }

    pub(crate) fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.argument_hint = Some(hint.into());
        self
    }
}

/// Built-in slash commands available in every session.
pub(crate) const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("model", "Select model"),
    ("compact", "Compact session context"),
    ("session", "Show session info"),
    ("fork", "Fork session at a previous message"),
    ("stats", "Show session statistics"),
    ("new", "Start a new session"),
    ("help", "Show available commands"),
];

/// Get the builtin commands as `SlashCommandInfo` vec.
pub(crate) fn builtin_commands() -> Vec<SlashCommandInfo> {
    BUILTIN_COMMANDS
        .iter()
        .map(|(name, desc)| SlashCommandInfo::new(*name, *desc))
        .collect()
}

/// Merge builtin commands with extension-registered commands.
pub(crate) fn get_all_commands(extensions: &[SlashCommandInfo]) -> Vec<SlashCommandInfo> {
    let mut commands = builtin_commands();
    commands.extend(extensions.iter().cloned());
    commands
}

/// Check if a line starts with a slash command.
///
/// Returns `Some(command_name)` if detected, stripping the leading `/`.
pub(crate) fn is_slash_command(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with('/') {
        return None;
    }
    // Exclude `/template:` which is handled separately
    if line.starts_with("/template:") {
        return None;
    }
    // Extract command name (up to first space or end)
    let cmd = &line[1..]; // skip /
    let name = cmd.split_whitespace().next().unwrap_or(cmd);
    Some(name)
}

/// Get the args part of a slash command line (everything after the command name).
pub(crate) fn get_command_args(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with('/') {
        return None;
    }
    let rest = &line[1..]; // skip /
    match rest.find(char::is_whitespace) {
        Some(pos) => Some(rest[pos + 1..].trim()),
        None => Some(""), // command with no args
    }
}

/// Find a slash command by name (case-insensitive).
pub(crate) fn find_command<'a>(
    name: &str,
    commands: &'a [SlashCommandInfo],
) -> Option<&'a SlashCommandInfo> {
    commands.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Look up a builtin command by name.
pub(crate) fn find_builtin_command(name: &str) -> Option<SlashCommandInfo> {
    BUILTIN_COMMANDS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(n, d)| SlashCommandInfo::new(*n, *d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_commands_have_expected_entries() {
        let cmds = builtin_commands();
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"model"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"session"));
        assert!(names.contains(&"fork"));
        assert!(names.contains(&"stats"));
        assert!(names.contains(&"new"));
        assert!(names.contains(&"help"));
        assert_eq!(names.len(), 7);
    }

    #[test]
    fn test_is_slash_command_detects() {
        assert_eq!(is_slash_command("/model"), Some("model"));
        assert_eq!(is_slash_command("/compact"), Some("compact"));
        assert_eq!(is_slash_command("/model gpt-4o"), Some("model"));
    }

    #[test]
    fn test_is_slash_command_excludes_template() {
        assert_eq!(is_slash_command("/template:review"), None);
    }

    #[test]
    fn test_is_slash_command_normal_text() {
        assert_eq!(is_slash_command("hello world"), None);
        assert_eq!(is_slash_command("not a /command"), None);
    }

    #[test]
    fn test_get_command_args() {
        assert_eq!(get_command_args("/model gpt-4o"), Some("gpt-4o"));
        assert_eq!(get_command_args("/model"), Some(""));
        assert_eq!(get_command_args("hello"), None);
    }

    #[test]
    fn test_find_command_case_insensitive() {
        let cmds = builtin_commands();
        assert!(find_command("MODEL", &cmds).is_some());
        assert!(find_command("Model", &cmds).is_some());
        assert!(find_command("nonexistent", &cmds).is_none());
    }

    #[test]
    fn test_get_all_commands_with_extensions() {
        let ext = vec![SlashCommandInfo::new("analyze", "Analyze code").with_hint("<file>")];
        let all = get_all_commands(&ext);
        assert_eq!(all.len(), 8); // 7 builtin + 1 extension
        assert!(all.iter().any(|c| c.name == "analyze"));
    }

    #[test]
    fn test_slash_command_info_with_hint() {
        let cmd = SlashCommandInfo::new("model", "Select model").with_hint("<model-id>");
        assert_eq!(cmd.argument_hint, Some("<model-id>".into()));
    }
}
