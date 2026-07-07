//! File-path and slash-command autocomplete provider.
//!
//! Ported from pi's `autocomplete.ts`. The provider walks the filesystem
//! synchronously (via `std::fs`) and uses [`crate::fuzzy::fuzzy_match`] for
//! filtering. An optional `fd`-based fuzzy search is left to the consumer.

use crate::fuzzy::fuzzy_match;
use std::path::{Path, PathBuf};

// ── types ───────────────────────────────────────────────────────────────────

pub struct AutocompleteItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// Awaitable alias — synchronous for now, but signature allows future async.
pub type Awaitable<T> = T;

#[allow(clippy::type_complexity)]
pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    /// Return argument completions for a given prefix; `None` = not supported.
    pub get_argument_completions: Option<Box<dyn Fn(&str) -> Option<Vec<AutocompleteItem>>>>,
}

pub struct AutocompleteSuggestions {
    pub items: Vec<AutocompleteItem>,
    /// What we're matching against (e.g. "/" or "src/").
    pub prefix: String,
}

pub trait AutocompleteProvider {
    /// Characters that should naturally trigger this provider at token boundaries.
    fn trigger_characters(&self) -> &[char] {
        &[]
    }

    /// Get autocomplete suggestions for current text/cursor position.
    /// Returns `None` if no suggestions available.
    fn get_suggestions(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        force: bool,
    ) -> Option<AutocompleteSuggestions>;

    /// Apply the selected item. Returns the new text and cursor position.
    fn apply_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> (Vec<String>, usize, usize);

    /// Whether file completion should trigger on explicit Tab.
    fn should_trigger_file_completion(
        &self,
        _lines: &[String],
        _cursor_line: usize,
        _cursor_col: usize,
    ) -> bool {
        true
    }
}

// ── Combined provider: slash commands + file paths ──────────────────────────

pub struct CombinedAutocompleteProvider {
    commands: Vec<(String, String)>,
    /// The command objects are passed as-is; we store just name+description for
    /// easier search.
    base_path: PathBuf,
}

impl CombinedAutocompleteProvider {
    pub fn new(commands: Vec<SlashCommand>, base_path: PathBuf) -> Self {
        let names = commands
            .into_iter()
            .map(|c| (c.name, c.description.unwrap_or_default()))
            .collect();
        Self {
            commands: names,
            base_path,
        }
    }
}

impl AutocompleteProvider for CombinedAutocompleteProvider {
    fn get_suggestions(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        force: bool,
    ) -> Option<AutocompleteSuggestions> {
        let current = lines.get(cursor_line)?.clone();
        let before_cursor = &current[..cursor_col.min(current.len())];

        // @ file attachments (fuzzy path)
        if let Some(prefix) = self.extract_at_prefix(before_cursor) {
            let (_raw, _is_at, is_quoted) = parse_path_prefix(&prefix);
            let suggestions = self.get_fuzzy_file_suggestions(&prefix, is_quoted);
            if suggestions.is_empty() {
                return None;
            }
            return Some(AutocompleteSuggestions {
                items: suggestions,
                prefix,
            });
        }

        // Slash commands
        if before_cursor.starts_with('/') {
            let space_idx = before_cursor.find(' ');
            if space_idx.is_none() {
                // Completing command name.
                let prefix = before_cursor.strip_prefix('/').unwrap_or("");
                let items: Vec<AutocompleteItem> = self
                    .commands
                    .iter()
                    .filter_map(|(name, desc)| {
                        fuzzy_match(prefix, name)?;
                        Some(AutocompleteItem {
                            value: name.clone(),
                            label: name.clone(),
                            description: if desc.is_empty() {
                                None
                            } else {
                                Some(desc.clone())
                            },
                        })
                    })
                    .collect();
                if items.is_empty() {
                    return None;
                }
                return Some(AutocompleteSuggestions {
                    items,
                    prefix: before_cursor.to_string(),
                });
            }
            // Command argument completion — skipped for now (no command objects).
            return None;
        }

        // File paths
        if let Some(path_match) = self.extract_path_prefix(before_cursor, force) {
            let suggestions = self.get_file_suggestions(&path_match);
            if suggestions.is_empty() {
                return None;
            }
            return Some(AutocompleteSuggestions {
                items: suggestions,
                prefix: path_match,
            });
        }

        None
    }

