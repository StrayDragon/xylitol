//! Product `$skill` completion source (c1130 / A10).
//!
//! Catalog comes from Trust-filtered `loaded_skills`; probe/apply mirror `@` / demo.

use xylitol_tui::autocomplete::{AutocompleteItem, AutocompleteSuggestions};
use xylitol_tui::completion::{CompletionContext, CompletionMatch, CompletionSource};
use xylitol_tui::extract_dollar_prefix;

/// Inline `$name` completion for the product editor.
pub struct DollarSkillSource {
    /// `(name, description)` — description may be empty.
    catalog: Vec<(String, String)>,
}

impl DollarSkillSource {
    pub fn new(catalog: Vec<(String, String)>) -> Self {
        Self { catalog }
    }
}

impl CompletionSource for DollarSkillSource {
    fn id(&self) -> &'static str {
        "dollar-skill"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        extract_dollar_prefix(ctx.before_cursor()).map(|prefix| CompletionMatch { prefix })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        extract_dollar_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let needle = m.prefix.strip_prefix('$').unwrap_or("");
        let items: Vec<AutocompleteItem> = self
            .catalog
            .iter()
            .filter(|(name, _)| name.starts_with(needle))
            .map(|(name, desc)| AutocompleteItem {
                value: format!("${name}"),
                label: name.clone(),
                description: if desc.is_empty() {
                    None
                } else {
                    Some(desc.clone())
                },
            })
            .collect();
        if items.is_empty() {
            None
        } else {
            Some(AutocompleteSuggestions {
                items,
                prefix: m.prefix.clone(),
            })
        }
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
        let suffix = " ";
        let new_line = format!("{}{}{}{}", before, item.value, suffix, after);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        (
            new_lines,
            cursor_line,
            before.len() + item.value.len() + suffix.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_tui::completion::CompletionContext;

    #[test]
    fn suggestions_filter_by_prefix() {
        let src = DollarSkillSource::new(vec![
            ("demo".into(), "d".into()),
            ("other".into(), String::new()),
        ]);
        let lines = ["use $de".to_string()];
        let ctx = CompletionContext {
            lines: &lines,
            cursor_line: 0,
            cursor_col: 7,
        };
        let m = src.probe(&ctx).expect("probe");
        let sug = src.suggestions(&ctx, &m).expect("suggestions");
        assert_eq!(sug.items.len(), 1);
        assert_eq!(sug.items[0].value, "$demo");
    }
}
