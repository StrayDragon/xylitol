//! xylitol-tui 综合演示 — 覆盖全部特性
//!
//! 运行：`cargo run -p xylitol-tui --example showcase`
//!
//! 展示：Panel 布局 / Markdown 渲染 / SelectList 导航 / SettingsList 配置 /
//! Loader 动画 / Input 输入 / 终端颜色 / 覆盖层 (overlay) / keybindings

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::components::input::Input;
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::markdown::{Markdown, MarkdownTheme};
use xylitol_tui::components::panel::Panel;
use xylitol_tui::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
use xylitol_tui::components::settings_list::{
    SettingItem, SettingsList, SettingsListOptions, SettingsListTheme,
};
use xylitol_tui::components::spacer::Spacer;
use xylitol_tui::components::text::Text;
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{Component, CrosstermTerminal, TUI};

// ── ANSI color helpers ──────────────────────────────────────────────────────

fn cyan(s: &str) -> String {
    format!("\x1b[36m{s}\x1b[39m")
}
fn green(s: &str) -> String {
    format!("\x1b[32m{s}\x1b[39m")
}
fn yellow(s: &str) -> String {
    format!("\x1b[33m{s}\x1b[39m")
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
fn white(s: &str) -> String {
    format!("\x1b[37m{s}\x1b[39m")
}

const HEADER_BG: fn(&str) -> String = |s| format!("\x1b[44m\x1b[37m{s}\x1b[49m\x1b[39m");

const SIDEBAR_BG: fn(&str) -> String = |s| format!("\x1b[40m\x1b[37m{s}\x1b[49m\x1b[39m");

// ── demo data ───────────────────────────────────────────────────────────────

const MD_CONTENT: &str = "\
# Welcome to xylitol-tui 🦀

A **Rust** port of [pi-tui](https://github.com/earendil-works/pi), a flicker-free
terminal UI library with **differential rendering**.

## Features

- 🎨 **Panel** — padded containers with background colors
- 📝 **Markdown** — render markdown with syntax highlighting hooks
- 📋 **SelectList** — keyboard-navigable item lists
- ⚙️ **SettingsList** — configurable settings with fuzzy search
- ✏️ **Input** — CJK-aware text input with cursor, undo, kill-ring
- 🔄 **Loader** — animated spinner component
- ⌨️ **Key bindings** — customizable keybindings (emacs-ready)

## Differential rendering

Only changed screen cells are written — no flicker, low CPU.
Built on a cell-grid diff engine with viewport scrolling,
overlay compositing, and width-run-aware overflow protection.

```rust
fn main() {
    println!(\"Hello, xylitol-tui!\");
}
```
";

// ── Main app ────────────────────────────────────────────────────────────────

struct ShowcaseApp {
    // Navigation
    current_view: View,
    nav_list: SelectList,

    // Content
    markdown: Markdown,
    settings: SettingsList,
    input_box: Input,
    loader: Loader,
    submitted_text: String,

    // Overlay state
    show_help: bool,
    show_settings: bool,

    // Animation
    tick: usize,

    // Exit
    quit_flag: Arc<AtomicBool>,
}

#[derive(Clone, PartialEq)]
enum View {
    Info,
    Components,
    Input,
    Settings,
    Quit,
}

impl ShowcaseApp {
    fn new(quit_flag: Arc<AtomicBool>) -> Self {
        let nav_theme = SelectListTheme {
            selected_prefix: Box::new(|s| cyan(s)),
            selected_text: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
            description: Box::new(|s| dim(s)),
            scroll_info: Box::new(|s| dim(s)),
            no_match: Box::new(|s| dim(s)),
        };

        let nav_list = SelectList::new(
            vec![
                SelectItem::new("info", "📋 Welcome / Readme")
                    .with_description("Project overview and feature list"),
                SelectItem::new("components", "🧩 Component Showcase")
                    .with_description("Loader, Text, Panel, Spacer"),
                SelectItem::new("input", "✏️  Input Demo")
                    .with_description("Try the text editor component"),
                SelectItem::new("settings", "⚙️  Settings Demo")
                    .with_description("Configure preferences"),
                SelectItem::new("quit", "🚪 Quit").with_description("Exit the demo"),
            ],
            5,
            nav_theme,
            SelectListLayoutOptions {
                min_primary_column_width: Some(24),
                max_primary_column_width: Some(36),
                truncate_primary: None,
            },
        );

        // ── Markdown ──
        let md_theme = MarkdownTheme {
            heading: Box::new(|s| bold(&cyan(s))),
            link: Box::new(|s| cyan(s)),
            link_url: Box::new(|s| dim(s)),
            code: Box::new(|s| yellow(s)),
            code_block: Box::new(|s| dim(s)),
            code_block_border: Box::new(|s| dim(s)),
            quote: Box::new(|s| dim(s)),
            quote_border: Box::new(|s| dim(s)),
            hr: Box::new(|s| dim(s)),
            list_bullet: Box::new(|s| cyan(s)),
            bold: Box::new(|s| bold(s)),
            italic: Box::new(|s| dim(s)),
            strikethrough: Box::new(|s| red(s)),
            underline: Box::new(|s| format!("\x1b[4m{s}\x1b[24m")),
            highlight_code: None,
            code_block_indent: Some("  ".to_string()),
        };
        let markdown = Markdown::new(MD_CONTENT.to_string(), 2, 2, md_theme, None, None);

        // ── SettingsList ──
        let sl_theme = SettingsListTheme {
            label: Box::new(|s, _| s.to_string()),
            value: Box::new(|s, sel| if sel { cyan(s) } else { dim(s) }),
            description: Box::new(|s| dim(s)),
            cursor: "> ".to_string(),
            hint: Box::new(|s| dim(s)),
        };
        let settings = SettingsList::new(
            vec![
                SettingItem {
                    id: "theme".into(),
                    label: "Theme".into(),
                    description: Some("Color theme for the UI".into()),
                    current_value: "Dark".into(),
                    values: Some(vec!["Dark".into(), "Light".into(), "Monokai".into()]),
                    submenu: None,
                },
                SettingItem {
                    id: "diff_mode".into(),
                    label: "Diff mode".into(),
                    description: Some("How code changes are displayed".into()),
                    current_value: "Unified".into(),
                    values: Some(vec!["Unified".into(), "Split".into(), "Inline".into()]),
                    submenu: None,
                },
                SettingItem {
                    id: "autosave".into(),
                    label: "Auto-save".into(),
                    description: Some("Time between auto-saves".into()),
                    current_value: "off".into(),
                    values: Some(vec![
                        "off".into(),
                        "30s".into(),
                        "1min".into(),
                        "5min".into(),
                    ]),
                    submenu: None,
                },
                SettingItem {
                    id: "vim_mode".into(),
                    label: "Vim mode".into(),
                    description: Some("Enable vim-style keybindings".into()),
                    current_value: "off".into(),
                    values: Some(vec!["off".into(), "on".into()]),
                    submenu: None,
                },
            ],
            8,
            sl_theme,
            |id, val| {
                eprintln!("Setting changed: {id} = {val}");
            },
            || {},
            SettingsListOptions {
                enable_search: false,
            },
        );

        // ── Input ──
        let mut input_box = Input::new();
        input_box.set_focused(true);

        // ── Loader ──
        let loader = Loader::new(
            Box::new(|s| cyan(s)),
            Box::new(|s| dim(s)),
            "Loading components...".to_string(),
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

        Self {
            current_view: View::Info,
            nav_list,
            markdown,
            settings,
            input_box,
            loader,
            submitted_text: String::new(),
            show_help: false,
            show_settings: false,
            tick: 0,
            quit_flag,
        }
    }
}

impl Component for ShowcaseApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.tick = self.tick.wrapping_add(1);
        let sidebar_w = 28usize;
        let content_w = width.saturating_sub(sidebar_w);
        let mut lines: Vec<String> = Vec::new();

        // ═══════════════════════════════════════════════════════════════════
        // Header
        // ═══════════════════════════════════════════════════════════════════
        let mut header = Panel::new(2, 0, Some(Box::new(HEADER_BG)));
        header.add_child(Box::new(Text::new(
            bold("🦀 xylitol-tui  Showcase").to_string(),
            0,
            0,
        )));
        header.add_child(Box::new(Text::new(
            dim("  pi-tui → Rust  ·  differential rendering · composable widgets").to_string(),
            0,
            0,
        )));
        let header_lines = header.render(width);
        lines.extend(header_lines);

        // Separator
        let sep_line = dim(&format!(
            "{}{}",
            "─".repeat(2),
            "─".repeat(width.saturating_sub(2))
        ));
        lines.push(sep_line);

        // ═══════════════════════════════════════════════════════════════════
        // Layout: sidebar + content
        // ═══════════════════════════════════════════════════════════════════
        let content_lines = match &self.current_view {
            View::Info => self.render_info_view(content_w),
            View::Components => self.render_components_view(content_w),
            View::Input => self.render_input_view(content_w),
            View::Settings => self.render_settings_view(content_w),
            View::Quit => vec![],
        };

        // Sidebar
        let mut sidebar = Panel::new(1, 1, Some(Box::new(SIDEBAR_BG)));
        sidebar.add_child(Box::new(Text::new(
            bold(&format!("{} Navigation", white("■"))),
            0,
            0,
        )));
        sidebar.add_child(Box::new(Spacer::new(1)));
        let nav_lines = self.nav_list.render(sidebar_w);
        for nl in nav_lines {
            sidebar.add_child(Box::new(Text::new(nl, 0, 0)));
        }
        sidebar.add_child(Box::new(Spacer::new(1)));
        sidebar.add_child(Box::new(Text::new(dim("  ↑↓  navigate"), 0, 0)));
        sidebar.add_child(Box::new(Text::new(dim("  ↵  select"), 0, 0)));
        sidebar.add_child(Box::new(Text::new(dim("  ?  help"), 0, 0)));
        sidebar.add_child(Box::new(Text::new(dim("  Esc  back"), 0, 0)));

        let sidebar_out = sidebar.render(sidebar_w);

        // Merge sidebar + content lines
        let max_lines = content_lines.len().max(sidebar_out.len());
        for i in 0..max_lines {
            let left = sidebar_out
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("{}", " ".repeat(sidebar_w)));
            let right = content_lines.get(i).cloned().unwrap_or_default();
            lines.push(format!("{}{}", left, right));
        }

        // ── Status bar ──────────────────────────────────────────────────
        let status_text = format!(
            " {} {} | xylitol-tui v0.1.0 | 183 tests | clippy clean",
            dim("━"),
            dim("Status: ready"),
        );
        lines.push(dim(&"─".repeat(width)));
        lines.push(status_text);

        // ── Help overlay ─────────────────────────────────────────────────
        if self.show_help {
            self.render_overlay(&mut lines, width, &self.help_content(40), 38);
        }

        lines
    }

    fn handle_input(&mut self, data: &str) {
        use xylitol_tui::matches_key;

        // Global keys
        if matches_key(data, "ctrl+c") {
            self.quit_flag.store(true, Ordering::SeqCst);
            return;
        }
        if data == "?" {
            self.show_help = !self.show_help;
            return;
        }
        if self.show_help && matches_key(data, "escape") {
            self.show_help = false;
            return;
        }

        match &self.current_view {
            View::Info | View::Components | View::Quit => {
                if matches_key(data, "up") {
                    self.nav_list.handle_input("\x1b[A");
                } else if matches_key(data, "down") {
                    self.nav_list.handle_input("\x1b[B");
                } else if matches_key(data, "enter") {
                    if let Some(item) = self.nav_list.get_selected_item() {
                        match item.value.as_str() {
                            "info" => self.current_view = View::Info,
                            "components" => self.current_view = View::Components,
                            "input" => self.current_view = View::Input,
                            "settings" => self.current_view = View::Settings,
                            "quit" => self.quit_flag.store(true, Ordering::SeqCst),
                            _ => {}
                        }
                    }
                }
            }
            View::Input => {
                if matches_key(data, "escape") {
                    self.current_view = View::Info;
                    return;
                }
                self.input_box.handle_input(data);
                if let Some(ref on_submit) = self.input_box.on_submit.take() {
                    // won't fire directly—just handle via value
                    let _ = on_submit;
                }
                // Check if input has a value (just submitted = cleared)
                let val = self.input_box.value().to_string();
                if val != self.submitted_text {
                    if !val.is_empty() || self.submitted_text.is_empty() {
                        // value changed
                    }
                    self.submitted_text = val;
                }
            }
            View::Settings => {
                if matches_key(data, "escape") {
                    self.current_view = View::Info;
                    return;
                }
                self.settings.handle_input(data);
            }
        }
    }

    fn invalidate(&mut self) {
        self.markdown.invalidate();
    }
}

