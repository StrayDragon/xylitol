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
use xylitol_tui::components::text::Text;
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{
    Component, CrosstermTerminal, Focusable, InputEvent, SystemClock, TUI, matches_key_event,
    truncate_to_width, visible_width, wrap_text_with_ansi,
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
    let initial_prompt = std::env::var("XYLITOL_AGENT_DEMO_INITIAL_PROMPT")
        .unwrap_or_else(|_| "tighten footer truncation and add a PTY acceptance test".into());

    tui.add_child(Box::new(FakeCodingAgentApp::new_with_prompt(
        quit_flag.clone(),
        &initial_prompt,
    )));
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

enum TimedAction {
    Event(ScriptEvent),
    StreamStart,
    StreamChunk(String),
    StreamFinish,
}

struct ScheduledAction {
    at_tick: u64,
    action: TimedAction,
}

pub struct FakeCodingAgentApp {
    transcript: Vec<TranscriptEntry>,
    pending_events: VecDeque<ScriptEvent>,
    scheduled_actions: VecDeque<ScheduledAction>,
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
    status_text: String,
    active_stream_entry: Option<usize>,
    scripted_turn: usize,
    script_tick: u64,
    scheduled_tail_tick: u64,
    rng_state: u64,
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
            scheduled_actions: VecDeque::new(),
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
            status_text: "Ready".into(),
            active_stream_entry: None,
            scripted_turn: 0,
            script_tick: 0,
            scheduled_tail_tick: 0,
            rng_state: 0x5eed_c0de_u64,
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

        let last_line = trimmed.lines().last().unwrap_or_default();

        if last_line == "/palette" || last_line == ":palette" {
            self.input.set_text(String::new());
            self.palette_open = true;
            self.settings_open = false;
            return;
        }

        if last_line == "/settings" || last_line == ":settings" {
            self.input.set_text(String::new());
            self.settings_open = true;
            self.palette_open = false;
            return;
        }

        self.last_submitted = trimmed.clone();
        self.push_entry(Role::User, trimmed.clone());
        self.input.set_text(String::new());
        self.auto_started = true;
        self.scripted_turn = self.scripted_turn.max(2);
        self.queue_simulated_turn(&trimmed);
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status_text = text.into();
        self.loader.set_message(self.status_text.clone());
    }

    fn spinner_active(&self) -> bool {
        self.status_text != "Ready"
            || !self.pending_events.is_empty()
            || !self.scheduled_actions.is_empty()
            || self.active_stream_entry.is_some()
    }

    fn random_between(&mut self, min: u64, max: u64) -> u64 {
        debug_assert!(min <= max);
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        min + (self.rng_state % (max - min + 1))
    }

    fn schedule_after_ticks(&mut self, delay_ticks: u64, action: TimedAction) {
        self.scheduled_tail_tick = self.scheduled_tail_tick.max(self.script_tick);
        self.scheduled_tail_tick += delay_ticks.max(1);
        self.scheduled_actions.push_back(ScheduledAction {
            at_tick: self.scheduled_tail_tick,
            action,
        });
    }

    fn queue_event(&mut self, delay_ticks: u64, event: ScriptEvent) {
        self.schedule_after_ticks(delay_ticks, TimedAction::Event(event));
    }

    fn queue_assistant_stream(&mut self, text: &str) {
        let start_delay = self.random_between(4, 8);
        self.schedule_after_ticks(start_delay, TimedAction::StreamStart);

        let chars: Vec<char> = text.chars().collect();
        let mut index = 0usize;
        while index < chars.len() {
            let current = chars[index];
            let take = if current == '\n' {
                1
            } else if current.is_ascii() {
                self.random_between(2, 5) as usize
            } else {
                self.random_between(1, 2) as usize
            };
            let end = (index + take).min(chars.len());
            let chunk: String = chars[index..end].iter().collect();
            let chunk_delay = self.random_between(1, 4);
            self.schedule_after_ticks(chunk_delay, TimedAction::StreamChunk(chunk));
            index = end;
        }

        let finish_delay = self.random_between(3, 6);
        self.schedule_after_ticks(finish_delay, TimedAction::StreamFinish);
    }

    fn build_assistant_reply(&mut self, prompt: &str) -> String {
        let focus = if prompt.contains("CJK") || prompt.contains("emoji") {
            "我会先盯住 CJK/emoji 的宽度预算，再看真实终端回放。"
        } else if prompt.contains("palette") || prompt.contains("command") {
            "我会先看 overlay 覆盖语义，再补 PTY/tmux smoke。"
        } else {
            "我会先复现主流程，再把验收和宽度预算一起收紧。"
        };
        let closing = if self.random_between(0, 1) == 0 {
            "这段回复现在就是用打字机式流式输出。"
        } else {
            "接下来会按流式打字机节奏把结果一点点吐出来。"
        };
        format!(
            "收到，我已经接住 `{prompt}`。\n\n{focus}\n\n- 先排查提交路径\n- 再补真实终端 smoke\n- 最后回到 `Ready` 等下一条输入\n\n{closing}"
        )
    }

    fn queue_simulated_turn(&mut self, prompt: &str) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.scheduled_tail_tick = self.script_tick;
        self.active_stream_entry = None;
        self.set_status("Thinking");

        let rg_status_delay = self.random_between(3, 8);
        self.queue_event(
            rg_status_delay,
            ScriptEvent::Status("Running rg search".into()),
        );
        let rg_tool_delay = self.random_between(2, 5);
        self.queue_event(
            rg_tool_delay,
            ScriptEvent::Tool(format!("rg -n \"{}\" packages/xylitol-tui tests", prompt)),
        );
        let mark_one_delay = self.random_between(1, 3);
        self.queue_event(mark_one_delay, ScriptEvent::MarkPlan(1));
        let file_delay = self.random_between(2, 4);
        self.queue_event(
            file_delay,
            ScriptEvent::File("tests/tui_e2e/pty.rs".to_string()),
        );
        let test_status_delay = self.random_between(2, 5);
        self.queue_event(
            test_status_delay,
            ScriptEvent::Status("Running agent_demo acceptance".into()),
        );
        let test_tool_delay = self.random_between(2, 4);
        self.queue_event(
            test_tool_delay,
            ScriptEvent::Tool("cargo test -p xylitol-tui --test agent_demo_test".into()),
        );
        let mark_two_delay = self.random_between(1, 3);
        self.queue_event(mark_two_delay, ScriptEvent::MarkPlan(2));
        let drafting_delay = self.random_between(2, 4);
        self.queue_event(drafting_delay, ScriptEvent::Status("Drafting reply".into()));
        let reply = self.build_assistant_reply(prompt);
        self.queue_assistant_stream(&reply);
    }

    fn begin_assistant_stream(&mut self) {
        self.active_stream_entry = Some(self.transcript.len());
        self.transcript.push(TranscriptEntry {
            role: Role::Assistant,
            text: String::new(),
        });
        self.set_status("Drafting reply");
    }

    fn append_assistant_stream(&mut self, chunk: &str) {
        if self.active_stream_entry.is_none() {
            self.begin_assistant_stream();
        }
        if let Some(index) = self.active_stream_entry
            && let Some(entry) = self.transcript.get_mut(index)
        {
            entry.text.push_str(chunk);
        }
    }

    fn finish_assistant_stream(&mut self) {
        self.active_stream_entry = None;
        self.set_status("Ready");
    }

    fn process_due_actions(&mut self) -> bool {
        let mut changed = false;

        while let Some(front) = self.scheduled_actions.front() {
            if front.at_tick > self.script_tick {
                break;
            }

            let action = self
                .scheduled_actions
                .pop_front()
                .expect("front action should exist")
                .action;
            match action {
                TimedAction::Event(event) => self.apply_event(event),
                TimedAction::StreamStart => self.begin_assistant_stream(),
                TimedAction::StreamChunk(chunk) => self.append_assistant_stream(&chunk),
                TimedAction::StreamFinish => self.finish_assistant_stream(),
            }
            changed = true;
        }

        changed
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
                self.set_status("Working");
                self.recent_tools.insert(0, text.clone());
                self.recent_tools.truncate(4);
                self.push_entry(Role::Tool, text);
            }
            ScriptEvent::Assistant(text) => {
                if self.pending_events.is_empty()
                    && self.scheduled_actions.is_empty()
                    && self.active_stream_entry.is_none()
                {
                    self.set_status("Ready");
                }
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
                self.set_status(text);
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
        // Full history: the engine scrolls older rows into terminal scrollback
        // (pi chatContainer model). Do not truncate here — that would hide
        // history from the emulator's scrollback.
        let mut lines = vec![Self::fit(&bold("Conversation"), width), String::new()];
        for entry in &self.transcript {
            let raw = format!("{} {}", Self::role_prefix(entry.role), entry.text);
            for line in wrap_text_with_ansi(&raw, width) {
                lines.push(Self::fit(&line, width));
            }
            lines.push(String::new());
        }
        lines
    }

    /// Fixed-height debug strip below the editor (pi: widgets-below / footer
    /// density). Always exactly 3 content rows so streaming does not jitter
    /// the input box height.
    fn debug_strip_lines(&self, width: usize) -> Vec<String> {
        let done = self.plan.iter().filter(|(d, _)| *d).count();
        let next = self
            .plan
            .iter()
            .find(|(d, _)| !*d)
            .map(|(_, t)| t.as_str())
            .unwrap_or("done");
        let tool = self.recent_tools.first().map(String::as_str).unwrap_or("-");
        let files = self.changed_files.len();
        vec![
            Self::fit(
                &format!(
                    "{} plan {done}/{}  next: {}",
                    dim("dbg"),
                    self.plan.len(),
                    dim(next)
                ),
                width,
            ),
            Self::fit(&format!("{} tool {}", dim("dbg"), dim(tool)), width),
            Self::fit(
                &format!(
                    "{} files {files}  {} {}",
                    dim("dbg"),
                    cyan("xylitol"),
                    dim("feat/tui-dev")
                ),
                width,
            ),
        ]
    }

    fn status_line(&mut self, width: usize) -> String {
        let activity = if self.spinner_active() {
            self.loader
                .render(width)
                .into_iter()
                .find(|line| !line.is_empty())
                .unwrap_or_else(|| dim(&self.status_text))
        } else {
            dim("Ready")
        };
        let dynamic = if self.last_submitted.is_empty() {
            dim("last: waiting for prompt")
        } else {
            let summary_w = width.saturating_sub(visible_width(&activity) + 10);
            let summary = truncate_to_width(&self.last_submitted, summary_w.max(12), "...", false);
            format!("last: {}", dim(&summary))
        };
        Self::fit(&format!("{activity}  |  {dynamic}"), width)
    }

    /// pi `showSelector`: replace the editor slot (bottom of the stack) so the
    /// popup stays in the viewport as transcript grows into scrollback.
    fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        if self.palette_open {
            return self.render_palette_slot(width);
        }
        if self.settings_open {
            return self.render_settings_slot(width);
        }
        self.input
            .render(width)
            .into_iter()
            .map(|line| Self::fit(&line, width))
            .collect()
    }

    fn render_palette_slot(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(Self::fit(&bold(" Command Palette"), width));
        lines.push(Self::fit(&dim(" Up/Down  Enter run  Esc close"), width));
        for line in self.palette.render(width) {
            lines.push(Self::fit(&line, width));
        }
        lines
    }

    fn render_settings_slot(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(Self::fit(&bold(" Session Settings"), width));
        lines.push(Self::fit(&dim(" Up/Down  Enter confirm  Esc close"), width));
        for line in self.settings.render(width) {
            lines.push(Self::fit(&line, width));
        }
        lines
    }
}

