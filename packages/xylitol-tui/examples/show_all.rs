//! xylitol-tui 全功能演示 — "kitchen sink" demo
//!
//! 运行：`cargo run -p xylitol-tui --example show_all`
//!
//! 展示：Editor / SelectList file browser / Panel / Markdown / Loader /
//! SettingsList / Keybindings / Text
//! 按 j/k 或 ↑↓ 导航，Enter 激活视图，q 退出。

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::autocomplete::CombinedAutocompleteProvider;
use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::markdown::{Markdown, MarkdownOptions, MarkdownTheme};
use xylitol_tui::components::panel::Panel;
use xylitol_tui::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
use xylitol_tui::components::settings_list::{
    SettingItem, SettingsList, SettingsListOptions, SettingsListTheme,
};
use xylitol_tui::components::text::Text;
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{Component, CrosstermTerminal, Focusable, SystemClock, TUI, visible_width};

fn cyan(s: &str) -> String {
    format!("\x1b[36m{s}\x1b[39m")
}
fn red(s: &str) -> String {
    format!("\x1b[31m{s}\x1b[39m")
}
fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[22m")
}
fn bold(s: &str) -> String {
    format!("\x1b[1m{s}\x1b[22m")
}
fn white_on_blue(s: &str) -> String {
    format!("\x1b[44m\x1b[37m{s}\x1b[49m\x1b[39m")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));
    let term = CrosstermTerminal::new()?;
    let mut tui = TUI::new(term);
    let quit_flag = Arc::new(AtomicBool::new(false));
    let app = ShowAllApp::new(quit_flag.clone());
    tui.add_child(Box::new(app));
    tui.set_focus(Some(0));
    tui.start_with_flag(&quit_flag)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Home,
    Edit,
    Files,
    Settings,
    Help,
}

const HOME_MD: &str = "\
# 🦀 xylitol-tui Show All

A **flicker-free** terminal UI library — differential rendering,
composable widgets, and full keyboard control.

## Features

| Feature | Status |
|---------|--------|
| **Editor** | multi-line, undo, kill-ring, history, jump, PasteBurst |
| **Autocomplete** | file-path + slash-command with fd recursive search |
| **SelectList** | keyboard-navigable lists (file browser) |
| **Panel** | padded containers with background colors |
| **Markdown** | pulldown-cmark renderer with theme hooks |
| **Loader** | animated spinner |
| **SettingsList** | configurable settings list |
| **Keybindings** | emacs-style defaults, fully customizable |

## Controls

j/k or ↑/↓ — navigate views  |  Enter — activate  |  q — quit

Press **Enter** on any view to explore.
";

const HELP_MD: &str = "\
# Key Bindings Reference

## Editor

| Key | Action |
|-----|--------|
| `Ctrl+u` | delete to line start |
| `Ctrl+k` | delete to line end |
| `Ctrl+w` | delete word backward |
| `Alt+d` | delete word forward |
| `Ctrl+y` | yank (paste) |
| `Alt+y` | yank-pop (cycle) |
| `Ctrl+/` | undo |
| `Ctrl+f` / `Ctrl+b` | jump forward/backward |
| `Ctrl+a` / `Ctrl+e` | line start / end |
| `Alt+f` / `Alt+b` | word forward / backward |
| `PgUp` / `PgDn` | page scroll |
| `Tab` | autocomplete |

## Global

| Key | Action |
|-----|--------|
| `q` | quit |
| `Esc` | back to home |
";

fn md_theme() -> MarkdownTheme {
    MarkdownTheme {
        heading: Box::new(bold),
        link: Box::new(cyan),
        link_url: Box::new(dim),
        code: Box::new(|s| format!("\x1b[33m{s}\x1b[39m")),
        code_block: Box::new(dim),
        code_block_border: Box::new(dim),
        quote: Box::new(dim),
        quote_border: Box::new(dim),
        hr: Box::new(dim),
        list_bullet: Box::new(cyan),
        bold: Box::new(bold),
        italic: Box::new(dim),
        strikethrough: Box::new(dim),
        underline: Box::new(|s| format!("\x1b[4m{s}\x1b[24m")),
        highlight_code: None,
        code_block_indent: None,
    }
}

fn sl_theme() -> SelectListTheme {
    SelectListTheme {
        selected_prefix: Box::new(cyan),
        selected_text: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
        description: Box::new(dim),
        scroll_info: Box::new(dim),
        no_match: Box::new(red),
    }
}

