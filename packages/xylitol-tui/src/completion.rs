//! Pluggable editor completion sources.
//!
//! The [`Editor`](crate::Editor) owns popup lifecycle (SelectList, ↑↓/Tab/Enter/Esc).
//! Applications register [`CompletionSource`]s for each trigger paradigm:
//!
//! - [`SlashCommandSource`] — `/help`, `/model`, … (line-leading)
//! - [`AtPathSource`] — `@path/to/file` (inline attachment)
//! - Future: `$skill`, `^agent`, etc. — implement [`CompletionSource`] and register;
//!   `$` SHOULD use [`extract_dollar_prefix`](crate::extract_dollar_prefix) so it
//!   behaves like `@` (mid-line reference), not like slash.
//!
//! ```ignore
//! editor.set_completion_sources(vec![
//!     Box::new(SlashCommandSource::new(commands)),
//!     Box::new(AtPathSource::new(cwd)),
//!     // Box::new(DollarSkillSource::new(skills)),
//! ]);
//! ```

use crate::autocomplete::{
    AutocompleteItem, AutocompleteSuggestions, SlashCommand, build_completion_value,
    expand_home_path, extract_at_prefix, parse_path_prefix, to_display_path,
};
use crate::fuzzy::fuzzy_match;
use std::path::PathBuf;

// ── context / match ─────────────────────────────────────────────────────────

pub struct CompletionContext<'a> {
    pub lines: &'a [String],
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl CompletionContext<'_> {
    pub fn before_cursor(&self) -> &str {
        let line = self
            .lines
            .get(self.cursor_line)
            .map(String::as_str)
            .unwrap_or("");
        &line[..self.cursor_col.min(line.len())]
    }
}

pub struct CompletionMatch {
    /// Text being completed (e.g. "/hel", "@src/").
    pub prefix: String,
}

// ── trait ───────────────────────────────────────────────────────────────────

/// One completable trigger paradigm (`/`, `@`, future `$` / `^`, …).
///
/// Editor probes sources in registry order; first [`probe`](Self::probe) hit wins.
pub trait CompletionSource: Send {
    fn id(&self) -> &'static str;

    /// Does this source own the cursor context?
    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch>;

    /// While popup is open for this source: close without editing when true
    /// (e.g. slash backspaced to lone `/`).
    fn should_dismiss(&self, ctx: &CompletionContext<'_>, m: &CompletionMatch) -> bool;

    fn suggestions(
        &self,
        ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions>;

    fn apply(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> (Vec<String>, usize, usize);
}

// ── registry ────────────────────────────────────────────────────────────────

pub struct CompletionRegistry {
    sources: Vec<Box<dyn CompletionSource>>,
    /// Index of the source that owns the open popup.
    active: Option<usize>,
}

impl Default for CompletionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            active: None,
        }
    }

    pub fn set_sources(&mut self, sources: Vec<Box<dyn CompletionSource>>) {
        self.sources = sources;
        self.active = None;
    }