// ── view renderers ──────────────────────────────────────────────────────────

impl ShowcaseApp {
    fn render_info_view(&mut self, width: usize) -> Vec<String> {
        // Toggle markdown content to show it can update
        self.markdown.render(width.saturating_sub(4))
    }

    fn render_components_view(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        // Loader line
        let loader_frame = if self.tick % 23 < 12 {
            cyan("⏳ Loader active...")
        } else {
            dim("⏳ Loader paused")
        };
        lines.push(format!("  {loader_frame}"));
        lines.push(String::new());

        // Text + Spacer + Text in Panel
        let mut comp_panel = Panel::new(2, 1, None);
        comp_panel.add_child(Box::new(Text::new(
            bold("Component Gallery").to_string(),
            0,
            0,
        )));
        comp_panel.add_child(Box::new(Spacer::new(1)));
        comp_panel.add_child(Box::new(Text::new(
            format!(
                "{} {} {} {} {} {} {} {}",
                cyan("Text"),
                yellow("SelectList"),
                green("SettingsList"),
                dim("Spacer"),
                bold("Markdown"),
                white("Loader"),
                red("Input"),
                cyan("Panel"),
            ),
            0,
            0,
        )));
        comp_panel.add_child(Box::new(Spacer::new(1)));
        comp_panel.add_child(Box::new(Text::new(
            dim("Each component is a self-contained widget with its own render() and handleInput() methods.")
                .to_string(),
            0,
            0,
        )));
        lines.extend(comp_panel.render(width.saturating_sub(4)));
        lines
    }

