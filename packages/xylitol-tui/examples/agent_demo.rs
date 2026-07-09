use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
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
use xylitol_tui::{
    Component, CrosstermTerminal, Focusable, SystemClock, TUI, matches_key, truncate_to_width,
    visible_width, wrap_text_with_ansi,
};

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
fn blue_bg(s: &str) -> String {
    format!("\x1b[44m\x1b[37m{s}\x1b[49m\x1b[39m")
}
fn selected_text(s: &str) -> String {
    format!("\x1b[7m{s}\x1b[27m")
}

#[allow(dead_code)]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));

    let term = CrosstermTerminal::new()?;
    let mut tui = TUI::new(term);
    let quit_flag = Arc::new(AtomicBool::new(false));

    tui.add_child(Box::new(FakeCodingAgentApp::new(quit_flag.clone())));
    tui.set_focus(Some(0));
    tui.start_with_flag(&quit_flag)
}

#[derive(Clone, Copy)]
enum Role {
    User,
    Assistant,
    Tool,
    System,
}

struct TranscriptEntry {
    role: Role,
    text: String,
}

enum ScriptEvent {
    Tool(String),
    Assistant(String),
    MarkPlan(usize),
    File(String),
    Status(String),
}

pub struct FakeCodingAgentApp {
    transcript: Vec<TranscriptEntry>,
    pending_events: VecDeque<ScriptEvent>,
    input: Editor,
    submit_slot: Rc<RefCell<Option<String>>>,
    palette_open: bool,
    palette: SelectList,
    settings_open: bool,
    settings: SettingsList,
    loader: Loader,
    plan: Vec<(bool, String)>,
    changed_files: Vec<String>,
    recent_tools: Vec<String>,
    footer_note: String,
    last_submitted: String,
    scripted_turn: usize,
    auto_started: bool,
    last_tick_at: Instant,
    quit_flag: Arc<AtomicBool>,
}

impl FakeCodingAgentApp {
    pub fn new(quit_flag: Arc<AtomicBool>) -> Self {
        Self::new_with_prompt(
            quit_flag,
            "tighten footer truncation and add a PTY acceptance test",
        )
    }