    pub fn clear(&mut self) {
        self.sources.clear();
        self.active = None;
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn clear_active(&mut self) {
        self.active = None;
    }

    pub fn set_active(&mut self, index: usize) {
        self.active = Some(index);
    }

    /// Probe all sources in order; return first match and its index.
    pub fn probe_first(&self, ctx: &CompletionContext<'_>) -> Option<(usize, CompletionMatch)> {
        for (i, src) in self.sources.iter().enumerate() {
            if let Some(m) = src.probe(ctx) {
                return Some((i, m));
            }
        }
        None
    }

    pub fn should_dismiss_active(&self, ctx: &CompletionContext<'_>) -> bool {
        let Some(i) = self.active else {
            return true;
        };
        let Some(src) = self.sources.get(i) else {
            return true;
        };
        match src.probe(ctx) {
            None => true,
            Some(m) => src.should_dismiss(ctx, &m),
        }
    }

    pub fn suggestions_for_active(
        &self,
        ctx: &CompletionContext<'_>,
    ) -> Option<AutocompleteSuggestions> {
        let i = self.active?;
        let src = self.sources.get(i)?;
        let m = src.probe(ctx)?;
        if src.should_dismiss(ctx, &m) {
            return None;
        }
        src.suggestions(ctx, &m)
    }

    /// Probe (or keep active) and fetch suggestions. Sets `active` on success.
    ///
    /// `should_dismiss` applies only while a popup is already open (e.g. slash
    /// backspaced to lone `/`). Opening on a fresh `/` still shows all commands.
    pub fn open_or_refresh(
        &mut self,
        ctx: &CompletionContext<'_>,
    ) -> Option<AutocompleteSuggestions> {
        if let Some(i) = self.active {
            let src = self.sources.get(i)?;
            match src.probe(ctx) {
                None => {
                    self.active = None;
                    return None;
                }
                Some(m) if src.should_dismiss(ctx, &m) => {
                    self.active = None;
                    return None;
                }
                Some(m) => return src.suggestions(ctx, &m),
            }
        }
        let (i, m) = self.probe_first(ctx)?;
        let src = self.sources.get(i)?;
        let s = src.suggestions(ctx, &m)?;
        self.active = Some(i);
        Some(s)
    }

    pub fn apply_active(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> Option<(Vec<String>, usize, usize)> {
        let i = self.active?;
        let src = self.sources.get(i)?;
        Some(src.apply(lines, cursor_line, cursor_col, item, prefix))
    }

    /// Apply using whichever source currently probes (for Tab auto-apply before popup).
    pub fn apply_probed(
        &self,
        source_index: usize,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> (Vec<String>, usize, usize) {
        self.sources[source_index].apply(lines, cursor_line, cursor_col, item, prefix)
    }
}

// ── SlashCommandSource ──────────────────────────────────────────────────────

pub struct SlashCommandSource {
    commands: Vec<(String, String)>,
}

impl SlashCommandSource {
    pub fn new(commands: Vec<SlashCommand>) -> Self {
        Self {
            commands: commands
                .into_iter()
                .map(|c| (c.name, c.description.unwrap_or_default()))
                .collect(),
        }
    }
}

impl CompletionSource for SlashCommandSource {
    fn id(&self) -> &'static str {
        "slash"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        let before = ctx.before_cursor();
        if before.starts_with('/') && !before.contains(' ') {
            Some(CompletionMatch {
                prefix: before.to_string(),
            })
        } else {
            None
        }
    }

    fn should_dismiss(&self, _ctx: &CompletionContext<'_>, m: &CompletionMatch) -> bool {
        // Lone `/` keeps the character but closes the popup.
        m.prefix == "/"
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let prefix = m.prefix.strip_prefix('/').unwrap_or("");
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
        Some(AutocompleteSuggestions {
            items,
            prefix: m.prefix.clone(),
        })
    }

    fn apply(
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
        let new_line = format!("/{} {}", item.value, after);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        (new_lines, cursor_line, before.len() + item.value.len() + 2)
    }
}

// ── AtPathSource ────────────────────────────────────────────────────────────

pub struct AtPathSource {
    base_path: PathBuf,
    #[allow(dead_code)] // reserved for fd-backed fuzzy (same as Combined)
    fd_path: Option<String>,
}

impl AtPathSource {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            fd_path: None,
        }
    }

    pub fn new_with_fd(base_path: PathBuf, fd_path: String) -> Self {
        Self {
            base_path,
            fd_path: Some(fd_path),
        }
    }