    fn apply_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> (Vec<String>, usize, usize) {
        let current = lines[cursor_line].clone();
        let before = &current[..cursor_col.saturating_sub(prefix.len())];
        let after = &current[cursor_col..];

        // Slash command completion: "/" + name + " "
        if prefix.starts_with('/') && before.trim().is_empty() {
            let new_line = format!("/{} {}", item.value, after);
            let mut new_lines = lines.to_vec();
            new_lines[cursor_line] = new_line;
            return (new_lines, cursor_line, before.len() + item.value.len() + 2);
        }

        // @-attachment
        if prefix.starts_with('@') {
            let is_dir = item.label.ends_with('/');
            let has_trailing_quote = item.value.ends_with('"');
            let cursor_offset = if is_dir && has_trailing_quote {
                item.value.len().saturating_sub(1)
            } else {
                item.value.len()
            };

            let suffix = if is_dir { "" } else { " " };
            let new_line = format!("{}{}{}{}", before, item.value, suffix, after);
            let mut new_lines = lines.to_vec();
            new_lines[cursor_line] = new_line;
            return (
                new_lines,
                cursor_line,
                before.len() + cursor_offset + suffix.len(),
            );
        }

        // Regular file path completion
        let has_trailing_quote = item.value.ends_with('"');
        let is_dir = item.label.ends_with('/');
        let cursor_offset = if is_dir && has_trailing_quote {
            item.value.len().saturating_sub(1)
        } else {
            item.value.len()
        };

        let new_line = format!("{}{}{}", before, item.value, after);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        (new_lines, cursor_line, before.len() + cursor_offset)
    }
}

// ── private helpers ─────────────────────────────────────────────────────────

impl CombinedAutocompleteProvider {
    fn extract_path_prefix(&self, text: &str, force: bool) -> Option<String> {
        let quoted = extract_quoted_prefix(text);
        if let Some(q) = quoted {
            return Some(q);
        }

        let delim = find_last_delimiter(text);
        let prefix = if delim < 0 {
            text
        } else {
            &text[(delim + 1) as usize..]
        };

        if force {
            return Some(prefix.to_string());
        }

        if prefix.contains('/') || prefix.starts_with('.') || prefix.starts_with("~/") {
            return Some(prefix.to_string());
        }

        if prefix.is_empty() && text.ends_with(' ') {
            return Some(prefix.to_string());
        }

        None
    }

    fn extract_at_prefix(&self, text: &str) -> Option<String> {
        let quoted = extract_quoted_prefix(text);
        if let Some(ref q) = quoted
            && q.starts_with("@\"")
        {
            return quoted;
        }

        let delim = find_last_delimiter(text);
        let start = if delim < 0 { 0 } else { (delim + 1) as usize };
        if text[start..].starts_with('@') {
            return Some(text[start..].to_string());
        }
        None
    }

    fn get_file_suggestions(&self, prefix: &str) -> Vec<AutocompleteItem> {
        let (raw, is_at, is_quoted) = parse_path_prefix(prefix);
        let expanded = expand_home_path(raw);

        let (search_dir, search_prefix, display_prefix) =
            match resolve_search_dir(&self.base_path, &expanded, raw) {
                Ok(r) => r,
                Err(_) => return vec![],
            };

        let entries = match std::fs::read_dir(&search_dir) {
            Ok(iter) => iter.filter_map(|e| e.ok()).collect::<Vec<_>>(),
            Err(_) => return vec![],
        };

        let lower = search_prefix.to_lowercase();
        let mut suggestions: Vec<AutocompleteItem> = entries
            .iter()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.to_lowercase().starts_with(&lower))
            })
            .filter_map(|e| {
                let is_dir = e.file_type().ok()?.is_dir();
                let name = e.file_name().to_str()?.to_string();

                let relative = construct_relative_path(&display_prefix, &name);

                let display = to_display_path(&relative);
                let path_value = if is_dir {
                    format!("{}/", display)
                } else {
                    display
                };
                let value = build_completion_value(&path_value, is_dir, is_at, is_quoted);
                Some(AutocompleteItem {
                    value,
                    label: if is_dir { format!("{name}/") } else { name },
                    description: None,
                })
            })
            .collect();

        // Sort: dirs first, then alphabetical.
        suggestions.sort_by(|a, b| {
            let a_dir = a.value.ends_with('/');
            let b_dir = b.value.ends_with('/');
            if a_dir && !b_dir {
                std::cmp::Ordering::Less
            } else if !a_dir && b_dir {
                std::cmp::Ordering::Greater
            } else {
                a.label.to_lowercase().cmp(&b.label.to_lowercase())
            }
        });

        suggestions
    }

    /// Fuzzy file search (without fd, using local read_dir). The real pi
    /// implementation uses `fd` for fast recursive walking. We keep the stub
    /// for the trait contract; consumers can inject their own.
    fn get_fuzzy_file_suggestions(&self, query: &str, _is_quoted: bool) -> Vec<AutocompleteItem> {
        let (raw, is_at, is_quoted) = parse_path_prefix(query);
        let expanded = expand_home_path(raw);

        // Walk the base dir (non-recursive for now).
        let search_dir = if expanded.starts_with('/') {
            PathBuf::from(&expanded)
        } else {
            self.base_path.join(&expanded)
        };

        let entries: Vec<_> = match std::fs::read_dir(&search_dir) {
            Ok(iter) => iter.filter_map(|e| e.ok()).collect(),
            Err(_) => return vec![],
        };

        let mut scored: Vec<(usize, bool, String)> = entries
            .iter()
            .filter_map(|e| {
                let is_dir = e.file_type().ok()?.is_dir();
                let name = e.file_name().to_str()?.to_string();
                let score = fuzzy_match(raw, &name).map(|m| m.score).unwrap_or(1.0);
                if score >= 0.0 {
                    return None;
                }
                Some(((-score * 1000.0) as usize, is_dir, name))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));

        scored
            .into_iter()
            .take(20)
            .map(|(_, is_dir, name)| {
                let path_value = if is_dir {
                    format!("{}/", name)
                } else {
                    name.clone()
                };
                AutocompleteItem {
                    value: build_completion_value(&path_value, is_dir, is_at, is_quoted),
                    label: if is_dir { format!("{name}/") } else { name },
                    description: Some(to_display_path(&path_value)),
                }
            })
            .collect()
    }
}

