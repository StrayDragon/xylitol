//! Pluggable editor completion sources.
//!
//! The [`Editor`](crate::Editor) owns popup lifecycle (SelectList, ↑↓/Tab/Enter/Esc).
//! Applications register [`CompletionSource`]s for each trigger paradigm:
//!
//! - [`SlashCommandSource`] — `/help`, `/model`, … (line-leading, **no** space yet)
//! - [`SlashArgCompletionSource`] — `/model <prefix>` argument ids (`/cmd `…;
//!   optional bare `/cmd` via [`SlashArgCompletionSource::with_bare_command`])
//! - [`AtPathSource`] — `@path/to/file` (inline attachment)
//! - Future: `$skill`, `^agent`, etc. — implement [`CompletionSource`] and register;
//!   `$` SHOULD use [`extract_dollar_prefix`](crate::extract_dollar_prefix) so it
//!   behaves like `@` (mid-line reference), not like slash.
//!
//! ```ignore
//! editor.set_completion_sources(vec![
//!     Box::new(
//!         SlashArgCompletionSource::new("model", model_catalog)
//!             .with_id("model-id")
//!             .with_bare_command(true), // demo: exact `/model` opens catalog
//!     ),
//!     Box::new(SlashCommandSource::new(commands)),
//!     Box::new(AtPathSource::new(cwd)),
//! ]);
//! ```
//!
//! Probe order: first hit wins. With default (no bare), slash name and slash-arg
//! probes are mutually exclusive (`/`… vs `/cmd `…). With `with_bare_command(true)`,
//! register the arg source **before** slash so exact `/cmd` opens the catalog.
use crate::autocomplete::{
    AutocompleteItem, AutocompleteSuggestions, SlashCommand, expand_home_path, extract_at_prefix,
    parse_path_prefix, path_autocomplete_item,
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

    /// Probe (or keep active) and fetch suggestions. Sets `active` on success.
    ///
    /// Always re-runs [`probe_first`] so a higher-priority source can take over
    /// (e.g. slash `/mod…` → exact `/model` with bare [`SlashArgCompletionSource`]).
    ///
    /// `should_dismiss` applies only while that same source already owns the
    /// popup (e.g. slash backspaced to lone `/`). Opening on a fresh `/` still
    /// shows all commands.
    pub fn open_or_refresh(
        &mut self,
        ctx: &CompletionContext<'_>,
    ) -> Option<AutocompleteSuggestions> {
        let Some((i, m)) = self.probe_first(ctx) else {
            self.active = None;
            return None;
        };
        let src = self.sources.get(i)?;
        if self.active == Some(i) && src.should_dismiss(ctx, &m) {
            self.active = None;
            return None;
        }
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

// ── SlashArgCompletionSource ────────────────────────────────────────────────

/// `/<command> <arg-prefix>` completion (e.g. `/model dee` → model ids).
///
/// By default, bare `/<command>` (**no** trailing space) does **not** probe —
/// leave that to [`SlashCommandSource`] / product editor-slot pickers.
/// Call [`SlashArgCompletionSource::with_bare_command`] when the host wants
/// exact `/command` to open the arg catalog immediately (agent_demo).
pub struct SlashArgCompletionSource {
    command: String,
    source_id: &'static str,
    /// When true, exact `/command` (no space) also probes with an empty arg.
    bare_command: bool,
    /// `(value/label, description)` — fuzzy-filtered by the arg fragment.
    catalog: Vec<(String, String)>,
}

impl SlashArgCompletionSource {
    pub fn new(command: impl Into<String>, catalog: Vec<(String, String)>) -> Self {
        Self {
            command: command.into(),
            source_id: "slash-arg",
            bare_command: false,
            catalog,
        }
    }

    pub fn with_id(mut self, id: &'static str) -> Self {
        self.source_id = id;
        self
    }

    /// Exact `/command` opens the catalog (same as `/command ` with empty arg).
    pub fn with_bare_command(mut self, enabled: bool) -> Self {
        self.bare_command = enabled;
        self
    }

    fn arg_prefix(&self, before: &str) -> Option<String> {
        if let Some(arg) = extract_slash_arg_prefix(before, &self.command) {
            return Some(arg.to_string());
        }
        if self.bare_command {
            let bare = format!("/{}", self.command);
            if before == bare {
                return Some(String::new());
            }
        }
        None
    }
}

/// If `before` is `/<command><space><arg…>`, return the arg fragment (may be empty).
///
/// Bare `/command` with no space → [`None`].
pub fn extract_slash_arg_prefix<'a>(before: &'a str, command: &str) -> Option<&'a str> {
    if command.is_empty() || command.contains(|c: char| c.is_whitespace()) {
        return None;
    }
    let head = format!("/{command} ");
    before.strip_prefix(head.as_str())
}

