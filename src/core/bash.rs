//! Bash execution vocabulary — pure functions shared by agent and infra.

/// Returns `(exclude_from_context, command_without_prefix)` for a `!`/`!!` line.
///
/// `!!cmd` → exclude=true; `!cmd` → exclude=false; otherwise `None`.
pub fn parse_bang_prefix(input: &str) -> Option<(bool, &str)> {
    let trimmed = input.trim_start();
    if let Some(rest) = trimmed.strip_prefix("!!") {
        Some((true, rest.trim_start()))
    } else if let Some(rest) = trimmed.strip_prefix('!') {
        Some((false, rest.trim_start()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_double_bang_as_exclude() {
        let (exclude, cmd) = parse_bang_prefix("!!ls -la").unwrap();
        assert!(exclude);
        assert_eq!(cmd, "ls -la");
    }

    #[test]
    fn parses_single_bang_as_include() {
        let (exclude, cmd) = parse_bang_prefix("!echo hi").unwrap();
        assert!(!exclude);
        assert_eq!(cmd, "echo hi");
    }

    #[test]
    fn ignores_non_bang_input() {
        assert!(parse_bang_prefix("ls").is_none());
        assert!(parse_bang_prefix("/compact").is_none());
    }
}