// ── path utilities ──────────────────────────────────────────────────────────

const PATH_DELIMITERS: &[char] = &[' ', '\t', '"', '\'', '='];

fn to_display_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn find_last_delimiter(text: &str) -> isize {
    for (i, ch) in text.char_indices().rev() {
        if PATH_DELIMITERS.contains(&ch) {
            return i as isize;
        }
    }
    -1
}

fn extract_quoted_prefix(text: &str) -> Option<String> {
    let quote_start = find_unclosed_quote(text)?;
    let start =
        if quote_start > 0 && text.as_bytes().get(quote_start.saturating_sub(1)) == Some(&b'@') {
            quote_start.saturating_sub(1)
        } else {
            quote_start
        };
    Some(text[start..].to_string())
}

fn find_unclosed_quote(text: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut quote_start = 0;
    for (i, ch) in text.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            if in_quotes {
                quote_start = i;
            }
        }
    }
    in_quotes.then_some(quote_start)
}

pub fn parse_path_prefix(prefix: &str) -> (&str, bool, bool) {
    if let Some(stripped) = prefix.strip_prefix("@\"") {
        (stripped, true, true)
    } else if let Some(stripped) = prefix.strip_prefix('"') {
        (stripped, false, true)
    } else if let Some(stripped) = prefix.strip_prefix('@') {
        (stripped, true, false)
    } else {
        (prefix, false, false)
    }
}

fn build_completion_value(path: &str, _is_directory: bool, is_at: bool, is_quoted: bool) -> String {
    let needs_quotes = is_quoted || path.contains(' ');
    let prefix = if is_at { "@" } else { "" };
    if !needs_quotes {
        format!("{prefix}{path}")
    } else {
        format!("{prefix}\"{path}\"")
    }
}

fn expand_home_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            let rest = path.strip_prefix("~/").unwrap_or("");
            if path.ends_with('/') {
                format!("{home}/{rest}/")
            } else {
                format!("{home}/{rest}")
            }
        } else {
            path.to_string()
        }
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        path.to_string()
    }
}

type ResolveResult = (PathBuf, String, String);

fn resolve_search_dir(base: &Path, expanded: &str, raw: &str) -> Result<ResolveResult, ()> {
    let is_root =
        raw.is_empty() || raw == "./" || raw == "../" || raw == "~" || raw == "~/" || raw == "/";

    if is_root {
        let search_dir = if raw.starts_with('~') || expanded.starts_with('/') {
            PathBuf::from(expanded)
        } else {
            base.join(expanded)
        };
        return Ok((search_dir, String::new(), raw.to_string()));
    }

    if raw.ends_with('/') {
        let search_dir = if raw.starts_with('~') || expanded.starts_with('/') {
            PathBuf::from(expanded)
        } else {
            base.join(expanded)
        };
        return Ok((search_dir, String::new(), raw.to_string()));
    }

    // Split into directory and file prefix
    let path = Path::new(expanded);
    let search_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| base.to_path_buf());
    let search_dir = if raw.starts_with('~') || raw.starts_with('/') {
        search_dir
    } else {
        base.join(search_dir)
    };
    let file_prefix = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let display_prefix = path
        .parent()
        .and_then(|p| p.to_str())
        .map(|s| format!("{}/", s.trim_end_matches('/')))
        .unwrap_or_default();

    Ok((search_dir, file_prefix, display_prefix))
}

/// Construct a display-relative path from a display prefix and entry name.
fn construct_relative_path(display_prefix: &str, name: &str) -> String {
    if display_prefix.ends_with('/') {
        format!("{display_prefix}{name}")
    } else if display_prefix.contains('/') || display_prefix.contains('\\') {
        let parent = std::path::Path::new(display_prefix)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".");
        if parent == "." {
            format!("./{name}")
        } else {
            format!("{}/{name}", parent.trim_end_matches('/'))
        }
    } else {
        name.to_string()
    }
}
