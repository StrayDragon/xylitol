use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use xylitol_tui::components::{
    panel::Panel,
    select_list::{SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme},
    spacer::Spacer,
    text::Text,
};
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{Component, CrosstermTerminal, TUI};

fn cyan(s: &str) -> String {
    format!("\x1b[36m{}\x1b[39m", s)
}
fn green(s: &str) -> String {
    format!("\x1b[32m{}\x1b[39m", s)
}
fn yellow(s: &str) -> String {
    format!("\x1b[33m{}\x1b[39m", s)
}
fn dim(s: &str) -> String {
    format!("\x1b[2m{}\x1b[22m", s)
}
fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[22m", s)
}
fn blue_bg(s: &str) -> String {
    format!("\x1b[44m\x1b[37m{}\x1b[49m\x1b[39m", s)
}
fn selected_text(s: &str) -> String {
    format!("\x1b[7m{}\x1b[27m", s)
}

const CONVERSATION: &[(&str, &str)] = &[
    (
        "user",
        "I need to build a command-line tool that searches files recursively for a pattern and shows matches with context.",
    ),
    (
        "assistant",
        "I'll help you build that! Let me create a `search-tool` command. Here's the plan:\n\n  • Use `walkdir` for recursive file traversal\n  • Use regex for pattern matching\n  • Include colored output with line numbers and context\n  • Add flags for case-insensitive, file-type filtering, and max depth\n\nLet me scaffold the project first...",
    ),
    (
        "system",
        "Ran: cargo init search-tool\nCreated Cargo.toml with deps: walkdir, regex, clap, colored",
    ),
    (
        "assistant",
        "Perfect! Now the implementation. Here's the core loop:",
    ),
    (
        "system",
        r##"Wrote src/main.rs:
```rust
use clap::Parser;
use colored::*;
use regex::Regex;
use walkdir::WalkDir;

#[derive(Parser)]
struct Args {
    pattern: String,
    #[arg(short, long)]
    dir: Option<String>,
    #[arg(short = 'C', long, default_value = "2")]
    context: usize,
    #[arg(short = 'i', long)]
    case_insensitive: bool,
    #[arg(short = 't', long)]
    file_type: Option<String>,
}
```"##,
    ),
    (
        "assistant",
        "I've implemented the search logic. The tool handles:\n\n  • Pattern compilation with `RegexBuilder`\n  • Context lines before/after matches\n  • Binary file detection and skipping\n  • Colorized output: file paths in cyan, line numbers in yellow, matches in green\n  • Progress indicator for large directories\n\nWould you like me to add any features?",
    ),
    (
        "user",
        "Yes, add streaming and make it multi-threaded with rayon.",
    ),
    (
        "system",
        r##"Updated Cargo.toml: added rayon = "1.10"

Rewrote src/main.rs to use:
  • rayon::scope for parallel directory traversal
  • Crossbeam channels for streaming results
  • Print matches as they're found
  • Atomic counter for progress tracking

Performance: 12x faster on 100k files, results appear immediately"##,
    ),
    (
        "assistant",
        "Done! The streaming version with rayon is ready. Here's a summary of what was built:\n\n  • 287 lines of Rust\n  • 3 dependencies (walkdir, regex, rayon)\n  • CLI with 5 flags via clap\n  • Handles 100k+ files with minimal memory\n  • Results stream in as they're found\n\nTry it: `cargo run -- \"TODO\" --dir ./src -i`",
    ),
    (
        "user",
        "It works great! Can you add the output to a file and show match statistics at the end?",
    ),
    (
        "assistant",
        "Adding file output and statistics. One moment...",
    ),
    (
        "system",
        r##"Modified src/main.rs:
  • Added --output flag for writing results to file
  • Added match statistics section at end of run
  • Stats: total files scanned, files matched, lines matched, match rate
  • Output format: file paths, line numbers, context, match counts"##,
    ),
    (
        "assistant",
        "All done. The final tool includes:\n\n  Stats output:\n  ┌─ Search Complete ─────────────┐\n  │ Files scanned:    1,247      │\n  │ Files matched:    42         │\n  │ Lines matched:    178        │\n  │ Match rate:       3.4%       │\n  │ Duration:         1.2s       │\n  └──────────────────────────────┘\n\nEverything is ready. Is there anything else you'd like?",
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));

    let term = CrosstermTerminal::new()?;
    let mut tui = TUI::new(term);

    let quit_flag = Arc::new(AtomicBool::new(false));
    let app = ChatApp::new(quit_flag.clone());
    tui.add_child(Box::new(app));
    tui.set_focus(Some(0));

    // Run event loop; check quit_flag in addition to stopped
    tui.start_with_flag(&quit_flag)
}