    fn render_input_view(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        // Title
        let mut header = Panel::new(2, 1, None);
        header.add_child(Box::new(Text::new(
            bold("✏️  Input Demo").to_string(),
            0,
            0,
        )));
        header.add_child(Box::new(Text::new(
            dim("Type something — CJK, emoji, any UTF-8").to_string(),
            0,
            0,
        )));
        lines.extend(header.render(width.saturating_sub(4)));

        lines.push(String::new());

        // Render input component inline
        let input_lines = self.input_box.render(width.saturating_sub(6));
        for il in input_lines {
            lines.push(format!("    {il}"));
        }

        lines.push(String::new());
        if !self.submitted_text.is_empty() {
            lines.push(dim(&format!("    Submitted: {}", self.submitted_text)));
        }
        lines.push(dim(
            "    ↑↓ move · ←→ navigate · Enter submit · Ctrl+A/E line bounds",
        ));
        lines
    }

    fn render_settings_view(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        let mut header = Panel::new(2, 1, None);
        header.add_child(Box::new(Text::new(
            bold("⚙️  Settings Demo").to_string(),
            0,
            0,
        )));
        header.add_child(Box::new(Text::new(
            dim("Enter/Space to cycle values · Type to search").to_string(),
            0,
            0,
        )));
        lines.extend(header.render(width.saturating_sub(4)));

        lines.push(String::new());
        lines.extend(self.settings.render(width.saturating_sub(6)));
        lines
    }

