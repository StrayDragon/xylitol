use std::sync::Arc;
use std::sync::Mutex;

use xylitol_plugin_sdk::mcp::{McpCatalogEntry, McpResourceRef};
use xylitol_tui::completion::{CompletionContext, CompletionMatch, CompletionSource};
use xylitol_tui::{AutocompleteItem, AutocompleteSuggestions};

use crate::mcp_attach::AttachmentTray;

pub type McpPickSink = Arc<Mutex<Vec<McpResourceRef>>>;

pub type McpCatalogWaker = Arc<dyn Fn() + Send + Sync>;

pub type McpCatalogStatusSlot = Arc<Mutex<crate::driver::McpCatalogState>>;

#[derive(Clone)]
pub struct McpCatalogLazy {
    entries: Arc<Mutex<Vec<McpCatalogEntry>>>,
    status: McpCatalogStatusSlot,
    waker: Arc<Mutex<Option<McpCatalogWaker>>>,
}

impl McpCatalogLazy {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(Mutex::new(crate::driver::McpCatalogState::Missing)),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    pub fn entries(&self) -> Vec<McpCatalogEntry> {
        self.entries.lock().map(|e| e.clone()).unwrap_or_default()
    }

    pub fn status(&self) -> crate::driver::McpCatalogState {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(crate::driver::McpCatalogState::Missing)
    }

    pub fn set_entries(&self, entries: Vec<McpCatalogEntry>) {
        if let Ok(mut slot) = self.entries.lock() {
            *slot = entries;
        }
    }

    pub fn set_status(&self, status: crate::driver::McpCatalogState) {
        if let Ok(mut slot) = self.status.lock() {
            *slot = status;
        }
    }

    pub fn set_waker(&self, waker: McpCatalogWaker) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(waker);
        }
    }

    pub fn source(&self, picks: McpPickSink) -> McpResourceSource {
        McpResourceSource {
            catalog: self.entries.clone(),
            picks,
            waker: self.waker.lock().ok().and_then(|w| w.clone()),
            status: Some(self.status.clone()),
        }
    }
}

pub struct McpResourceSource {
    catalog: Arc<Mutex<Vec<McpCatalogEntry>>>,
    picks: McpPickSink,
    waker: Option<McpCatalogWaker>,
    status: Option<McpCatalogStatusSlot>,
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
        let prefix = extract_mcp_at_prefix(ctx.before_cursor())?;
        let empty = self.catalog.lock().map(|c| c.is_empty()).unwrap_or(true);
        if empty && let Some(waker) = &self.waker {
            waker();
        }
        Some(CompletionMatch { prefix })
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
        let catalog = self.catalog.lock().map(|c| c.clone()).unwrap_or_default();
        let items: Vec<AutocompleteItem> = catalog
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
            let state = self
                .status
                .as_ref()
                .and_then(|s| s.lock().ok().map(|v| v.clone()));
            let note = match state {
                Some(crate::driver::McpCatalogState::Loading) => "loading mcp catalog…",
                Some(crate::driver::McpCatalogState::Failed(reason)) => {
                    return Some(AutocompleteSuggestions {
                        items: vec![AutocompleteItem {
                            value: String::new(),
                            label: format!("mcp catalog unavailable: {reason}"),
                            description: None,
                        }],
                        prefix: m.prefix.clone(),
                    });
                }
                _ => return None,
            };
            return Some(AutocompleteSuggestions {
                items: vec![AutocompleteItem {
                    value: String::new(),
                    label: note.to_string(),
                    description: None,
                }],
                prefix: m.prefix.clone(),
            });
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
        let lazy = McpCatalogLazy::new();
        lazy.set_entries(vec![entry("fs", "file:///a", "notes")]);
        let src = lazy.source(Arc::new(Mutex::new(Vec::new())));
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
    fn empty_catalog_probes_and_wakes_the_loader() {
        let woken = Arc::new(Mutex::new(0usize));
        let flag = woken.clone();
        let lazy = McpCatalogLazy::new();
        lazy.set_waker(Arc::new(move || {
            *flag.lock().unwrap_or_else(|e| e.into_inner()) += 1;
        }));
        let src = lazy.source(Arc::new(Mutex::new(Vec::new())));
        let a = one_line("@mcp:");
        assert!(
            src.probe(&ctx_of(&a, 5)).is_some(),
            "an empty catalog must still claim the token so the lazy fetch can start"
        );
        assert_eq!(
            *woken.lock().unwrap_or_else(|e| e.into_inner()),
            1,
            "probing an empty catalog must wake the background fetch"
        );
        let b = one_line("no token");
        assert!(src.probe(&ctx_of(&b, 8)).is_none());
    }

    #[test]
    fn apply_removes_prefix_and_queues_pick() {
        let picks: McpPickSink = Arc::new(Mutex::new(Vec::new()));
        let lazy = McpCatalogLazy::new();
        lazy.set_entries(vec![entry("fs", "file:///a", "notes")]);
        let src = lazy.source(picks.clone());
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