    pub fn new_with_prompt(quit_flag: Arc<AtomicBool>, initial_prompt: &str) -> Self {
        let submit_slot = Rc::new(RefCell::new(None));
        let submit_clone = submit_slot.clone();

        let mut input = Editor::new(
            EditorTheme {
                border_color: Box::new(cyan),
                select_list_theme: SelectListTheme::default(),
            },
            EditorOptions {
                padding_x: 1,
                terminal_rows: 8,
            },
            Box::new(SystemClock),
        );
        input.set_focused(true);
        input.on_submit = Some(Box::new(move |text| {
            *submit_clone.borrow_mut() = Some(text);
        }));
        input.set_text(initial_prompt.to_string());

        let palette = SelectList::new(
            vec![
                SelectItem::new("tests", "Run regression tests")
                    .with_description("Replay the current TUI acceptance suite"),
                SelectItem::new("diff", "Summarize staged diff")
                    .with_description("Explain the current code change set"),
                SelectItem::new("compact", "Compact conversation")
                    .with_description("Simulate a context compaction checkpoint"),
            ],
            5,
            SelectListTheme {
                selected_prefix: Box::new(cyan),
                selected_text: Box::new(selected_text),
                description: Box::new(dim),
                scroll_info: Box::new(dim),
                no_match: Box::new(red),
            },
            SelectListLayoutOptions {
                min_primary_column_width: Some(24),
                max_primary_column_width: Some(34),
                truncate_primary: None,
            },
        );

        let settings = SettingsList::new(
            vec![
                SettingItem {
                    id: "model".into(),
                    label: "Model".into(),
                    description: Some("execution model".into()),
                    current_value: "claude-sonnet-4".into(),
                    values: Some(vec![
                        "claude-sonnet-4".into(),
                        "gpt-5".into(),
                        "gpt-5-mini".into(),
                    ]),
                    submenu: None,
                },
                SettingItem {
                    id: "approval".into(),
                    label: "Approval".into(),
                    description: Some("tool execution policy".into()),
                    current_value: "never".into(),
                    values: Some(vec!["never".into(), "on-request".into()]),
                    submenu: None,
                },
                SettingItem {
                    id: "diff".into(),
                    label: "Diff mode".into(),
                    description: Some("review format".into()),
                    current_value: "unified".into(),
                    values: Some(vec!["unified".into(), "split".into()]),
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
            |_id: &str, _val: &str| {},
            || {},
            SettingsListOptions {
                enable_search: false,
            },
        );

        let loader = Loader::new(
            Box::new(cyan),
            Box::new(dim),
            "Ready".to_string(),
            Some(LoaderIndicatorOptions {
                frames: vec!["-".into(), "\\".into(), "|".into(), "/".into()],
                interval_ms: 80,
            }),
        );

        let mut app = Self {
            transcript: Vec::new(),
            pending_events: VecDeque::new(),
            input,
            submit_slot,
            palette_open: false,
            palette,
            settings_open: false,
            settings,
            loader,
            plan: vec![
                (true, "Read failing terminal report".into()),
                (false, "Reproduce in fake agent harness".into()),
                (false, "Fix width budgeting".into()),
                (false, "Run PTY smoke".into()),
            ],
            changed_files: vec!["packages/xylitol-tui/examples/agent_demo.rs".into()],
            recent_tools: vec!["read_file examples/agent_demo.rs".into()],
            footer_note: "Ctrl+P command palette  |  Ctrl+S settings  |  Ctrl+C quit".into(),
            last_submitted: String::new(),
            scripted_turn: 0,
            auto_started: false,
            last_tick_at: Instant::now(),
            quit_flag,
        };
        app.seed_transcript();
        app
    }

    fn seed_transcript(&mut self) {
        self.push_entry(
            Role::System,
            "Session restored in /home/l8ng/Projects/__straydragon__/xylitol on branch feat/tui-dev",
        );
        self.push_entry(
            Role::User,
            "Collapse examples into one fake coding-agent demo and keep foot interaction stable.",
        );
        self.push_entry(
            Role::Assistant,
            "Read the existing examples and the pi coding-agent flow first, then rebuild one stable primary scenario with real terminal acceptance coverage.",
        );
    }

    fn push_entry(&mut self, role: Role, text: impl Into<String>) {
        self.transcript.push(TranscriptEntry {
            role,
            text: text.into(),
        });
    }

    fn process_submit(&mut self, text: String) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            self.advance_script();
            return;
        }

        self.last_submitted = trimmed.clone();
        self.push_entry(Role::User, trimmed.clone());
        self.input.set_text(String::new());

        let mut events = VecDeque::new();
        events.push_back(ScriptEvent::Status("Running rg and cargo test".into()));
        events.push_back(ScriptEvent::Tool(format!(
            "rg -n \"{}\" packages/xylitol-tui tests",
            trimmed
        )));
        events.push_back(ScriptEvent::MarkPlan(1));
        events.push_back(ScriptEvent::File("tests/tui_e2e/pty.rs".to_string()));
        events.push_back(ScriptEvent::Tool(
            "cargo test -p xylitol-tui --test agent_demo_test".into(),
        ));
        events.push_back(ScriptEvent::MarkPlan(2));
        events.push_back(ScriptEvent::Assistant(format!(
            "Submitted your prompt as a task: `{}`. Next step: tighten footer truncation and fold the example flow into PTY smoke.",
            trimmed
        )));
        events.push_back(ScriptEvent::Status("Ready".into()));
        self.pending_events.extend(events);
        self.advance_script();
    }