struct ChatApp {
    #[allow(dead_code)]
    scroll_offset: usize,
    sent_messages: Vec<String>,
    preset_turn: usize,
    preset_step: usize,
    show_help: bool,
    show_settings: bool,
    settings_menu: SelectList,
    quit_flag: Arc<AtomicBool>,
}

impl ChatApp {
    fn new(quit_flag: Arc<AtomicBool>) -> Self {
        let settings_theme = SelectListTheme {
            selected_text: Box::new(selected_text),
            selected_prefix: Box::new(cyan),
            description: Box::new(dim),
            scroll_info: Box::new(dim),
            no_match: Box::new(dim),
        };
        let settings_menu = SelectList::new(
            vec![
                SelectItem::new("model", "Model: claude-sonnet-4")
                    .with_description("Current AI model"),
                SelectItem::new("context", "Context length: 200K")
                    .with_description("Maximum token context"),
                SelectItem::new("theme", "Theme: dark").with_description("Color scheme"),
                SelectItem::new("diff", "Diff mode: unified")
                    .with_description("Code display format"),
                SelectItem::new("autosave", "Auto-save: 5min")
                    .with_description("File autosave interval"),
            ],
            5,
            settings_theme,
            SelectListLayoutOptions {
                min_primary_column_width: Some(24),
                max_primary_column_width: Some(40),
                truncate_primary: None,
            },
        );

        Self {
            scroll_offset: 0,
            sent_messages: Vec::new(),
            preset_turn: 0,
            preset_step: 0,
            show_help: false,
            show_settings: false,
            settings_menu,
            quit_flag,
        }
    }

    fn advance_preset(&mut self) -> bool {
        if self.preset_turn >= CONVERSATION.len() {
            return false;
        }
        let (role, content) = CONVERSATION[self.preset_turn];
        let marker = role_marker(role);
        self.sent_messages.push(format!("{} {}", marker, content));
        self.preset_turn += 1;
        true
    }
}

fn role_marker(role: &str) -> String {
    match role {
        "user" => bold(&yellow("You")),
        "assistant" => bold(&green("AI")),
        "system" => dim("sys"),
        _ => role.to_string(),
    }
}

