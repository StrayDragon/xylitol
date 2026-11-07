use std::sync::Arc;
use std::sync::Mutex;

use xylitol_plugin_sdk::mcp::{McpCatalogEntry, McpResourceRef};
use xylitol_tui::autocomplete::{AutocompleteItem, AutocompleteSuggestions};
use xylitol_tui::completion::{CompletionContext, CompletionMatch, CompletionSource};

use crate::mcp_attach::AttachmentTray;

pub type McpPickSink = Arc<Mutex<Vec<McpResourceRef>>>;

pub struct McpResourceSource {
    catalog: Vec<McpCatalogEntry>,
    picks: McpPickSink,
}

impl McpResourceSource {
    pub fn new(catalog: Vec<McpCatalogEntry>, picks: McpPickSink) -> Self {
        Self { catalog, picks }
    }
}

fn extract_mcp_at_prefix(before_cursor: &str) -> Option<String> {
    let start = before_cursor.rfind('@')?;
    let token = &before_cursor[start..];
    if !token.starts_with("@mcp:") || token.contains(char::is_whitespace) {
        return None;
    }
    Some(token.to_string())
}

impl CompletionSource for McpResourceSource {
    fn id(&self) -> &'static str {
        "mcp-resource"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        if self.catalog.is_empty() {
            return None;
        }
        extract_mcp_at_prefix(ctx.before_cursor()).map(|prefix| CompletionMatch { prefix })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        extract_mcp_at_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let needle = m.prefix.trim_start_matches("@mcp:").to_ascii_lowercase();
        let items: Vec<AutocompleteItem> = self
            .catalog
            .iter()
            .filter(|e| e.uri.is_some())
            .filter(|e| {
                let hay = format!(
                    "{}/{} {}",
                    e.server,
                    e.uri.as_deref().unwrap_or_default(),
                    e.name
                )
                .to_ascii_lowercase();
                needle.is_empty() || hay.contains(&needle)
            })
            .map(|e| AutocompleteItem {
                value: format!("{}\u{0}{}", e.server, e.uri.as_deref().unwrap_or_default()),
                label: format!("{}/{}", e.server, e.uri.as_deref().unwrap_or_default()),
                description: if e.name.trim().is_empty() {
                    None
                } else {
                    Some(e.name.trim().to_string())
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
        let mut picks = self.picks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((server, uri)) = item.value.split_once('\u{0}') {
            picks.push(McpResourceRef {
                server: server.to_string(),
                uri: uri.to_string(),
            });
        }
        let current = lines[cursor_line].clone();
        let cut = cursor_col.saturating_sub(prefix.len());
        let before = current[..cut].to_string();
        let after = current[cursor_col..].to_string();
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = format!("{before}{after}");
        (new_lines, cursor_line, cut)
    }
}

pub fn drain_picks_into_tray(picks: &McpPickSink, tray: &mut AttachmentTray) -> usize {
    let taken = std::mem::take(&mut *picks.lock().unwrap_or_else(|e| e.into_inner()));
    let mut added = 0usize;
    for r in taken {
        if tray.add(&r.server, &r.uri) {
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(server: &str, uri: &str, name: &str) -> McpCatalogEntry {
        McpCatalogEntry {
            server: server.into(),
            kind: "resource".into(),
            name: name.into(),
            description: String::new(),
            uri: Some(uri.into()),
            schema: None,
        }
    }

    fn ctx_of(lines: &[String], col: usize) -> CompletionContext<'_> {
        CompletionContext {
            lines,
            cursor_line: 0,
            cursor_col: col,
        }
    }

    fn one_line(line: &str) -> [String; 1] {
        [line.to_string()]
    }

    #[test]
    fn probe_requires_mcp_token() {
        let src = McpResourceSource::new(
            vec![entry("fs", "file:///a", "notes")],
            Arc::new(Mutex::new(Vec::new())),
        );
        let a = one_line("see @mcp:");
        assert!(src.probe(&ctx_of(&a, 9)).is_some());
        let b = one_line("see @src/ma");
        assert!(src.probe(&ctx_of(&b, 10)).is_none());
        let c = one_line("no token");
        assert!(src.probe(&ctx_of(&c, 8)).is_none());
        let d = one_line("@mcp:x @src");
        assert!(src.probe(&ctx_of(&d, 11)).is_none());
    }

    #[test]
    fn empty_catalog_never_probes() {
        let src = McpResourceSource::new(Vec::new(), Arc::new(Mutex::new(Vec::new())));
        let a = one_line("@mcp:");
        assert!(src.probe(&ctx_of(&a, 5)).is_none());
    }

    #[test]
    fn apply_removes_prefix_and_queues_pick() {
        let picks: McpPickSink = Arc::new(Mutex::new(Vec::new()));
        let src = McpResourceSource::new(vec![entry("fs", "file:///a", "notes")], picks.clone());
        let item = AutocompleteItem {
            value: "fs\u{0}file:///a".into(),
            label: "fs/file:///a".into(),
            description: None,
        };
        let lines = vec!["use @mcp:file".to_string()];
        let (new_lines, line, col) = src.apply(&lines, 0, 13, &item, "@mcp:file");
        assert_eq!(new_lines, vec!["use ".to_string()]);
        assert_eq!((line, col), (0, 4));
        assert_eq!(picks.lock().unwrap().len(), 1);
        assert_eq!(picks.lock().unwrap()[0].uri, "file:///a");
    }
}