impl Component for FakeCodingAgentApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        // Single-column stack (pi interactive): transcript → status → editor
        // slot (or selector in place of editor) → debug → footer.
        // Selectors replace the editor slot so they stay in the viewport as
        // transcript grows into scrollback (pi showSelector / editorContainer).
        let mut lines = Vec::new();

        let mut header = Panel::new(1, 0, Some(Box::new(blue_bg)));
        header.add_child(Box::new(Text::new(
            format!(
                "{}  transcript · editor · scrollback  |  Ctrl+P/S selector",
                bold("agent_demo")
            ),
            0,
            0,
        )));
        lines.extend(header.render(width));
        lines.push(Self::fit(&dim(&"-".repeat(width)), width));

        lines.extend(self.transcript_lines(width));

        lines.push(Self::fit(&dim(&"-".repeat(width)), width));
        lines.push(self.status_line(width));
        lines.extend(self.render_editor_slot(width));

        lines.push(Self::fit(&dim(&"-".repeat(width)), width));
        lines.extend(self.debug_strip_lines(width));
        let footer = if self.palette_open || self.settings_open {
            "Esc close selector  |  Ctrl+C quit"
        } else {
            self.footer_note.as_str()
        };
        lines.push(Self::fit(&dim(footer), width));

        lines
            .into_iter()
            .map(|line| Self::fit(&line, width))
            .collect()
    }

    fn handle_input(&mut self, event: InputEvent) {
        let key = match &event {
            InputEvent::Key(k) => k,
            InputEvent::Paste(_) => {
                self.input.handle_input(event);
                let submitted = { self.submit_slot.borrow_mut().take() };
                if let Some(text) = submitted {
                    self.process_submit(text);
                }
                return;
            }
        };

        if matches_key_event(key, "ctrl+c") {
            self.quit_flag.store(true, Ordering::SeqCst);
            return;
        }

        if matches_key_event(key, "escape") {
            self.palette_open = false;
            self.settings_open = false;
            return;
        }

        if self.palette_open {
            if matches_key_event(key, "up") || matches_key_event(key, "down") {
                self.palette.handle_input(event);
            } else if matches_key_event(key, "enter") {
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
            if matches_key_event(key, "up")
                || matches_key_event(key, "down")
                || matches_key_event(key, "enter")
            {
                self.settings.handle_input(event);
            }
            return;
        }

        if matches_key_event(key, "ctrl+p") {
            self.palette_open = true;
            self.settings_open = false;
            return;
        }
        if matches_key_event(key, "ctrl+s") {
            self.settings_open = true;
            self.palette_open = false;
            return;
        }
        if matches_key_event(key, "ctrl+o") {
            self.advance_script();
            return;
        }

        self.input.handle_input(event);
        let submitted = { self.submit_slot.borrow_mut().take() };
        if let Some(text) = submitted {
            self.process_submit(text);
        }
    }

    fn invalidate(&mut self) {}

    fn tick(&mut self) -> bool {
        let mut changed = false;
        self.script_tick = self.script_tick.saturating_add(1);

        if self.spinner_active()
            && self.last_tick_at.elapsed().as_millis() >= self.loader.interval_ms() as u128
        {
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

        changed |= self.process_due_actions();

        changed
    }
}

impl Focusable for FakeCodingAgentApp {
    fn set_focused(&mut self, _focused: bool) {}

    fn is_focused(&self) -> bool {
        true
    }
}