impl Component for ChatApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        // Header
        let mut header = Panel::new(2, 0, Some(Box::new(blue_bg)));
        header.add_child(Box::new(Text::new(
            format!("{} pi-tui agent demo (preset conversation)", bold("⚡")),
            0,
            0,
        )));
        lines.extend(header.render(width));

        // Separator
        let sep = dim(&format!(
            "{}{}",
            "─".repeat(2),
            "─".repeat(width.saturating_sub(2))
        ));
        let mut sep_text = Text::new(sep, 0, 0);
        lines.extend(sep_text.render(width));

        // Conversation
        let total = self.sent_messages.len();
        let visible = 12usize;
        let start = total.saturating_sub(visible);

        for i in start..total {
            let mut msg_text = Text::new(format!("  {}", self.sent_messages[i]), 1, 0);
            lines.extend(msg_text.render(width));
            let mut gap = Spacer::new(1);
            lines.extend(gap.render(width));
        }

        // Typing indicator
        if self.preset_turn < CONVERSATION.len() {
            let indicator = if self.preset_step % 8 < 4 { "▌" } else { " " };
            let hint = dim(&format!(
                "  {} Press Enter to continue conversation...",
                indicator
            ));
            let mut hint_text = Text::new(hint, 1, 0);
            lines.extend(hint_text.render(width));
        } else {
            let hint = dim("  Conversation complete. Press Ctrl+C to exit.");
            let mut hint_text = Text::new(hint, 1, 0);
            lines.extend(hint_text.render(width));
        }

        // Help overlay
        if self.show_help {
            let mut help = Panel::new(
                2,
                1,
                Some(Box::new(|s: &str| {
                    format!("\x1b[47m\x1b[30m{}\x1b[49m\x1b[39m", s)
                })),
            );
            help.add_child(Box::new(Text::new(
                bold(" Keyboard Shortcuts").to_string(),
                0,
                0,
            )));
            help.add_child(Box::new(Spacer::new(1)));
            help.add_child(Box::new(Text::new(
                "  Enter    — Continue conversation".into(),
                0,
                0,
            )));
            help.add_child(Box::new(Text::new(
                "  ?        — Toggle this help".into(),
                0,
                0,
            )));
            help.add_child(Box::new(Text::new(
                "  s        — Open settings".into(),
                0,
                0,
            )));
            help.add_child(Box::new(Text::new("  Esc/Ctrl+C — Exit".into(), 0, 0)));
            help.add_child(Box::new(Spacer::new(1)));
            help.add_child(Box::new(Text::new(
                dim("  Press ? to close").to_string(),
                0,
                0,
            )));

            let help_lines = help.render(36);
            let row_off = 2usize;
            let col_off = width.saturating_sub(38) / 2;

            while lines.len() < row_off + help_lines.len() {
                lines.push(String::new());
            }
            for (i, hl) in help_lines.iter().enumerate() {
                let row = row_off + i;
                let existing = lines[row].clone();
                let pad = if existing.len() < col_off {
                    " ".repeat(col_off - existing.len())
                } else {
                    String::new()
                };
                lines[row] = format!("{}{}{}", existing, pad, hl);
            }
        }

        // Settings overlay
        if self.show_settings {
            let mut panel = Panel::new(
                2,
                1,
                Some(Box::new(|s: &str| {
                    format!("\x1b[47m\x1b[30m{}\x1b[49m\x1b[39m", s)
                })),
            );
            panel.add_child(Box::new(Text::new(bold(" Settings").to_string(), 0, 0)));
            panel.add_child(Box::new(Spacer::new(1)));
            let panel_lines = panel.render(40);
            let menu_lines = self.settings_menu.render(40);
            let all: Vec<String> = panel_lines
                .into_iter()
                .chain(menu_lines)
                .chain(vec![String::new(), dim("  ↑↓ select  Esc close")])
                .collect();

            let row_off = 2usize;
            let col_off = width.saturating_sub(42) / 2;
            while lines.len() < row_off + all.len() {
                lines.push(String::new());
            }
            for (i, sl) in all.iter().enumerate() {
                let row = row_off + i;
                let existing = lines[row].clone();
                let pad = if existing.len() < col_off {
                    " ".repeat(col_off - existing.len())
                } else {
                    String::new()
                };
                lines[row] = format!("{}{}{}", existing, pad, sl);
            }
        }

        self.preset_step = (self.preset_step + 1) % 8;
        lines
    }

    fn handle_input(&mut self, data: &str) {
        use xylitol_tui::matches_key;

        if matches_key(data, "ctrl+c") || matches_key(data, "escape") {
            if self.show_settings {
                self.show_settings = false;
            } else if self.show_help {
                self.show_help = false;
            } else {
                self.quit_flag.store(true, Ordering::SeqCst);
            }
            return;
        }
        if data == "?" {
            self.show_help = !self.show_help;
            self.show_settings = false;
            return;
        }
        if data == "s" && !self.show_settings {
            self.show_settings = !self.show_settings;
            self.show_help = false;
            return;
        }

        if self.show_settings {
            if matches_key(data, "up") {
                self.settings_menu.handle_input("\x1b[A");
            } else if matches_key(data, "down") {
                self.settings_menu.handle_input("\x1b[B");
            } else if matches_key(data, "enter") {
                self.show_settings = false;
            }
            return;
        }
        if self.show_help && (matches_key(data, "enter") || data == "?") {
            self.show_help = false;
            return;
        }

        if matches_key(data, "enter") && self.preset_turn < CONVERSATION.len() {
            self.advance_preset();
        }
    }

    fn invalidate(&mut self) {}
}