    fn advance_script(&mut self) {
        if let Some(event) = self.pending_events.pop_front() {
            self.apply_event(event);
            return;
        }

        let fallback = match self.scripted_turn {
            0 => Some(vec![
                ScriptEvent::Status("Running acceptance harness".into()),
                ScriptEvent::Tool("cargo test -p xylitol-tui --test agent_demo_test".into()),
                ScriptEvent::MarkPlan(1),
                ScriptEvent::Assistant(
                    "agent_demo is now the single example surface; the old kitchen-sink demos are scheduled for removal.".into(),
                ),
                ScriptEvent::Status("Ready".into()),
            ]),
            1 => Some(vec![
                ScriptEvent::Tool("cargo test --test tui_e2e -- --ignored".into()),
                ScriptEvent::MarkPlan(3),
                ScriptEvent::Assistant(
                    "Real terminal smoke should stay focused on the primary flow rather than keeping every showcase alive.".into(),
                ),
            ]),
            _ => None,
        };

        if let Some(events) = fallback {
            self.scripted_turn += 1;
            self.pending_events.extend(events);
            if let Some(event) = self.pending_events.pop_front() {
                self.apply_event(event);
            }
        }
    }

    fn apply_event(&mut self, event: ScriptEvent) {
        match event {
            ScriptEvent::Tool(text) => {
                self.loader.set_message("Working".to_string());
                self.recent_tools.insert(0, text.clone());
                self.recent_tools.truncate(4);
                self.push_entry(Role::Tool, text);
            }
            ScriptEvent::Assistant(text) => {
                self.loader.set_message("Ready".to_string());
                self.push_entry(Role::Assistant, text);
            }
            ScriptEvent::MarkPlan(index) => {
                if let Some((done, _)) = self.plan.get_mut(index) {
                    *done = true;
                }
            }
            ScriptEvent::File(path) => {
                if !self.changed_files.iter().any(|existing| existing == &path) {
                    self.changed_files.push(path);
                }
            }
            ScriptEvent::Status(text) => {
                self.loader.set_message(text);
            }
        }
    }

    fn role_prefix(role: Role) -> String {
        match role {
            Role::User => bold(&yellow("You")),
            Role::Assistant => bold(&green("Agent")),
            Role::Tool => cyan("tool"),
            Role::System => dim("sys"),
        }
    }

    fn fit(text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let clipped = if visible_width(text) > width {
            truncate_to_width(text, width, "...", false)
        } else {
            text.to_string()
        };
        let pad = width.saturating_sub(visible_width(&clipped));
        format!("{clipped}{}", " ".repeat(pad))
    }

    fn transcript_lines(&self, width: usize) -> Vec<String> {
        let mut lines = vec![Self::fit(&bold("Conversation"), width), String::new()];
        let start = self.transcript.len().saturating_sub(10);
        for entry in &self.transcript[start..] {
            let raw = format!("{} {}", Self::role_prefix(entry.role), entry.text);
            for line in wrap_text_with_ansi(&raw, width) {
                lines.push(Self::fit(&line, width));
            }
            lines.push(String::new());
        }
        lines
    }

    fn sidebar_lines(&self, width: usize) -> Vec<String> {
        let mut lines = vec![Self::fit(&bold("Workspace"), width)];
        for raw in [
            format!("repo   {}", cyan("xylitol")),
            format!("branch {}", cyan("feat/tui-dev")),
            format!("model  {}", cyan("claude-sonnet-4")),
        ] {
            lines.push(Self::fit(&raw, width));
        }
        lines.push(String::new());
        lines.push(Self::fit(&bold("Plan"), width));
        for (done, text) in &self.plan {
            let marker = if *done { green("[x]") } else { yellow("[ ]") };
            lines.push(Self::fit(&format!("{marker} {text}"), width));
        }
        lines.push(String::new());
        lines.push(Self::fit(&bold("Changed Files"), width));
        for path in self.changed_files.iter().rev().take(4) {
            lines.push(Self::fit(&format!("- {path}"), width));
        }
        lines.push(String::new());
        lines.push(Self::fit(&bold("Recent Tools"), width));
        for tool in self.recent_tools.iter().take(4) {
            lines.push(Self::fit(&format!("- {tool}"), width));
        }
        lines
    }

