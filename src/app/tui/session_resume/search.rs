//! Session resume search / sort (pi `session-selector-search` port).

use regex::Regex;
use xylitol_tui::fuzzy_match;

use crate::app::core::driver::SessionListEntry;
use crate::protocol::ports::flatten_session_forest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Threaded,
    Recent,
    Fuzzy,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Threaded => "Threaded",
            Self::Recent => "Recent",
            Self::Fuzzy => "Fuzzy",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Threaded => Self::Recent,
            Self::Recent => Self::Fuzzy,
            Self::Fuzzy => Self::Threaded,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameFilter {
    #[default]
    All,
    Named,
}

impl NameFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Named => "Named",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::All => Self::Named,
            Self::Named => Self::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionScope {
    #[default]
    Current,
    All,
}

impl SessionScope {
    pub fn toggle(self) -> Self {
        match self {
            Self::Current => Self::All,
            Self::All => Self::Current,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParsedSearchQuery {
    pub regex: Option<Regex>,
    pub phrases: Vec<String>,
    pub tokens: Vec<String>,
}

/// Parse filter text: `re:<pattern>`, `"exact phrase"`, and fuzzy tokens.
pub fn parse_search_query(raw: &str) -> ParsedSearchQuery {
    let mut out = ParsedSearchQuery::default();
    let mut i = 0;
    let chars: Vec<char> = raw.chars().collect();
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if chars[i] == '"' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            let phrase: String = chars[start..i].iter().collect();
            if !phrase.is_empty() {
                out.phrases.push(phrase);
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        let token: String = chars[start..i].iter().collect();
        if let Some(pat) = token.strip_prefix("re:") {
            if let Ok(re) = Regex::new(pat) {
                out.regex = Some(re);
            }
        } else if !token.is_empty() {
            out.tokens.push(token);
        }
    }
    out
}

fn entry_search_text(entry: &SessionListEntry) -> String {
    let primary = entry
        .name
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(entry.first_message.as_deref().filter(|s| !s.is_empty()))
        .unwrap_or(entry.id.as_str());
    format!("{primary} {}", entry.id)
}

/// Whether `entry` matches the parsed query (regex / phrase / fuzzy tokens).
pub fn match_session(entry: &SessionListEntry, query: &ParsedSearchQuery) -> bool {
    let hay = entry_search_text(entry);
    if let Some(re) = &query.regex
        && !re.is_match(&hay)
    {
        return false;
    }
    for phrase in &query.phrases {
        if !hay.to_lowercase().contains(&phrase.to_lowercase()) {
            return false;
        }
    }
    for token in &query.tokens {
        if fuzzy_match(token, &hay).is_none() && !hay.to_lowercase().contains(&token.to_lowercase())
        {
            return false;
        }
    }
    true
}

fn fuzzy_score(entry: &SessionListEntry, query: &ParsedSearchQuery) -> f64 {
    let hay = entry_search_text(entry);
    let mut score = 0.0f64;
    for token in &query.tokens {
        if let Some(m) = fuzzy_match(token, &hay) {
            score += m.score;
        } else if hay.to_lowercase().contains(&token.to_lowercase()) {
            score -= 1.0;
        } else {
            score += 1000.0;
        }
    }
    for phrase in &query.phrases {
        if hay.to_lowercase().contains(&phrase.to_lowercase()) {
            score -= 2.0;
        } else {
            score += 1000.0;
        }
    }
    score
}

fn canonical_cwd(path: &str) -> Option<String> {
    if path == "." {
        return std::env::current_dir()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .map(|p| p.to_string_lossy().into_owned());
    }
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| {
            std::path::Path::new(&home)
                .join(rest)
                .to_string_lossy()
                .into_owned()
        })
    } else if path == "~" {
        std::env::var_os("HOME").map(|h| h.to_string_lossy().into_owned())
    } else {
        Some(path.to_string())
    }?;
    std::path::Path::new(&expanded)
        .canonicalize()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn cwd_matches(session_cwd: Option<&str>, current_cwd: &str) -> bool {
    let Some(sc) = session_cwd.filter(|s| !s.is_empty()) else {
        return false;
    };
    match (canonical_cwd(sc), canonical_cwd(current_cwd)) {
        (Some(a), Some(b)) => a == b,
        _ => sc == current_cwd,
    }
}

pub fn is_named(entry: &SessionListEntry) -> bool {
    entry.name.as_deref().is_some_and(|s| !s.trim().is_empty())
}

/// Filter by scope / name / search, then sort per mode.
pub fn filter_and_sort(
    entries: &[SessionListEntry],
    scope: SessionScope,
    current_cwd: &str,
    name_filter: NameFilter,
    sort: SortMode,
    raw_query: &str,
) -> Vec<SessionListEntry> {
    let query = parse_search_query(raw_query);
    let mut filtered: Vec<SessionListEntry> = entries
        .iter()
        .filter(|e| match scope {
            SessionScope::All => true,
            SessionScope::Current => cwd_matches(e.cwd.as_deref(), current_cwd),
        })
        .filter(|e| match name_filter {
            NameFilter::All => true,
            NameFilter::Named => is_named(e),
        })
        .filter(|e| {
            if query.regex.is_none() && query.phrases.is_empty() && query.tokens.is_empty() {
                true
            } else {
                match_session(e, &query)
            }
        })
        .cloned()
        .collect();

    match sort {
        SortMode::Threaded => flatten_session_forest(filtered),
        SortMode::Recent => {
            filtered.sort_by(|a, b| {
                b.modified_unix
                    .unwrap_or(0)
                    .cmp(&a.modified_unix.unwrap_or(0))
            });
            for e in &mut filtered {
                e.tree_prefix.clear();
            }
            filtered
        }
        SortMode::Fuzzy => {
            if query.regex.is_none() && query.phrases.is_empty() && query.tokens.is_empty() {
                filtered.sort_by(|a, b| {
                    b.modified_unix
                        .unwrap_or(0)
                        .cmp(&a.modified_unix.unwrap_or(0))
                });
            } else {
                filtered.sort_by(|a, b| {
                    fuzzy_score(a, &query)
                        .partial_cmp(&fuzzy_score(b, &query))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            for e in &mut filtered {
                e.tree_prefix.clear();
            }
            filtered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, name: Option<&str>, msg: &str, cwd: Option<&str>) -> SessionListEntry {
        SessionListEntry {
            id: id.into(),
            name: name.map(str::to_string),
            first_message: Some(msg.into()),
            message_count: 1,
            modified_unix: Some(100),
            parent_session_id: None,
            tree_prefix: String::new(),
            cwd: cwd.map(str::to_string),
            path: None,
        }
    }

    #[test]
    fn parse_regex_and_phrase() {
        let q = parse_search_query(r#"re:foo "bar baz" hello"#);
        assert!(q.regex.is_some());
        assert_eq!(q.phrases, vec!["bar baz"]);
        assert_eq!(q.tokens, vec!["hello"]);
    }

    #[test]
    fn match_regex_filters() {
        let q = parse_search_query("re:older");
        assert!(match_session(&row("a", None, "older chat", None), &q));
        assert!(!match_session(&row("b", None, "new chat", None), &q));
    }

    #[test]
    fn named_filter_hides_unnamed() {
        let entries = vec![
            row("a", Some("Named"), "x", None),
            row("b", None, "unnamed preview", None),
        ];
        let out = filter_and_sort(
            &entries,
            SessionScope::All,
            ".",
            NameFilter::Named,
            SortMode::Recent,
            "",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
    }

    #[test]
    fn scope_current_filters_cwd() {
        let entries = vec![
            row("a", None, "here", Some("/tmp/proj")),
            row("b", None, "elsewhere", Some("/other")),
        ];
        let out = filter_and_sort(
            &entries,
            SessionScope::Current,
            "/tmp/proj",
            NameFilter::All,
            SortMode::Recent,
            "",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
    }
}
