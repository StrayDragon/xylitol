//! Slash command parsing and completion.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SlashCommand {
    Clear,
    Help,
    Quit,
}

impl SlashCommand {
    pub(crate) fn parse(input: &str) -> Option<Self> {
        let s = input.trim();
        if !s.starts_with('/') {
            return None;
        }
        let cmd = s.trim_start_matches('/').trim();
        match cmd {
            "clear" => Some(Self::Clear),
            "help" => Some(Self::Help),
            "quit" | "exit" => Some(Self::Quit),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Clear => "/clear",
            Self::Help => "/help",
            Self::Quit => "/quit",
        }
    }
}

/// Simple prefix-tree completer for slash commands.
#[derive(Debug, Clone)]
pub(crate) struct Completer {
    trie: TrieNode,
}

impl Completer {
    pub(crate) fn new() -> Self {
        let mut c = Self {
            trie: TrieNode::default(),
        };
        for cmd in ["/clear", "/help", "/quit"] {
            c.insert(cmd);
        }
        c
    }

    fn insert(&mut self, word: &str) {
        let mut node = &mut self.trie;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.terminal = true;
    }

    /// Complete the current input when it looks like a slash command.
    ///
    /// - If there is exactly one matching command, return the full command.
    /// - Otherwise, return the input unchanged.
    pub(crate) fn complete(&self, input: &str) -> String {
        let trimmed = input.trim_end();
        if !trimmed.starts_with('/') {
            return input.to_string();
        }

        let matches = self.matches_for_prefix(trimmed);
        if matches.len() == 1 {
            return matches[0].clone();
        }

        // If the current input is already an exact command, keep it.
        if matches.iter().any(|m| m == trimmed) {
            return trimmed.to_string();
        }

        // Otherwise, keep original (could be ambiguous or missing).
        trimmed.to_string()
    }

    fn matches_for_prefix(&self, prefix: &str) -> Vec<String> {
        let mut node = &self.trie;
        for ch in prefix.chars() {
            let Some(next) = node.children.get(&ch) else {
                return Vec::new();
            };
            node = next;
        }

        let mut out = Vec::new();
        node.collect(prefix, &mut out);
        out
    }
}

#[derive(Debug, Default, Clone)]
struct TrieNode {
    terminal: bool,
    children: BTreeMap<char, TrieNode>,
}

impl TrieNode {
    fn collect(&self, prefix: &str, out: &mut Vec<String>) {
        if self.terminal {
            out.push(prefix.to_string());
        }
        for (ch, child) in &self.children {
            let mut next = String::with_capacity(prefix.len() + 1);
            next.push_str(prefix);
            next.push(*ch);
            child.collect(&next, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slash_command() {
        assert_eq!(SlashCommand::parse("/clear"), Some(SlashCommand::Clear));
        assert_eq!(SlashCommand::parse("  /help  "), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("/quit"), Some(SlashCommand::Quit));
        assert_eq!(SlashCommand::parse("/unknown"), None);
        assert_eq!(SlashCommand::parse("hello"), None);
    }

    #[test]
    fn test_completion_unique() {
        let c = Completer::new();
        assert_eq!(c.complete("/cl"), "/clear");
        assert_eq!(c.complete("/he"), "/help");
    }

    #[test]
    fn test_completion_ambiguous_or_missing() {
        let c = Completer::new();
        assert_eq!(c.complete("/"), "/");
        assert_eq!(c.complete("/q"), "/quit");
        assert_eq!(c.complete("/x"), "/x");
    }
}
