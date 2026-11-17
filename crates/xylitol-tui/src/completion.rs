use crate::autocomplete_fd::walk_directory_with_fd;
use crate::fuzzy::fuzzy_match;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

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
    pub prefix: String,
}

pub trait CompletionSource: Send {
    fn id(&self) -> &'static str;

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch>;

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

pub struct CompletionRegistry {
    sources: Vec<Box<dyn CompletionSource>>,
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

    pub fn apply_probed(
        &self,
        source_index: usize,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> Option<(Vec<String>, usize, usize)> {
        let src = self.sources.get(source_index)?;
        Some(src.apply(lines, cursor_line, cursor_col, item, prefix))
    }
}

pub struct SlashCommandSource {
    commands: Vec<(String, String)>,
    show_on_bare_slash: bool,
}

impl SlashCommandSource {
    pub fn new(commands: Vec<SlashCommand>) -> Self {
        Self {
            commands: commands
                .into_iter()
                .map(|c| (c.name, c.description.unwrap_or_default()))
                .collect(),
            show_on_bare_slash: false,
        }
    }

    pub fn show_on_bare_slash(mut self) -> Self {
        self.show_on_bare_slash = true;
        self
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
        !self.show_on_bare_slash && m.prefix == "/"
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

pub struct SlashArgCompletionSource {
    command: String,
    source_id: &'static str,
    bare_command: bool,
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
        Some(CompletionMatch { prefix: arg })
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
        let head = if before == format!("/{}", self.command) {
            format!("/{} ", self.command)
        } else {
            before.to_string()
        };
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

pub fn fd_path_suggestions(
    base_path: &Path,
    fd_path: &str,
    query: &str,
    max_results: usize,
) -> Vec<AutocompleteItem> {
    let (raw, is_at, is_quoted) = parse_path_prefix(query);
    let expanded = expand_home_path(raw);
    if expanded.trim().is_empty() {
        return Vec::new();
    }
    let base = base_path.to_string_lossy().to_string();
    let entries = walk_directory_with_fd(&base, fd_path, &expanded, 100, CancellationToken::new());
    let filter = expanded.rsplit('/').next().unwrap_or(expanded.as_str());
    let mut scored: Vec<(f64, bool, String)> = entries
        .into_iter()
        .filter_map(|(path, is_dir)| {
            let name = path.rsplit('/').next().unwrap_or(path.as_str());
            if filter.is_empty() {
                Some((0.0, is_dir, path))
            } else {
                fuzzy_match(filter, name).map(|m| (m.score, is_dir, path))
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.2.cmp(&b.2))
    });
    scored
        .into_iter()
        .take(max_results.min(20))
        .map(|(_, is_dir, path)| path_autocomplete_item("", &path, is_dir, is_at, is_quoted))
        .collect()
}

pub struct AtPathSource {
    base_path: PathBuf,
    fd_path: Option<String>,
}

impl AtPathSource {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            fd_path: None,
        }
    }

    pub fn new_with_fd(base_path: PathBuf, fd_path: Option<String>) -> Self {
        Self { base_path, fd_path }
    }

    fn get_fuzzy_file_suggestions(&self, query: &str, _is_quoted: bool) -> Vec<AutocompleteItem> {
        let (raw, is_at, is_quoted) = parse_path_prefix(query);
        let expanded = expand_home_path(raw);

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
        extract_at_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let (_raw, _is_at, is_quoted) = parse_path_prefix(&m.prefix);
        let items = self
            .fd_path
            .as_deref()
            .map(|fd| fd_path_suggestions(&self.base_path, fd, &m.prefix, 20))
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| self.get_fuzzy_file_suggestions(&m.prefix, is_quoted));
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

    #[test]
    fn apply_probed_out_of_bounds_is_noop() {
        let reg = CompletionRegistry::new();
        let lines = vec!["/he".to_string()];
        let item = AutocompleteItem {
            value: "help".into(),
            label: "help".into(),
            description: None,
        };
        assert!(reg.apply_probed(0, &lines, 0, 3, &item, "/").is_none());
    }

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

        let shown = slash().show_on_bare_slash();
        assert!(!shown.should_dismiss(&ctx, &m));
        let all = shown.suggestions(&ctx, &m).expect("bare / lists catalog");
        assert_eq!(all.items[0].value, "help");
        assert_eq!(all.items[1].value, "model");
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

        let mid = vec!["/mod".into()];
        let ctx = CompletionContext {
            lines: &mid,
            cursor_line: 0,
            cursor_col: 4,
        };
        let s = reg.open_or_refresh(&ctx).unwrap();
        assert!(s.items.iter().any(|i| i.value == "model"));
        assert_eq!(reg.active_index(), Some(1));

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

#[cfg(test)]
mod fd_path_tests {
    use super::*;

    #[test]
    fn fd_recalls_deep_path_when_available() {
        let available = std::process::Command::new("fd")
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success());
        if !available {
            return;
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("src").join("deep");
        std::fs::create_dir_all(&nested).expect("dirs");
        std::fs::write(nested.join("utils.rs"), "fn x() {}").expect("file");
        let items = fd_path_suggestions(dir.path(), "fd", "@src/deep/uti", 20);
        assert!(
            items
                .iter()
                .any(|item| item.value.contains("src/deep/utils.rs")),
            "fd must recall deep path: {:?}",
            items
                .iter()
                .map(|item| item.value.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn missing_fd_falls_back_to_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let items = fd_path_suggestions(dir.path(), "__no_such_fd__", "@a", 20);
        assert!(items.is_empty());
    }

    #[test]
    fn fd_recalls_cjk_filename() {
        let available = std::process::Command::new("fd")
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success());
        if !available {
            return;
        }
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("中文.rs"), "fn x() {}").expect("file");
        let items = fd_path_suggestions(dir.path(), "fd", "@中文", 20);
        assert!(
            items.iter().any(|item| item.value.contains("中文.rs")),
            "fd must recall CJK filename: {:?}",
            items
                .iter()
                .map(|item| item.value.clone())
                .collect::<Vec<_>>()
        );
    }
}

#[derive(Clone)]
pub struct AutocompleteItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[allow(clippy::type_complexity)]
pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    pub get_argument_completions: Option<Box<dyn Fn(&str) -> Option<Vec<AutocompleteItem>>>>,
}

pub struct AutocompleteSuggestions {
    pub items: Vec<AutocompleteItem>,
    pub prefix: String,
}

pub(crate) const PATH_DELIMITERS: &[char] = &[' ', '\t', '"', '\'', '='];

pub(crate) fn to_display_path(value: &str) -> String {
    value.replace('\\', "/")
}

pub fn extract_at_prefix(text: &str) -> Option<String> {
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

pub fn extract_dollar_prefix(text: &str) -> Option<String> {
    let delim = find_last_delimiter(text);
    let start = if delim < 0 { 0 } else { (delim + 1) as usize };
    if text[start..].starts_with('$') {
        Some(text[start..].to_string())
    } else {
        None
    }
}

pub(crate) fn find_last_delimiter(text: &str) -> isize {
    for (i, ch) in text.char_indices().rev() {
        if PATH_DELIMITERS.contains(&ch) {
            return i as isize;
        }
    }
    -1
}

pub(crate) fn extract_quoted_prefix(text: &str) -> Option<String> {
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

pub(crate) fn build_completion_value(
    path: &str,
    _is_directory: bool,
    is_at: bool,
    is_quoted: bool,
) -> String {
    let needs_quotes = is_quoted || path.contains(' ');
    let prefix = if is_at { "@" } else { "" };
    if !needs_quotes {
        format!("{prefix}{path}")
    } else {
        format!("{prefix}\"{path}\"")
    }
}

pub(crate) fn expand_home_path(path: &str) -> String {
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

pub(crate) fn construct_relative_path(display_prefix: &str, name: &str) -> String {
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

pub(crate) fn path_autocomplete_item(
    display_prefix: &str,
    name: &str,
    is_dir: bool,
    is_at: bool,
    is_quoted: bool,
) -> AutocompleteItem {
    let relative = construct_relative_path(display_prefix, name);
    let display = to_display_path(&relative);
    let path_value = if is_dir {
        if display.ends_with('/') {
            display
        } else {
            format!("{display}/")
        }
    } else {
        display
    };
    let label = if is_dir {
        format!("{name}/")
    } else {
        name.to_string()
    };
    let description = if path_value == label {
        None
    } else {
        Some(path_value.clone())
    };
    AutocompleteItem {
        value: build_completion_value(&path_value, is_dir, is_at, is_quoted),
        label,
        description,
    }
}

#[cfg(test)]
mod path_item_tests {
    use super::path_autocomplete_item;

    #[test]
    fn root_entry_has_no_duplicate_description() {
        let item = path_autocomplete_item("", "foo.rs", false, true, false);
        assert_eq!(item.value, "@foo.rs");
        assert_eq!(item.label, "foo.rs");
        assert_eq!(item.description, None);
    }

    #[test]
    fn nested_entry_keeps_prefix_and_shows_relative_desc() {
        let item = path_autocomplete_item(".git/", "COMMIT_EDITMSG", false, true, false);
        assert_eq!(item.value, "@.git/COMMIT_EDITMSG");
        assert_eq!(item.label, "COMMIT_EDITMSG");
        assert_eq!(item.description.as_deref(), Some(".git/COMMIT_EDITMSG"));
    }

    #[test]
    fn nested_dir_keeps_trailing_slash_on_value() {
        let item = path_autocomplete_item(".git/", "objects", true, true, false);
        assert_eq!(item.value, "@.git/objects/");
        assert_eq!(item.label, "objects/");
        assert_eq!(item.description.as_deref(), Some(".git/objects/"));
    }
}

#[cfg(test)]
mod dollar_prefix_tests {
    use super::extract_dollar_prefix;

    #[test]
    fn extracts_inline_dollar_after_words() {
        assert_eq!(extract_dollar_prefix("use $"), Some("$".into()));
        assert_eq!(extract_dollar_prefix("use $dem"), Some("$dem".into()));
        assert_eq!(extract_dollar_prefix("$alone"), Some("$alone".into()));
    }

    #[test]
    fn ignores_when_no_dollar_token() {
        assert_eq!(extract_dollar_prefix("use skill"), None);
        assert_eq!(extract_dollar_prefix("price is 5$"), None);
        assert_eq!(extract_dollar_prefix("use $demo more"), None);
    }
}