    fn get_fuzzy_file_suggestions(&self, query: &str, _is_quoted: bool) -> Vec<AutocompleteItem> {
        let (raw, is_at, is_quoted) = parse_path_prefix(query);
        let expanded = expand_home_path(raw);

        // If the query looks like a directory prefix (`src/`), list that dir;
        // otherwise list base and fuzzy-filter by the file name fragment.
        let (search_dir, filter) = if expanded.ends_with('/') {
            let dir = if expanded.starts_with('/') {
                PathBuf::from(&expanded)
            } else {
                self.base_path.join(&expanded)
            };
            (dir, String::new())
        } else if let Some((parent, name)) = expanded.rsplit_once('/') {
            let dir = if parent.starts_with('/') {
                PathBuf::from(parent)
            } else if parent.is_empty() {
                self.base_path.clone()
            } else {
                self.base_path.join(parent)
            };
            (dir, name.to_string())
        } else {
            (self.base_path.clone(), expanded)
        };

        let entries: Vec<_> = match std::fs::read_dir(&search_dir) {
            Ok(iter) => iter.filter_map(|e| e.ok()).collect(),
            Err(_) => return vec![],
        };

        let mut scored: Vec<(f64, bool, String)> = entries
            .iter()
            .filter_map(|e| {
                let is_dir = e.file_type().ok()?.is_dir();
                let name = e.file_name().to_str()?.to_string();
                if filter.is_empty() {
                    return Some((0.0, is_dir, name));
                }
                let m = fuzzy_match(&filter, &name)?;
                Some((m.score, is_dir, name))
            })
            .collect();

        scored.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        scored
            .into_iter()
            .take(20)
            .map(|(_, is_dir, name)| {
                let path_value = if is_dir {
                    format!("{name}/")
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

impl CompletionSource for AtPathSource {
    fn id(&self) -> &'static str {
        "at-path"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        extract_at_prefix(ctx.before_cursor()).map(|prefix| CompletionMatch { prefix })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        // Dismiss when `@` token is gone. Bare `@` stays open (list cwd).
        extract_at_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let (_raw, _is_at, is_quoted) = parse_path_prefix(&m.prefix);
        let items = self.get_fuzzy_file_suggestions(&m.prefix, is_quoted);
        if items.is_empty() {
            return None;
        }
        Some(AutocompleteSuggestions {
            items,
            prefix: m.prefix.clone(),
        })
    }

    fn apply(
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
        (
            new_lines,
            cursor_line,
            before.len() + cursor_offset + suffix.len(),
        )
    }
}

/// Build the default slash + `@` source pair from a Combined-style config.
pub fn sources_from_combined(
    commands: Vec<SlashCommand>,
    base_path: PathBuf,
    fd_path: Option<String>,
) -> Vec<Box<dyn CompletionSource>> {
    let at = match fd_path {
        Some(fd) => AtPathSource::new_with_fd(base_path, fd),
        None => AtPathSource::new(base_path),
    };
    vec![Box::new(SlashCommandSource::new(commands)), Box::new(at)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slash() -> SlashCommandSource {
        SlashCommandSource::new(vec![
            SlashCommand {
                name: "help".into(),
                description: Some("Show help".into()),
                argument_hint: None,
                get_argument_completions: None,
            },
            SlashCommand {
                name: "model".into(),
                description: Some("Switch model".into()),
                argument_hint: None,
                get_argument_completions: None,
            },
        ])
    }

    #[test]
    fn slash_probe_and_dismiss() {
        let src = slash();
        let lines = vec!["/hel".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 4,
        };
        let m = src.probe(&ctx).expect("probe /hel");
        assert_eq!(m.prefix, "/hel");
        assert!(!src.should_dismiss(&ctx, &m));

        let lines = vec!["/".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 1,
        };
        let m = src.probe(&ctx).expect("probe /");
        assert!(src.should_dismiss(&ctx, &m));
    }

    #[test]
    fn slash_suggestions_filter() {
        let src = slash();
        let lines = vec!["/hel".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 4,
        };
        let m = src.probe(&ctx).unwrap();
        let s = src.suggestions(&ctx, &m).unwrap();
        assert!(s.items.iter().any(|i| i.value == "help"));
        assert!(!s.items.iter().any(|i| i.value == "model"));
    }

    #[test]
    fn at_path_probe() {
        let src = AtPathSource::new(PathBuf::from("."));
        let lines = vec!["see @foo".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 8,
        };
        let m = src.probe(&ctx).expect("probe @foo");
        assert_eq!(m.prefix, "@foo");
    }

    #[test]
    fn registry_first_wins() {
        let mut reg = CompletionRegistry::new();
        reg.set_sources(vec![
            Box::new(slash()),
            Box::new(AtPathSource::new(PathBuf::from("."))),
        ]);
        let lines = vec!["/h".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 2,
        };
        let (i, m) = reg.probe_first(&ctx).unwrap();
        assert_eq!(i, 0);
        assert_eq!(m.prefix, "/h");
        assert_eq!(reg.sources[i].id(), "slash");
    }
}