impl CompletionSource for SlashArgCompletionSource {
    fn id(&self) -> &'static str {
        self.source_id
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        let arg = self.arg_prefix(ctx.before_cursor())?;
        Some(CompletionMatch {
            // Arg fragment only — Editor apply replaces `prefix.len()` before cursor.
            prefix: arg,
        })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        self.arg_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let needle = m.prefix.as_str();
        let items: Vec<AutocompleteItem> = self
            .catalog
            .iter()
            .filter_map(|(id, desc)| {
                fuzzy_match(needle, id)?;
                Some(AutocompleteItem {
                    value: id.clone(),
                    label: id.clone(),
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
        // Bare `/command` → normalize to `/command ` before writing the id.
        let head = if before == format!("/{}", self.command) {
            format!("/{} ", self.command)
        } else {
            before.to_string()
        };
        // Keep `/command ` head; write selected id; trailing space for Enter submit.
        let suffix = if after.is_empty() {
            " "
        } else if after.starts_with(' ') {
            ""
        } else {
            " "
        };
        let new_line = format!("{head}{}{suffix}{after}", item.value);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        (
            new_lines,
            cursor_line,
            head.len() + item.value.len() + suffix.len(),
        )
    }
}

// ── AtPathSource ────────────────────────────────────────────────────────────

pub struct AtPathSource {
    base_path: PathBuf,
}

impl AtPathSource {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn get_fuzzy_file_suggestions(&self, query: &str, _is_quoted: bool) -> Vec<AutocompleteItem> {
        let (raw, is_at, is_quoted) = parse_path_prefix(query);
        let expanded = expand_home_path(raw);

        // If the query looks like a directory prefix (`src/`), list that dir;
        // otherwise list base and fuzzy-filter by the file name fragment.
        // `display_prefix` is the path stem kept in `value` (pi: displayPrefix + name).
        let (search_dir, filter, display_prefix) = if expanded.ends_with('/') {
            let dir = if expanded.starts_with('/') {
                PathBuf::from(&expanded)
            } else {
                self.base_path.join(&expanded)
            };
            (dir, String::new(), expanded.clone())
        } else if let Some((parent, name)) = expanded.rsplit_once('/') {
            let dir = if parent.starts_with('/') {
                PathBuf::from(parent)
            } else if parent.is_empty() {
                self.base_path.clone()
            } else {
                self.base_path.join(parent)
            };
            let display_prefix = if parent.is_empty() {
                String::new()
            } else {
                format!("{}/", parent.trim_end_matches('/'))
            };
            (dir, name.to_string(), display_prefix)
        } else {
            (self.base_path.clone(), expanded, String::new())
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
                path_autocomplete_item(&display_prefix, &name, is_dir, is_at, is_quoted)
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
    fn at_path_keeps_dir_prefix_in_value() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("subdir");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("nested.rs"), "").unwrap();

        let src = AtPathSource::new(dir.path().to_path_buf());
        let lines = vec!["@subdir/".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 8,
        };
        let m = src.probe(&ctx).expect("probe @subdir/");
        let s = src.suggestions(&ctx, &m).expect("suggestions");
        let item = s
            .items
            .iter()
            .find(|i| i.label == "nested.rs")
            .expect("nested.rs");
        assert_eq!(item.value, "@subdir/nested.rs");
        assert_eq!(item.description.as_deref(), Some("subdir/nested.rs"));
    }

    #[test]
    fn at_path_omits_description_when_same_as_label() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("only.rs"), "").unwrap();

        let src = AtPathSource::new(dir.path().to_path_buf());
        let lines = vec!["@".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 1,
        };
        let m = src.probe(&ctx).expect("probe @");
        let s = src.suggestions(&ctx, &m).expect("suggestions");
        let item = s
            .items
            .iter()
            .find(|i| i.label == "only.rs")
            .expect("only.rs");
        assert_eq!(item.value, "@only.rs");
        assert_eq!(item.description, None);
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

    fn model_arg() -> SlashArgCompletionSource {
        SlashArgCompletionSource::new(
            "model",
            vec![
                ("deepseek-v4-flash".into(), "opencode-go".into()),
                ("deepseek/deepseek-v4-pro".into(), "commandcode".into()),
                ("grok-4.5:slow".into(), "cursor".into()),
            ],
        )
        .with_id("model-id")
    }

    #[test]
    fn slash_arg_skips_bare_command_by_default() {
        assert!(extract_slash_arg_prefix("/model", "model").is_none());
        let src = model_arg();
        let lines = vec!["/model".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 6,
        };
        assert!(
            src.probe(&ctx).is_none(),
            "bare /model must not steal picker"
        );
        assert!(slash().probe(&ctx).is_some(), "slash owns bare /model");
    }

    #[test]
    fn slash_arg_bare_command_opt_in() {
        let src = model_arg().with_bare_command(true);
        let lines = vec!["/model".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 6,
        };
        let m = src.probe(&ctx).expect("bare /model with opt-in");
        assert_eq!(m.prefix, "");
        let s = src.suggestions(&ctx, &m).unwrap();
        assert!(s.items.iter().any(|i| i.value == "deepseek-v4-flash"));
        let item = s.items[0].clone();
        let (new_lines, _, col) = src.apply(&lines, 0, 6, &item, &m.prefix);
        assert!(
            new_lines[0].starts_with("/model "),
            "bare apply must insert space; got {}",
            new_lines[0]
        );
        assert!(new_lines[0].contains(&item.value));
        assert_eq!(col, new_lines[0].len());
    }

    #[test]
    fn slash_arg_fuzzy_and_apply() {
        let src = model_arg();
        let lines = vec!["/model dee".into()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 10,
        };
        let m = src.probe(&ctx).expect("probe /model dee");
        assert_eq!(m.prefix, "dee");
        let s = src.suggestions(&ctx, &m).unwrap();
        assert!(
            s.items.iter().any(|i| i.value.contains("deepseek")),
            "fuzzy dee → deepseek*; got {:?}",
            s.items.iter().map(|i| i.value.as_str()).collect::<Vec<_>>()
        );
        assert!(!s.items.iter().any(|i| i.value.starts_with("grok")));

        let item = s
            .items
            .iter()
            .find(|i| i.value == "deepseek-v4-flash")
            .cloned()
            .expect("flash in list");
        let (new_lines, _, col) = src.apply(&lines, 0, 10, &item, &m.prefix);
        assert_eq!(new_lines[0], "/model deepseek-v4-flash ");
        assert_eq!(col, "/model deepseek-v4-flash ".len());
    }

    #[test]
    fn slash_arg_and_slash_mutex_in_registry() {
        let mut reg = CompletionRegistry::new();
        reg.set_sources(vec![Box::new(model_arg()), Box::new(slash())]);

        let bare = vec!["/model".into()];
        let ctx = CompletionContext {
            lines: &bare,
            cursor_line: 0,
            cursor_col: 6,
        };
        let (i, _) = reg.probe_first(&ctx).unwrap();
        assert_eq!(reg.sources[i].id(), "slash");

        let arg = vec!["/model dee".into()];
        let ctx = CompletionContext {
            lines: &arg,
            cursor_line: 0,
            cursor_col: 10,
        };
        let (i, m) = reg.probe_first(&ctx).unwrap();
        assert_eq!(reg.sources[i].id(), "model-id");
        assert_eq!(m.prefix, "dee");
    }

    #[test]
    fn open_or_refresh_switches_to_higher_priority_source() {
        let mut reg = CompletionRegistry::new();
        reg.set_sources(vec![
            Box::new(model_arg().with_bare_command(true)),
            Box::new(slash()),
        ]);

        // Start on slash while typing `/mod`.
        let mid = vec!["/mod".into()];
        let ctx = CompletionContext {
            lines: &mid,
            cursor_line: 0,
            cursor_col: 4,
        };
        let s = reg.open_or_refresh(&ctx).unwrap();
        assert!(s.items.iter().any(|i| i.value == "model"));
        assert_eq!(reg.active_index(), Some(1));

        // Exact `/model` must hand off to bare model-id without cancel+reopen.
        let exact = vec!["/model".into()];
        let ctx = CompletionContext {
            lines: &exact,
            cursor_line: 0,
            cursor_col: 6,
        };
        let s = reg.open_or_refresh(&ctx).unwrap();
        assert!(s.items.iter().any(|i| i.value == "deepseek-v4-flash"));
        assert_eq!(reg.active_index(), Some(0));
    }
}