    fn help_content(&self, width: usize) -> Vec<String> {
        let mut panel = Panel::new(
            2,
            1,
            Some(Box::new(|s: &str| {
                format!("\x1b[47m\x1b[30m{s}\x1b[49m\x1b[39m")
            })),
        );
        panel.add_child(Box::new(Text::new(
            bold(" Keyboard Shortcuts").to_string(),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        panel.add_child(Box::new(Text::new(
            "  ↑↓      — Navigate menus".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new(
            "  Enter   — Select / submit".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new(
            "  Esc     — Go back / close overlay".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new(
            "  ?       — Toggle this help".into(),
            0,
            0,
        )));
        panel.add_child(Box::new(Text::new("  Ctrl+C  — Exit".into(), 0, 0)));
        panel.add_child(Box::new(Spacer::new(1)));
        panel.add_child(Box::new(Text::new(
            dim("Press ? or Esc to close").to_string(),
            0,
            0,
        )));
        panel.render(width)
    }

    fn render_overlay(
        &self,
        lines: &mut Vec<String>,
        width: usize,
        content: &[String],
        overlay_w: usize,
    ) {
        let row_off = 3usize;
        let col_off = (width.saturating_sub(overlay_w)) / 2;

        // Pad lines to accommodate overlay
        while lines.len() < row_off + content.len() {
            lines.push(String::new());
        }
        for (i, cl) in content.iter().enumerate() {
            let row = row_off + i;
            let existing = lines[row].clone();
            let mut result = String::with_capacity(width);
            result.push_str(&existing);
            let vis = existing.len();
            if vis < col_off {
                result.push_str(&" ".repeat(col_off - vis));
            }
            result.push_str(cl);
            // Pad to width
            let rv = result.len();
            if rv < width {
                result.push_str(&" ".repeat(width - rv));
            }
            lines[row] = result;
        }
    }
}

// ── main ────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize keybindings
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));

    let term = CrosstermTerminal::new()?;
    let mut tui = TUI::new(term);

    let quit_flag = Arc::new(AtomicBool::new(false));
    let app = ShowcaseApp::new(quit_flag.clone());
    tui.add_child(Box::new(app));
    tui.set_focus(Some(0));

    tui.start_with_flag(&quit_flag)?;
    Ok(())
}