fn build_file_browser(dir: &PathBuf) -> SelectList {
    let mut items = vec![SelectItem::new("..", "../").with_description("parent directory")];
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut ents: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        ents.sort_by_key(|e| {
            (
                !e.file_type().map(|t| t.is_dir()).unwrap_or(false),
                e.file_name().to_string_lossy().to_lowercase(),
            )
        });
        for e in ents {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let label = if is_dir {
                format!("{name}/")
            } else {
                name.clone()
            };
            let desc = if is_dir {
                "directory".to_string()
            } else {
                e.metadata()
                    .map(|m| {
                        let len = m.len();
                        if len < 1024 {
                            format!("{len} B")
                        } else {
                            format!("{:.1} KB", len as f64 / 1024.0)
                        }
                    })
                    .unwrap_or_else(|_| "?".into())
            };
            items.push(SelectItem::new(name, label).with_description(&desc));
        }
    }
    SelectList::new(
        items,
        12,
        sl_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(20),
            max_primary_column_width: Some(50),
            truncate_primary: None,
        },
    )
}

struct ShowAllApp {
    view: View,
    nav: SelectList,
    header: Panel,
    status: String,
    editor: Editor,
    file_browser: SelectList,
    file_browser_dir: PathBuf,
    file_browser_dirty: bool,
    settings: SettingsList,
    loader: Loader,
    quit_flag: Arc<AtomicBool>,
    submitted: Rc<RefCell<String>>,
}

impl ShowAllApp {
    fn new(quit_flag: Arc<AtomicBool>) -> Self {
        let mut header = Panel::new(2, 1, Some(Box::new(white_on_blue)));
        header.add_child(Box::new(Text::new("xylitol-tui".into(), 0, 0)));
        header.add_child(Box::new(Text::new(
            "differential-rendering terminal UI toolkit".into(),
            0,
            0,
        )));

        let nav = SelectList::new(
            vec![
                SelectItem::new("home", "🏠  Home").with_description("Overview & feature matrix"),
                SelectItem::new("edit", "✏️  Editor")
                    .with_description("Multi-line editor with autocomplete"),
                SelectItem::new("files", "📁  Files").with_description("File browser"),
                SelectItem::new("settings", "⚙️  Settings")
                    .with_description("Configurable settings list"),
                SelectItem::new("help", "❓  Help").with_description("Key bindings reference"),
            ],
            5,
            sl_theme(),
            SelectListLayoutOptions {
                min_primary_column_width: Some(16),
                max_primary_column_width: Some(30),
                truncate_primary: None,
            },
        );

        let clock = Box::new(SystemClock);
        let mut editor = Editor::new(
            EditorTheme {
                border_color: Box::new(cyan),
                select_list_theme: SelectListTheme::default(),
            },
            EditorOptions {
                padding_x: 0,
                terminal_rows: 40,
            },
            clock,
        );
        let submitted = Rc::new(RefCell::new(String::new()));
        let sub_clone = submitted.clone();
        editor.on_submit = Some(Box::new(move |t| {
            *sub_clone.borrow_mut() = t;
        }));
        editor.set_text(
            "// Type here! Tab for autocomplete, Ctrl+u to delete line, Ctrl+y to yank.\n// Up/Down for history, Ctrl+f then a char to jump."
                .to_string(),
        );
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let provider = CombinedAutocompleteProvider::new(vec![], cwd);
        editor.set_autocomplete_provider(Some(provider));

        let settings = SettingsList::new(
            vec![
                SettingItem {
                    id: "theme".into(),
                    label: "Theme".into(),
                    description: Some("dark / light".into()),
                    current_value: "dark".into(),
                    values: Some(vec!["dark".into(), "light".into()]),
                    submenu: None,
                },
                SettingItem {
                    id: "autocomplete".into(),
                    label: "Autocomplete".into(),
                    description: Some("on / off".into()),
                    current_value: "on".into(),
                    values: Some(vec!["on".into(), "off".into()]),
                    submenu: None,
                },
                SettingItem {
                    id: "history".into(),
                    label: "Max history".into(),
                    description: Some("10-1000".into()),
                    current_value: "100".into(),
                    values: Some(vec!["10".into(), "100".into(), "500".into()]),
                    submenu: None,
                },
            ],
            5,
            SettingsListTheme {
                label: Box::new(|s, _| s.to_string()),
                value: Box::new(|s, _| cyan(s)),
                description: Box::new(dim),
                cursor: ">".into(),
                hint: Box::new(dim),
            },
            |id: &str, val: &str| {
                eprintln!("[settings] {id} = {val}");
            },
            || {},
            SettingsListOptions {
                enable_search: false,
            },
        );

        let loader = Loader::new(
            Box::new(cyan),
            Box::new(dim),
            "...".into(),
            Some(LoaderIndicatorOptions {
                frames: vec![
                    "⠋".into(),
                    "⠙".into(),
                    "⠹".into(),
                    "⠸".into(),
                    "⠼".into(),
                    "⠴".into(),
                    "⠦".into(),
                    "⠧".into(),
                    "⠇".into(),
                    "⠏".into(),
                ],
                interval_ms: 80,
            }),
        );

        let fb_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let fb = build_file_browser(&fb_dir);

        ShowAllApp {
            view: View::Home,
            nav,
            header,
            status: "Home — press Enter on a view to explore".into(),
            editor,
            file_browser: fb,
            file_browser_dir: fb_dir,
            file_browser_dirty: false,
            settings,
            loader,
            quit_flag,
            submitted,
        }
    }
}