    fn merge_columns(
        left: &[String],
        right: &[String],
        left_w: usize,
        right_w: usize,
    ) -> Vec<String> {
        let mut out = Vec::new();
        let rows = left.len().max(right.len());
        for i in 0..rows {
            let l = left.get(i).map(String::as_str).unwrap_or("");
            let r = right.get(i).map(String::as_str).unwrap_or("");
            out.push(format!(
                "{}  {}",
                Self::fit(l, left_w),
                Self::fit(r, right_w)
            ));
        }
        out
    }

    fn status_line(&mut self, width: usize) -> String {
        let activity = self
            .loader
            .render(width)
            .into_iter()
            .find(|line| !line.is_empty())
            .unwrap_or_else(|| dim("Ready"));
        let dynamic = if self.last_submitted.is_empty() {
            dim("last: waiting for prompt")
        } else {
            let summary_w = width.saturating_sub(visible_width(&activity) + 10);
            let summary = truncate_to_width(&self.last_submitted, summary_w.max(12), "...", false);
            format!("last: {}", dim(&summary))
        };
        Self::fit(&format!("{activity}  |  {dynamic}"), width)
    }

    fn render_palette_overlay(&mut self, width: usize, lines: &mut Vec<String>) {
        let overlay_w = width.min(54).saturating_sub(4).max(24);
        let mut panel = Panel::new(
            1,
            1,
            Some(Box::new(|s: &str| {
                format!("\x1b[47m\x1b[30m{s}\x1b[49m\x1b[39m")
            })),
        );
        panel.add_child(Box::new(Text::new(
            bold(" Command Palette").to_string(),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        let mut overlay = panel.render(overlay_w);
        overlay.extend(self.palette.render(overlay_w));
        overlay.push(String::new());
        overlay.push(Self::fit(
            &dim("  Up/Down select  Enter run  Esc close"),
            overlay_w,
        ));
        self.blit_overlay(lines, &overlay, width);
    }

    fn render_settings_overlay(&mut self, width: usize, lines: &mut Vec<String>) {
        let overlay_w = width.min(54).saturating_sub(4).max(24);
        let mut panel = Panel::new(
            1,
            1,
            Some(Box::new(|s: &str| {
                format!("\x1b[47m\x1b[30m{s}\x1b[49m\x1b[39m")
            })),
        );
        panel.add_child(Box::new(Text::new(
            bold(" Session Settings").to_string(),
            0,
            0,
        )));
        panel.add_child(Box::new(Spacer::new(1)));
        let mut overlay = panel.render(overlay_w);
        overlay.extend(self.settings.render(overlay_w));
        overlay.push(String::new());
        overlay.push(Self::fit(
            &dim("  Up/Down navigate  Enter confirm  Esc close"),
            overlay_w,
        ));
        self.blit_overlay(lines, &overlay, width);
    }

    fn blit_overlay(&self, lines: &mut Vec<String>, overlay: &[String], width: usize) {
        let overlay_w = overlay
            .iter()
            .map(|line| visible_width(line))
            .max()
            .unwrap_or(0);
        let col_off = width.saturating_sub(overlay_w) / 2;
        let row_off = 3usize;
        while lines.len() < row_off + overlay.len() {
            lines.push(String::new());
        }
        for (i, line) in overlay.iter().enumerate() {
            let row = row_off + i;
            let merged = format!("{}{}", " ".repeat(col_off), Self::fit(line, overlay_w));
            lines[row] = Self::fit(&merged, width);
        }
    }
}

impl Component for FakeCodingAgentApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();

        let mut header = Panel::new(2, 0, Some(Box::new(blue_bg)));
        header.add_child(Box::new(Text::new(
            format!("{} fake coding agent demo", bold("agent")),
            0,
            0,
        )));
        header.add_child(Box::new(Text::new(
            "main scenario only: transcript, tools, plan sidebar, editor, PTY-safe widths".into(),
            0,
            0,
        )));
        lines.extend(header.render(width));

        let status = format!(
            " {} | Ctrl+P palette  Ctrl+S settings  Enter submit  Ctrl+O advance ",
            dim("agent_demo"),
        );
        lines.push(Self::fit(&status, width));
        lines.push(Self::fit(&dim(&"-".repeat(width)), width));

        let sidebar_w = width.saturating_sub(56).clamp(24, 32);
        let left_w = width.saturating_sub(sidebar_w + 9);
        let body = Self::merge_columns(
            &self.transcript_lines(left_w),
            &self.sidebar_lines(sidebar_w),
            left_w,
            sidebar_w,
        );
        lines.extend(body);

        lines.push(Self::fit(&dim(&"-".repeat(width)), width));
        lines.push(self.status_line(width));
        lines.push(Self::fit(&dim(&self.footer_note), width));

        for line in self.input.render(width) {
            lines.push(Self::fit(&line, width));
        }

        if self.palette_open {
            self.render_palette_overlay(width, &mut lines);
        }
        if self.settings_open {
            self.render_settings_overlay(width, &mut lines);
        }

        lines
            .into_iter()
            .map(|line| Self::fit(&line, width))
            .collect()
    }

    fn handle_input(&mut self, data: &str) {
        if matches_key(data, "ctrl+c") {
            self.quit_flag.store(true, Ordering::SeqCst);
            return;
        }

        if matches_key(data, "escape") {
            self.palette_open = false;
            self.settings_open = false;
            return;
        }

        if self.palette_open {
            if matches_key(data, "up") {
                self.palette.handle_input("\x1b[A");
            } else if matches_key(data, "down") {
                self.palette.handle_input("\x1b[B");
            } else if matches_key(data, "enter") {
                if let Some(item) = self.palette.get_selected_item() {
                    match item.value.as_str() {
                        "tests" => {
                            self.pending_events.push_back(ScriptEvent::Tool(
                                "cargo test -p xylitol-tui --lib".into(),
                            ));
                            self.pending_events.push_back(ScriptEvent::Assistant(
                                "Regression tests are queued. Next step: rerun the PTY smoke against the primary example.".into(),
                            ));
                        }
                        "diff" => {
                            self.push_entry(
                                Role::Assistant,
                                "Current diff is concentrated in example consolidation, acceptance harness cleanup, and the E2E surface switch.",
                            );
                        }
                        "compact" => {
                            self.push_entry(
                                Role::System,
                                "Compaction checkpoint: examples rewritten to a single fake coding-agent flow.",
                            );
                        }
                        _ => {}
                    }
                }
                self.palette_open = false;
                self.advance_script();
            }
            return;
        }

        if self.settings_open {
            if matches_key(data, "up") {
                self.settings.handle_input("\x1b[A");
            } else if matches_key(data, "down") {
                self.settings.handle_input("\x1b[B");
            } else if matches_key(data, "enter") {
                self.settings.handle_input("\r");
            }
            return;
        }

        if matches_key(data, "ctrl+p") {
            self.palette_open = true;
            self.settings_open = false;
            return;
        }
        if matches_key(data, "ctrl+s") {
            self.settings_open = true;
            self.palette_open = false;
            return;
        }
        if matches_key(data, "ctrl+o") {
            self.advance_script();
            return;
        }

        self.input.handle_input(data);
        let submitted = { self.submit_slot.borrow_mut().take() };
        if let Some(text) = submitted {
            self.process_submit(text);
        }
    }

    fn invalidate(&mut self) {}

    fn tick(&mut self) -> bool {
        let mut changed = false;

        if self.last_tick_at.elapsed().as_millis() >= self.loader.interval_ms() as u128 {
            self.loader.tick();
            self.last_tick_at = Instant::now();
            changed = true;
        }

        if !self.auto_started {
            self.auto_started = true;
            self.advance_script();
            changed = true;
        } else if !self.pending_events.is_empty() || self.scripted_turn < 2 {
            self.advance_script();
            changed = true;
        }

        changed
    }
}

impl Focusable for FakeCodingAgentApp {
    fn set_focused(&mut self, _focused: bool) {}

    fn is_focused(&self) -> bool {
        true
    }
}