impl Component for ShowAllApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        if self.file_browser_dirty {
            self.file_browser = build_file_browser(&self.file_browser_dir.clone());
            self.file_browser_dirty = false;
        }

        let cw = width.saturating_sub(4);
        let mut out: Vec<String> = Vec::new();
        out.extend(self.header.render(width));

        let status_line = format!(
            " {} | q=quit arrows=navigate Enter=select | {} ",
            dim(&self.status),
            match self.view {
                View::Home => "🏠 Home",
                View::Edit => "✏️ Editor",
                View::Files => "📁 Files",
                View::Settings => "⚙️ Settings",
                View::Help => "❓ Help",
            }
        );
        let pad = width.saturating_sub(visible_width(&status_line));
        out.push(format!("{status_line}{}", " ".repeat(pad)));
        out.push(dim(&"─".repeat(width)));

        let sidebar_w = 32;
        let nav_items = self.nav.render(30);

        let mk = |text: &str| {
            Markdown::new(
                text.into(),
                0,
                0,
                md_theme(),
                None,
                Some(MarkdownOptions::default()),
            )
        };

        let content: Vec<String> = match self.view {
            View::Home => mk(HOME_MD).render(cw.saturating_sub(sidebar_w)),
            View::Help => mk(HELP_MD).render(cw.saturating_sub(sidebar_w)),
            View::Edit => self.editor.render(cw),
            View::Files => {
                let fb = self.file_browser.render(cw.saturating_sub(sidebar_w));
                let mut lines = vec![
                    format!(" 📂 {}", dim(&self.file_browser_dir.display().to_string())),
                    String::new(),
                ];
                lines.extend(fb);
                lines
            }
            View::Settings => self.settings.render(cw.saturating_sub(sidebar_w)),
        };

        let max_h = nav_items.len().max(content.len());
        for i in 0..max_h {
            let n = nav_items.get(i).map(|s| s.as_str()).unwrap_or("");
            let c = content.get(i).map(|s| s.as_str()).unwrap_or("");
            let npad = sidebar_w.saturating_sub(visible_width(n));
            out.push(format!("  {n}{}{c}", " ".repeat(npad.saturating_add(2))));
        }

        out.push(dim(&"─".repeat(width)));
        let spinner = self.loader.render(20).first().cloned().unwrap_or_default();
        out.push(format!(
            "  Loader: {spinner} | submitted: {} | q=quit Esc=back",
            dim(&self.submitted.borrow())
        ));

        out
    }

    fn handle_input(&mut self, data: &str) {
        if data == "q" || data == "\x03" {
            self.quit_flag.store(true, Ordering::SeqCst);
            return;
        }

        match self.view {
            View::Edit => {
                if data == "\x1b" {
                    self.view = View::Home;
                    return;
                }
                self.editor.handle_input(data);
            }
            View::Files => {
                if data == "\x1b" {
                    self.view = View::Home;
                    return;
                }
                self.file_browser.handle_input(data);
                if (data == "\n" || data == "\r")
                    && let Some(item) = self.file_browser.get_selected_item()
                {
                    if item.value == ".." {
                        if let Some(parent) = self.file_browser_dir.parent() {
                            self.file_browser_dir = parent.to_path_buf();
                            self.file_browser_dirty = true;
                        }
                    } else if item.label.ends_with('/') {
                        self.file_browser_dir = self.file_browser_dir.join(&item.value);
                        self.file_browser_dirty = true;
                    }
                }
            }
            View::Settings => {
                if data == "\x1b" {
                    self.view = View::Home;
                    return;
                }
                self.settings.handle_input(data);
            }
            View::Help | View::Home => {
                self.nav.handle_input(data);
                if (data == "\n" || data == "\r")
                    && let Some(item) = self.nav.get_selected_item()
                {
                    match item.value.as_str() {
                        "home" => {
                            self.view = View::Home;
                            self.status = "Home".into();
                        }
                        "edit" => {
                            self.view = View::Edit;
                            self.status = "Editor — Tab=autocomplete, Up/Down=history".into();
                        }
                        "files" => {
                            self.view = View::Files;
                            self.file_browser_dirty = true;
                            self.status = "File browser — Enter=expand, Esc=back".into();
                        }
                        "settings" => {
                            self.view = View::Settings;
                            self.status = "Settings — arrows=select".into();
                        }
                        "help" => {
                            self.view = View::Help;
                            self.status = "Help — key bindings".into();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn invalidate(&mut self) {}
}

impl Focusable for ShowAllApp {
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool {
        true
    }
}
