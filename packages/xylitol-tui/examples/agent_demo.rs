use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use xylitol_tui::autocomplete::SlashCommand;
use xylitol_tui::completion::{AtPathSource, SlashCommandSource};
use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
use xylitol_tui::components::settings_list::{
    SettingItem, SettingsList, SettingsListOptions, SettingsListTheme,
};
use xylitol_tui::keybindings::{KeybindingsManager, create_default_definitions, set_keybindings};
use xylitol_tui::{
    Component, CrosstermTerminal, DiffInput, DiffOptions, DiffTheme, Focusable, InputEvent,
    InputListenerResult, Markdown, MarkdownOptions, MarkdownTheme, SystemClock, TUI,
    apply_background_to_line, highlight_code, matches_key_event, render_diff_lines,
    truncate_to_width, visible_width, wrap_text_with_ansi,
};

/// Demo slash commands (static; product would load from Driver / protocol).
/// Names omit the leading `/` — Editor's CombinedAutocompleteProvider adds it.
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("help", "Show this help"),
    ("model", "Switch execution model"),
    ("compact", "Compact conversation history"),
    ("export", "Export current session"),
    ("session", "Session management"),
    ("settings", "Open settings panel"),
    ("palette", "Open command palette"),
    ("diff", "Show workspace diff"),
];

fn slash_commands() -> Vec<SlashCommand> {
    SLASH_COMMANDS
        .iter()
        .map(|(name, desc)| SlashCommand {
            name: (*name).to_string(),
            description: Some((*desc).to_string()),
            argument_hint: None,
            get_argument_completions: None,
        })
        .collect()
}

fn cyan(s: &str) -> String {
    format!("\x1b[36m{s}\x1b[39m")
}

/// Wrap a key chord for block-adjacent hints: `(Ctrl+T)`.
fn key_hint(chord: &str) -> String {
    dim(&format!("({chord})"))
}

/// Tool / edit block execution tint (DESIGN.md `tool-*-bg`, Mocha).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolBlockStatus {
    Pending,
    Success,
    Error,
}

impl ToolBlockStatus {
    /// RGB matching `src/app/tui/DESIGN.md` colors.tool-*-bg.
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Pending => (0x31, 0x32, 0x44), // #313244
            Self::Success => (0x24, 0x35, 0x2a), // #24352a
            Self::Error => (0x35, 0x24, 0x28),   // #352428
        }
    }

    /// Truecolor bg open sequence (`48;2;R;G;B`) — for docs / raw-ANSI asserts.
    #[allow(dead_code)]
    pub fn ansi_bg_param(self) -> String {
        let (r, g, b) = self.rgb();
        format!("48;2;{r};{g};{b}")
    }
}

/// Full-row tint: truecolor bg + `\x1b[49m` only (must not wipe content fg).
fn paint_tool_bg(line: &str, width: usize, status: ToolBlockStatus) -> String {
    let (r, g, b) = status.rgb();
    apply_background_to_line(line, width, &|s| {
        format!("\x1b[48;2;{r};{g};{b}m{s}\x1b[49m")
    })
}

/// Richer unified sample via pi edit format (aligned `±N content`).
fn sample_unified_pair() -> DiffInput {
    DiffInput::from_edit_pair(
        "fn ready() -> bool {\n    true\n}\n",
        "fn ready(prompt: &str) -> bool {\n    !prompt.is_empty()\n}\n",
    )
}

/// Side-by-side sample — LinePair so SBS layout is exercised; compact gutters by default.
fn sample_sbs_pair() -> DiffInput {
    DiffInput::LinePair {
        old: "status: Ready\nfooter: cwd · model\n".into(),
        new: "status: Working\nfooter: cwd · model · context%\n".into(),
        path: Some("src/app/tui/ui_root.rs".into()),
    }
}

/// Legacy display_diff gutter sample (still exercised).
fn sample_display_diff() -> String {
    [
        "      ... | --- a/packages/xylitol-tui/examples/agent_demo.rs",
        "      ... | +++ b/packages/xylitol-tui/examples/agent_demo.rs",
        "  10    10 |     Component, CrosstermTerminal,",
        "  11       | -    Focusable, InputEvent,",
        "       11 | +    DiffInput, Focusable, InputEvent,",
    ]
    .join("\n")
}

/// Larger edit-tool style sample (context + change) for simulated Edit steps.
fn sample_edit_tool_pair() -> DiffInput {
    DiffInput::from_edit_pair(
        "pub fn footer_note(cwd: &str, model: &str) -> String {\n    format!(\"{cwd} · {model}\")\n}\n",
        "pub fn footer_note(cwd: &str, model: &str, ctx: u8) -> String {\n    format!(\"{cwd} · {model} · {ctx}%\")\n}\n",
    )
}

fn demo_markdown_theme() -> MarkdownTheme {
    let id = |s: &str| s.to_string();
    MarkdownTheme {
        heading: Box::new(bold),
        link: Box::new(cyan),
        link_url: Box::new(dim),
        code: Box::new(|s| format!("\x1b[36m{s}\x1b[39m")),
        code_block: Box::new(|s| s.to_string()),
        // DESIGN: no fence chrome
        code_block_border: Box::new(|_| String::new()),
        quote: Box::new(dim),
        quote_border: Box::new(dim),
        hr: Box::new(dim),
        list_bullet: Box::new(id),
        bold: Box::new(bold),
        italic: Box::new(|s| format!("\x1b[3m{s}\x1b[23m")),
        strikethrough: Box::new(|s| format!("\x1b[9m{s}\x1b[29m")),
        underline: Box::new(|s| format!("\x1b[4m{s}\x1b[24m")),
        highlight_code: Some(Box::new(highlight_code)),
        code_block_indent: None,
    }
}
fn magenta(s: &str) -> String {
    format!("\x1b[35m{s}\x1b[39m")
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

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        quit_flag.clone(),
        &initial_prompt,
    )));
    FakeCodingAgentApp::install_input_listeners(&app, &mut tui);
    tui.add_child(Box::new(SharedFakeCodingAgentApp(app)));
    tui.set_focus(Some(0));
    tui.start_with_flag(&quit_flag)
}

/// Thin `Component` wrapper so input listeners can share the same app state.
pub struct SharedFakeCodingAgentApp(pub Rc<RefCell<FakeCodingAgentApp>>);

impl Component for SharedFakeCodingAgentApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.0.borrow_mut().render(width)
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.0.borrow_mut().handle_input(event);
    }

    fn invalidate(&mut self) {
        self.0.borrow_mut().invalidate();
    }

    fn tick(&mut self) -> bool {
        self.0.borrow_mut().tick()
    }

    fn wants_key_release(&self) -> bool {
        self.0.borrow().wants_key_release()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    System,
}

/// App-layer glyph config (DESIGN.md): no font probing — env / Alt+G only.
/// (Avoid Ctrl+G: reserved for future external-editor open.)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GlyphSet {
    Unicode,
    Ascii,
}

impl GlyphSet {
    fn from_env() -> Self {
        match std::env::var("XYLITOL_TUI_GLYPH_SET").ok().as_deref() {
            Some("ascii") | Some("ASCII") => Self::Ascii,
            _ => Self::Unicode,
        }
    }

    fn cycle(self) -> Self {
        match self {
            Self::Unicode => Self::Ascii,
            Self::Ascii => Self::Unicode,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Unicode => "unicode",
            Self::Ascii => "ascii",
        }
    }

    fn user(self) -> &'static str {
        match self {
            Self::Unicode => "❯",
            Self::Ascii => ">",
        }
    }

    fn tool(self) -> &'static str {
        match self {
            Self::Unicode => "⚙",
            Self::Ascii => "*",
        }
    }

    fn system(self) -> &'static str {
        match self {
            Self::Unicode => "·",
            Self::Ascii => ".",
        }
    }

    fn fold(self) -> &'static str {
        match self {
            Self::Unicode => "▶",
            Self::Ascii => ">",
        }
    }

    fn unfold(self) -> &'static str {
        match self {
            Self::Unicode => "▼",
            Self::Ascii => "v",
        }
    }
}

enum TranscriptEntry {
    Message {
        role: Role,
        text: String,
    },
    /// Collapsible thinking block (pi-style ExpandableText preview).
    Thinking {
        expanded: bool,
        body: String,
    },
    /// Collapsible tool block: one-line summary; detail when expanded.
    Tool {
        expanded: bool,
        status: ToolBlockStatus,
        summary: String,
        detail: String,
    },
    /// Collapsible Diff block (c451 `Diff` / `render_diff_lines`).
    Diff {
        expanded: bool,
        status: ToolBlockStatus,
        summary: String,
        input: DiffInput,
        /// `None` = always unified; `Some(n)` = side-by-side when width ≥ n.
        side_by_side_min_width: Option<usize>,
    },
}

enum ScriptEvent {
    Tool(String),
    /// Agent Edit tool: summary line + expanded pi-format Diff (pops open like pi).
    Edit {
        summary: String,
        input: DiffInput,
    },
    Assistant(String),
    MarkPlan(usize),
    File(String),
    Status(String),
}

enum TimedAction {
    Event(ScriptEvent),
    StreamStart(StreamKind),
    StreamChunk(StreamKind, String),
    StreamFinish(StreamKind),
    /// Flip a specific Tool/Diff entry (must bind index — never "last").
    SetToolStatus {
        index: usize,
        status: ToolBlockStatus,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamKind {
    Thinking,
    Assistant,
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
    /// Rotates default streamed fence language when prompt has no lang keyword.
    fence_rotate: usize,
    auto_started: bool,
    last_tick_at: Instant,
    quit_flag: Arc<AtomicBool>,
    glyph_set: GlyphSet,
}

impl FakeCodingAgentApp {
    pub fn new(quit_flag: Arc<AtomicBool>) -> Self {
        Self::new_with_prompt(
            quit_flag,
            "tighten footer truncation and add a PTY acceptance test",
        )
    }

    /// Register pre-focus listeners for Ctrl+C / Esc (c455).
    pub fn install_input_listeners(
        app: &Rc<RefCell<Self>>,
        tui: &mut TUI<impl xylitol_tui::Terminal>,
    ) {
        let app_ctrl = app.clone();
        tui.add_input_listener(move |event| {
            let InputEvent::Key(key) = &event else {
                return InputListenerResult::Continue;
            };
            if matches_key_event(key, "ctrl+c") {
                app_ctrl.borrow_mut().on_ctrl_c();
                return InputListenerResult::Consumed;
            }
            if matches_key_event(key, "escape") && app_ctrl.borrow_mut().on_escape() {
                return InputListenerResult::Consumed;
            }
            InputListenerResult::Continue
        });
    }

    /// Ctrl+C: clear editor when non-empty; otherwise quit.
    pub fn on_ctrl_c(&mut self) {
        if !self.input.get_text().is_empty() {
            self.input.set_text(String::new());
            return;
        }
        self.quit_flag.store(true, Ordering::SeqCst);
    }

    /// Test helper: current editor text.
    pub fn input_text_for_test(&self) -> String {
        self.input.get_text()
    }

    pub fn status_text_for_test(&self) -> &str {
        &self.status_text
    }

    pub fn clear_scheduled_actions_for_test(&mut self) {
        self.scheduled_actions.clear();
    }

    /// Stop idle fallback turns from interfering with harness injects.
    pub fn freeze_script_for_test(&mut self) {
        self.auto_started = true;
        self.scripted_turn = 99;
        self.pending_events.clear();
    }

    /// Harness: push a pending tool (no long scripted turn). Returns transcript index.
    pub fn inject_pending_tool_for_test(&mut self) -> usize {
        let index = self.transcript.len();
        self.push_tool(
            "inject-tool · running",
            "pending detail (demo)",
            ToolBlockStatus::Pending,
        );
        self.set_status("Working");
        index
    }

    /// Harness: push an already-finished tool (header has cmd; detail has no `$` echo).
    pub fn push_tool_for_test(&mut self, summary: impl Into<String>, detail: impl Into<String>) {
        self.push_tool(summary, detail, ToolBlockStatus::Success);
    }

    /// Harness: flip a specific tool/diff entry to success.
    pub fn complete_tool_at_for_test(&mut self, index: usize) {
        self.set_tool_status_at(index, ToolBlockStatus::Success);
    }

    /// Harness: schedule independent flips for two pending tools (parallel feel).
    pub fn inject_parallel_pending_tools_for_test(&mut self) -> (usize, usize) {
        let a = self.inject_pending_tool_for_test();
        // Second tool with a distinct summary.
        let b = self.transcript.len();
        self.push_tool(
            "inject-tool-b · running",
            "pending detail B (demo)",
            ToolBlockStatus::Pending,
        );
        self.schedule_from_now(
            3,
            TimedAction::SetToolStatus {
                index: a,
                status: ToolBlockStatus::Success,
            },
        );
        self.schedule_from_now(
            8,
            TimedAction::SetToolStatus {
                index: b,
                status: ToolBlockStatus::Success,
            },
        );
        (a, b)
    }

    /// Esc: close overlays; abort active stream; otherwise let Editor handle.
    /// Returns true if the event was consumed.
    pub fn on_escape(&mut self) -> bool {
        if self.palette_open || self.settings_open {
            self.palette_open = false;
            self.settings_open = false;
            return true;
        }
        if self.active_stream_entry.is_some() || !self.scheduled_actions.is_empty() {
            self.abort_active_stream();
            return true;
        }
        false
    }

    fn abort_active_stream(&mut self) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.active_stream_entry = None;
        self.set_status("Ready");
        self.push_message(Role::System, "stream aborted");
    }

    pub fn new_with_prompt(quit_flag: Arc<AtomicBool>, initial_prompt: &str) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self::new_with_prompt_at(quit_flag, initial_prompt, cwd)
    }

    /// Like [`new_with_prompt`] but pins `@` path completion to `cwd` (tests / demos).
    pub fn new_with_prompt_at(
        quit_flag: Arc<AtomicBool>,
        initial_prompt: &str,
        cwd: std::path::PathBuf,
    ) -> Self {
        let submit_slot = Rc::new(RefCell::new(None));
        let submit_clone = submit_slot.clone();

        let mut input = Editor::new(
            EditorTheme {
                // Muted operation-zone border (DESIGN.md / fig2).
                border_color: Box::new(dim),
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
        // Pluggable CompletionSources: `/` slash cmds + `@` path picker.
        // Demo-only static lists — no Driver wiring. Future `$`/`^` = more sources.
        input.set_completion_sources(vec![
            Box::new(SlashCommandSource::new(slash_commands())),
            Box::new(AtPathSource::new(cwd)),
        ]);
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
            // Keep short: 80-col harness must fit note + glyph + key cues.
            footer_note: "~/xylitol · sonnet-4".into(),
            last_submitted: String::new(),
            status_text: "Ready".into(),
            active_stream_entry: None,
            scripted_turn: 0,
            script_tick: 0,
            scheduled_tail_tick: 0,
            rng_state: 0x5eed_c0de_u64,
            fence_rotate: 0,
            auto_started: false,
            last_tick_at: Instant::now(),
            quit_flag,
            glyph_set: GlyphSet::from_env(),
        };
        app.seed_transcript();
        app
    }

    fn seed_transcript(&mut self) {
        // One-shot help — fold keys live on blocks as `(Ctrl+T)` / `(Alt+E)`.
        self.push_message(
            Role::System,
            "keys: Enter submit · /cmds · @path · (Ctrl+P) palette · (Ctrl+S) settings · (Alt+G) glyphs · (Ctrl+O) step · Esc · (Ctrl+C)",
        );
        self.push_message(
            Role::System,
            "stream fence: prompt 含 rust/python/typescript/json 定点语言；否则每轮轮换",
        );
        self.push_message(
            Role::User,
            "Collapse examples into one fake coding-agent demo and keep foot interaction stable.",
        );
        self.push_thinking(
            "Plan: read existing examples and the pi coding-agent ExpandableText flow, then rebuild one stable primary scenario with real terminal acceptance coverage.\n\nKeep transcript in scrollback; mark the editor as the operation zone with borders.",
        );
        self.push_message(
            Role::Assistant,
            "Read the existing examples and the pi coding-agent flow first, then rebuild one stable primary scenario with real terminal acceptance coverage.\n\n```rust\nfn demo() {\n    println!(\"highlight\");\n}\n```",
        );
        self.push_tool(
            "read packages/xylitol-tui/examples/agent_demo.rs · 42ms · 790 lines",
            "ok — opened agent_demo.rs\n(preview) FakeCodingAgentApp + scripted turn harness",
            ToolBlockStatus::Success,
        );
        // Seed blocks start expanded so SBS / edit / gutter are visible without Alt+E.
        // Primary Edit look: pi unified compact (seed + simulated Edit tool).
        self.push_diff_ex(
            "edited demo.rs (+2 -2) unified edit-format",
            sample_unified_pair(),
            None, // always unified — Edit tool path
            true,
            ToolBlockStatus::Success,
        );
        // Optional wide layout (supported, uncommon); packed columns, not half-stretch.
        self.push_diff_ex(
            "edited ui_root.rs (+2 -2) side-by-side (optional)",
            sample_sbs_pair(),
            Some(60),
            true,
            ToolBlockStatus::Success,
        );
        // display_diff gutter path still covered.
        self.push_diff_ex(
            "edited demo.rs (display_diff gutter)",
            DiffInput::DisplayText(sample_display_diff()),
            None,
            true,
            ToolBlockStatus::Success,
        );
        // Error tint exemplar (collapsed detail still paints header).
        self.push_tool(
            "cargo test -p xylitol-tui --test missing · fail",
            "error — test binary `missing` not found (demo stub)",
            ToolBlockStatus::Error,
        );
    }

    fn push_message(&mut self, role: Role, text: impl Into<String>) {
        self.transcript.push(TranscriptEntry::Message {
            role,
            text: text.into(),
        });
    }

    fn push_thinking(&mut self, body: impl Into<String>) {
        self.transcript.push(TranscriptEntry::Thinking {
            expanded: true,
            body: body.into(),
        });
    }

    fn push_tool(
        &mut self,
        summary: impl Into<String>,
        detail: impl Into<String>,
        status: ToolBlockStatus,
    ) {
        self.transcript.push(TranscriptEntry::Tool {
            expanded: true,
            status,
            summary: summary.into(),
            detail: detail.into(),
        });
    }

    fn push_diff_ex(
        &mut self,
        summary: impl Into<String>,
        input: DiffInput,
        side_by_side_min_width: Option<usize>,
        expanded: bool,
        status: ToolBlockStatus,
    ) {
        self.transcript.push(TranscriptEntry::Diff {
            expanded,
            status,
            summary: summary.into(),
            input,
            side_by_side_min_width,
        });
    }

    fn toggle_thinking_blocks(&mut self) {
        // If anything is open, close all (so mid-stream ^T can hide the live
        // typewriter). Only expand when every thinking block is already closed.
        let any_expanded = self
            .transcript
            .iter()
            .any(|e| matches!(e, TranscriptEntry::Thinking { expanded: true, .. }));
        for entry in &mut self.transcript {
            if let TranscriptEntry::Thinking { expanded, .. } = entry {
                *expanded = !any_expanded;
            }
        }
    }

    fn toggle_tool_blocks(&mut self) {
        let any_expanded = self.transcript.iter().any(|e| {
            matches!(
                e,
                TranscriptEntry::Tool { expanded: true, .. }
                    | TranscriptEntry::Diff { expanded: true, .. }
            )
        });
        for entry in &mut self.transcript {
            match entry {
                TranscriptEntry::Tool { expanded, .. } | TranscriptEntry::Diff { expanded, .. } => {
                    *expanded = !any_expanded;
                }
                _ => {}
            }
        }
    }

    fn cycle_glyph_set(&mut self) {
        self.glyph_set = self.glyph_set.cycle();
        self.push_message(
            Role::System,
            format!(
                "glyph_set={} (Alt+G cycle; or XYLITOL_TUI_GLYPH_SET=ascii|unicode)",
                self.glyph_set.label()
            ),
        );
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
        self.push_message(Role::User, trimmed.clone());
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
            || self.any_pending_tool_blocks()
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

    /// Uneven delay: mostly short, sometimes a hitch (avoids metronome feel).
    fn jitter_ticks(&mut self, short_lo: u64, short_hi: u64) -> u64 {
        match self.random_between(0, 9) {
            0 => self.random_between(short_hi.saturating_add(8), short_hi.saturating_add(28)),
            1..=2 => self.random_between(short_hi.saturating_add(2), short_hi.saturating_add(10)),
            _ => self.random_between(short_lo, short_hi),
        }
    }

    fn schedule_after_ticks(&mut self, delay_ticks: u64, action: TimedAction) {
        self.scheduled_tail_tick = self.scheduled_tail_tick.max(self.script_tick);
        self.scheduled_tail_tick += delay_ticks.max(1);
        self.scheduled_actions.push_back(ScheduledAction {
            at_tick: self.scheduled_tail_tick,
            action,
        });
    }

    /// Schedule relative to *now* (not the serial event tail) — for independent tool flips.
    fn schedule_from_now(&mut self, delay_ticks: u64, action: TimedAction) {
        let at = self.script_tick.saturating_add(delay_ticks.max(1));
        self.scheduled_actions.push_back(ScheduledAction {
            at_tick: at,
            action,
        });
        self.scheduled_tail_tick = self.scheduled_tail_tick.max(at);
    }

    fn queue_event(&mut self, delay_ticks: u64, event: ScriptEvent) {
        self.schedule_after_ticks(delay_ticks, TimedAction::Event(event));
    }

    fn queue_stream(&mut self, kind: StreamKind, text: &str) {
        // Pause before first token (model "spin up").
        let start_delay = self.jitter_ticks(6, 14);
        self.schedule_after_ticks(start_delay, TimedAction::StreamStart(kind));

        let chars: Vec<char> = text.chars().collect();
        let mut index = 0usize;
        while index < chars.len() {
            let current = chars[index];
            // Burst vs drip: occasionally dump a longer run, often 1–3 chars.
            let take = if current == '\n' {
                1
            } else {
                match self.random_between(0, 9) {
                    0..=1 => self.random_between(6, 14) as usize, // burst
                    2..=4 => self.random_between(3, 6) as usize,
                    _ => {
                        if current.is_ascii() {
                            self.random_between(1, 3) as usize
                        } else {
                            1
                        }
                    }
                }
            };
            let end = (index + take).min(chars.len());
            let chunk: String = chars[index..end].iter().collect();
            // Thinking streams a bit slower / hitchier than the final reply.
            let chunk_delay = match kind {
                StreamKind::Thinking => self.jitter_ticks(2, 7),
                StreamKind::Assistant => self.jitter_ticks(1, 5),
            };
            self.schedule_after_ticks(chunk_delay, TimedAction::StreamChunk(kind, chunk));
            index = end;
        }

        let finish_delay = self.jitter_ticks(4, 10);
        self.schedule_after_ticks(finish_delay, TimedAction::StreamFinish(kind));
    }

    fn queue_thinking_stream(&mut self, text: &str) {
        self.queue_stream(StreamKind::Thinking, text);
    }

    fn queue_assistant_stream(&mut self, text: &str) {
        self.queue_stream(StreamKind::Assistant, text);
    }

    /// Pick a streamed fence: keyword wins; otherwise rotate rust→python→ts→json.
    fn pick_stream_fence(&mut self, prompt: &str) -> (&'static str, &'static str) {
        let p = prompt.to_ascii_lowercase();
        let (lang, focus) = if p.contains("python") || p.contains("py ") {
            (
                "python",
                "下面流式吐一段 Python fence，对照其它语言看多语言高亮。",
            )
        } else if p.contains("typescript")
            || p.contains(".ts")
            || p.split_whitespace().any(|w| w == "ts" || w == "tsx")
        {
            (
                "typescript",
                "下面流式吐一段 TypeScript fence，验收 syntect 在 TS 上的着色。",
            )
        } else if p.contains("json") {
            (
                "json",
                "下面流式吐一段 JSON fence，看结构字面量高亮是否干净。",
            )
        } else if p.contains("rust") || p.contains("highlight") || p.contains("stream") {
            (
                "rust",
                "下面会流式吐出一段带 fence 的 Rust，用来验收 syntect 在未闭合→闭合过程中的表现。",
            )
        } else if p.contains("cjk") || p.contains("emoji") {
            (
                "rust",
                "我会先盯住 CJK/emoji 的宽度预算，再看真实终端回放。",
            )
        } else if p.contains("palette") || p.contains("command") {
            ("rust", "我会先看 overlay 覆盖语义，再补 PTY/tmux smoke。")
        } else {
            let langs = ["rust", "python", "typescript", "json"];
            let lang = langs[self.fence_rotate % langs.len()];
            self.fence_rotate = self.fence_rotate.wrapping_add(1);
            (
                lang,
                "我会先复现主流程；本轮流式 fence 语言会轮换，方便肉眼对比高亮。",
            )
        };

        let fence = match lang {
            "python" => {
                "```python\ndef accept(prompt: str) -> bool:\n    # streamed fence — watch highlight land as the block closes\n    return bool(prompt)\n```"
            }
            "typescript" => {
                "```typescript\nfunction accept(prompt: string): boolean {\n  // streamed fence — watch highlight land as the block closes\n  return prompt.length > 0;\n}\n```"
            }
            "json" => {
                "```json\n{\n  \"accept\": true,\n  \"note\": \"streamed fence — watch highlight land as the block closes\"\n}\n```"
            }
            _ => {
                "```rust\nfn accept(prompt: &str) -> bool {\n    // streamed fence — watch highlight land as the block closes\n    !prompt.is_empty()\n}\n```"
            }
        };
        (focus, fence)
    }

    fn build_assistant_reply(&mut self, prompt: &str) -> String {
        let (focus, fence) = self.pick_stream_fence(prompt);

        let closing = if self.random_between(0, 1) == 0 {
            "这段回复现在就是用打字机式流式输出。"
        } else {
            "接下来会按流式打字机节奏把结果一点点吐出来。"
        };
        format!(
            "收到，我已经接住 `{prompt}`。\n\n{focus}\n\n\
             {fence}\n\n\
             - 先排查提交路径\n\
             - 再补真实终端 smoke\n\
             - 最后回到 `Ready` 等下一条输入\n\n{closing}"
        )
    }

    fn queue_simulated_turn(&mut self, prompt: &str) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.scheduled_tail_tick = self.script_tick;
        self.active_stream_entry = None;
        self.set_status("Thinking");

        let thinking_body = format!(
            "User asked: {prompt}\n\nI'll search the tree, run acceptance, then stream a reply.\n\n(hesitating on width budget vs scrollback…)"
        );
        // Typewriter the thinking block first (auto-expanded while streaming).
        self.queue_thinking_stream(&thinking_body);

        // After thinking: spawn a parallel-ish tool wave (tight appear, independent flips).
        let think_dwell = self.jitter_ticks(8, 18);
        self.queue_event(think_dwell, ScriptEvent::Status("Running tools".into()));
        self.queue_event(
            1,
            ScriptEvent::Tool(format!("rg -n \"{}\" packages/xylitol-tui tests", prompt)),
        );
        self.queue_event(1, ScriptEvent::MarkPlan(1));
        self.queue_event(1, ScriptEvent::File("tests/tui_e2e/pty.rs".to_string()));
        self.queue_event(
            1,
            ScriptEvent::Tool("cargo test -p xylitol-tui --test agent_demo_test".into()),
        );
        self.queue_event(1, ScriptEvent::MarkPlan(2));
        self.queue_event(
            1,
            ScriptEvent::Edit {
                summary: "edit src/app/tui/ui_root.rs (+1 -1)".into(),
                input: sample_edit_tool_pair(),
            },
        );
        self.queue_event(1, ScriptEvent::MarkPlan(3));
        // Let overlapping Pending flips breathe before drafting.
        let drafting_delay = self.jitter_ticks(18, 36);
        self.queue_event(drafting_delay, ScriptEvent::Status("Drafting reply".into()));
        let reply = self.build_assistant_reply(prompt);
        self.queue_assistant_stream(&reply);
    }

    fn begin_stream(&mut self, kind: StreamKind) {
        self.active_stream_entry = Some(self.transcript.len());
        match kind {
            StreamKind::Thinking => {
                self.transcript.push(TranscriptEntry::Thinking {
                    // Expanded while streaming so the typewriter is visible.
                    expanded: true,
                    body: String::new(),
                });
                self.set_status("Thinking");
            }
            StreamKind::Assistant => {
                self.transcript.push(TranscriptEntry::Message {
                    role: Role::Assistant,
                    text: String::new(),
                });
                self.set_status("Drafting reply");
            }
        }
    }

    fn append_stream(&mut self, kind: StreamKind, chunk: &str) {
        if self.active_stream_entry.is_none() {
            self.begin_stream(kind);
        }
        if let Some(index) = self.active_stream_entry {
            match (kind, self.transcript.get_mut(index)) {
                (StreamKind::Thinking, Some(TranscriptEntry::Thinking { body, .. })) => {
                    // Append only — do not force expand. ^T during stream must stick.
                    body.push_str(chunk);
                }
                (StreamKind::Assistant, Some(TranscriptEntry::Message { text, .. })) => {
                    text.push_str(chunk);
                }
                _ => {}
            }
        }
    }

    fn finish_stream(&mut self, kind: StreamKind) {
        if let Some(index) = self.active_stream_entry
            && kind == StreamKind::Thinking
            && let Some(TranscriptEntry::Thinking { expanded, .. }) = self.transcript.get_mut(index)
        {
            // Default tidy: collapse when the stream ends. User can ^T reopen.
            // (Does not fight mid-stream ^T — that only mattered while appending.)
            *expanded = false;
        }
        self.active_stream_entry = None;
        if kind == StreamKind::Assistant {
            self.set_status("Ready");
        }
    }

    fn process_due_actions(&mut self) -> bool {
        // Flips use schedule_from_now and may interleave past serial events — drain all due.
        let tick = self.script_tick;
        let mut due = Vec::new();
        let mut rest = VecDeque::new();
        while let Some(action) = self.scheduled_actions.pop_front() {
            if action.at_tick <= tick {
                due.push(action.action);
            } else {
                rest.push_back(action);
            }
        }
        self.scheduled_actions = rest;

        let mut changed = false;
        for action in due {
            match action {
                TimedAction::Event(event) => self.apply_event(event),
                TimedAction::StreamStart(kind) => self.begin_stream(kind),
                TimedAction::StreamChunk(kind, chunk) => self.append_stream(kind, &chunk),
                TimedAction::StreamFinish(kind) => self.finish_stream(kind),
                TimedAction::SetToolStatus { index, status } => {
                    self.set_tool_status_at(index, status);
                }
            }
            changed = true;
        }
        changed
    }

    fn any_pending_tool_blocks(&self) -> bool {
        self.transcript.iter().any(|e| {
            matches!(
                e,
                TranscriptEntry::Tool {
                    status: ToolBlockStatus::Pending,
                    ..
                } | TranscriptEntry::Diff {
                    status: ToolBlockStatus::Pending,
                    ..
                }
            )
        })
    }

    fn script_work_pending(&self) -> bool {
        !self.pending_events.is_empty()
            || self.active_stream_entry.is_some()
            || self.scheduled_actions.iter().any(|a| {
                matches!(
                    a.action,
                    TimedAction::Event(_)
                        | TimedAction::StreamStart(_)
                        | TimedAction::StreamChunk(_, _)
                        | TimedAction::StreamFinish(_)
                )
            })
    }

    /// After tool tint flips: keep Working while any block is Pending; else Ready if idle.
    fn sync_status_after_tools(&mut self) {
        if self.active_stream_entry.is_some() {
            return;
        }
        if self.any_pending_tool_blocks() {
            self.set_status("Working");
            return;
        }
        if !self.script_work_pending() {
            self.set_status("Ready");
        }
    }

    fn set_tool_status_at(&mut self, index: usize, status: ToolBlockStatus) {
        match self.transcript.get_mut(index) {
            Some(TranscriptEntry::Tool {
                status: slot,
                summary,
                ..
            })
            | Some(TranscriptEntry::Diff {
                status: slot,
                summary,
                ..
            }) => {
                *slot = status;
                if status == ToolBlockStatus::Success && summary.contains("· running") {
                    *summary = summary.replace("· running", "· ok");
                }
            }
            _ => {}
        }
        self.sync_status_after_tools();
    }

    fn advance_script(&mut self) {
        if let Some(event) = self.pending_events.pop_front() {
            self.apply_event(event);
            return;
        }

        let fallback = match self.scripted_turn {
            0 => Some(vec![
                ScriptEvent::Status("Running acceptance harness".into()),
                // Tight burst: both Pending at once, independent flips.
                ScriptEvent::Tool("cargo test -p xylitol-tui --test agent_demo_test".into()),
                ScriptEvent::Tool("cargo test --test tui_e2e -- --ignored".into()),
                ScriptEvent::MarkPlan(1),
                ScriptEvent::Assistant(
                    "agent_demo is now the single example surface; the old kitchen-sink demos are scheduled for removal.".into(),
                ),
                ScriptEvent::Status("Ready".into()),
            ]),
            1 => Some(vec![
                ScriptEvent::Tool("cargo check -p xylitol-tui".into()),
                ScriptEvent::MarkPlan(3),
                ScriptEvent::Assistant(
                    "Real terminal smoke should stay focused on the primary flow rather than keeping every showcase alive.".into(),
                ),
                ScriptEvent::Status("Ready".into()),
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
                let detail = "(exit 0 — demo stub)".to_string();
                let index = self.transcript.len();
                self.push_tool(
                    format!("{text} · running"),
                    detail,
                    ToolBlockStatus::Pending,
                );
                // Independent completion — wide jitter so multiple Pending overlap.
                let flip = self.jitter_ticks(10, 28);
                self.schedule_from_now(
                    flip,
                    TimedAction::SetToolStatus {
                        index,
                        status: ToolBlockStatus::Success,
                    },
                );
            }
            ScriptEvent::Edit { summary, input } => {
                self.set_status("Working");
                self.recent_tools.insert(0, summary.clone());
                self.recent_tools.truncate(4);
                let index = self.transcript.len();
                // pi Edit: unified compact Diff, expanded (pops open). Never SBS.
                self.push_diff_ex(
                    format!("{summary} · running"),
                    input,
                    None,
                    true,
                    ToolBlockStatus::Pending,
                );
                let flip = self.jitter_ticks(12, 32);
                self.schedule_from_now(
                    flip,
                    TimedAction::SetToolStatus {
                        index,
                        status: ToolBlockStatus::Success,
                    },
                );
                if !self.changed_files.iter().any(|p| p.contains("ui_root.rs")) {
                    self.changed_files
                        .push("src/app/tui/ui_root.rs".to_string());
                }
            }
            ScriptEvent::Assistant(text) => {
                self.push_message(Role::Assistant, text);
                self.sync_status_after_tools();
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
                let t = text;
                // Don't clobber Working while tool blocks are still Pending.
                if t == "Ready" && self.any_pending_tool_blocks() {
                    self.set_status("Working");
                } else {
                    self.set_status(t);
                }
            }
        }
    }

    fn role_prefix(&self, role: Role) -> String {
        match role {
            Role::User => magenta(self.glyph_set.user()),
            Role::Assistant => String::new(),
            Role::System => dim(self.glyph_set.system()),
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

    fn push_wrapped(lines: &mut Vec<String>, raw: &str, width: usize) {
        for line in wrap_text_with_ansi(raw, width) {
            lines.push(Self::fit(&line, width));
        }
    }

    fn transcript_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let g = self.glyph_set;
        for entry in &self.transcript {
            match entry {
                TranscriptEntry::Message { role, text } => {
                    if matches!(role, Role::Assistant) {
                        // Always Markdown so streaming fences get highlight as they close.
                        let mut md = Markdown::new(
                            text.clone(),
                            0,
                            0,
                            demo_markdown_theme(),
                            None,
                            Some(MarkdownOptions::default()),
                        );
                        for line in md.render(width) {
                            lines.push(Self::fit(&line, width));
                        }
                    } else {
                        let prefix = self.role_prefix(*role);
                        let raw = if prefix.is_empty() {
                            text.clone()
                        } else {
                            format!("{prefix} {text}")
                        };
                        Self::push_wrapped(&mut lines, &raw, width);
                    }
                }
                TranscriptEntry::Thinking { expanded, body } => {
                    let marker = if *expanded { g.unfold() } else { g.fold() };
                    let header = format!("{marker} thinking  {}", key_hint("Ctrl+T"));
                    Self::push_wrapped(&mut lines, &header, width);
                    if *expanded {
                        Self::push_wrapped(&mut lines, &dim(body), width);
                    }
                }
                TranscriptEntry::Tool {
                    expanded,
                    status,
                    summary,
                    detail,
                } => {
                    let marker = if *expanded { g.unfold() } else { g.fold() };
                    let header = format!("{marker} {} {summary}  {}", g.tool(), key_hint("Alt+E"));
                    let mut block = Vec::new();
                    Self::push_wrapped(&mut block, &header, width);
                    if *expanded {
                        Self::push_wrapped(&mut block, &dim(detail), width);
                    }
                    for line in block {
                        lines.push(paint_tool_bg(&line, width, *status));
                    }
                }
                TranscriptEntry::Diff {
                    expanded,
                    status,
                    summary,
                    input,
                    side_by_side_min_width,
                } => {
                    let marker = if *expanded { g.unfold() } else { g.fold() };
                    let header = format!("{marker} {} {summary}  {}", g.tool(), key_hint("Alt+E"));
                    // Status tint on header only — Diff body keeps its own fg/bg
                    // (painting tool-success-bg over red/green diff lines looks broken).
                    let mut header_lines = Vec::new();
                    Self::push_wrapped(&mut header_lines, &header, width);
                    for line in header_lines {
                        lines.push(paint_tool_bg(&line, width, *status));
                    }
                    if *expanded {
                        let theme = DiffTheme::default();
                        let opts = DiffOptions {
                            word_level: true,
                            side_by_side_min_width: *side_by_side_min_width,
                            ..DiffOptions::default()
                        };
                        let rendered = render_diff_lines(input, width, &theme, &opts);
                        for line in rendered {
                            lines.push(Self::fit(&line, width));
                        }
                    }
                }
            }
            lines.push(String::new());
        }
        lines
    }

    /// Busy-only status (DESIGN.md): idle returns None so the stack stays short.
    fn status_line(&mut self, width: usize) -> Option<String> {
        if !self.spinner_active() {
            return None;
        }
        let activity = self
            .loader
            .render(width)
            .into_iter()
            .find(|line| !line.is_empty())
            .unwrap_or_else(|| dim(&self.status_text));
        Some(Self::fit(&activity, width))
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
        // Minimal stack (DESIGN.md / pi): transcript → [status] → editor|selector → footer.
        let mut lines = Vec::new();
        lines.extend(self.transcript_lines(width));
        if let Some(status) = self.status_line(width) {
            lines.push(status);
        }
        lines.extend(self.render_editor_slot(width));
        let footer_owned;
        let footer_ref = if self.palette_open || self.settings_open {
            "esc close · ↑↓ · Enter"
        } else {
            // Compact cue strip — full list is in the seed system line.
            footer_owned = format!(
                // Keep cue strip short — narrow terminals (80 cols) still fit.
                "{} · {} · /@ (Ctrl+P)/(Ctrl+S) (Alt+G) (Ctrl+O)",
                self.footer_note,
                self.glyph_set.label()
            );
            footer_owned.as_str()
        };
        lines.push(Self::fit(&dim(footer_ref), width));
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
            self.on_ctrl_c();
            return;
        }

        if matches_key_event(key, "escape") && self.on_escape() {
            return;
        }
        // Fall through so Editor can dismiss slash CommandPopup (Esc).

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
                            self.push_diff_ex(
                                "workspace diff (+2 -2) unified",
                                sample_unified_pair(),
                                None,
                                true,
                                ToolBlockStatus::Success,
                            );
                            self.push_diff_ex(
                                "workspace diff side-by-side",
                                sample_sbs_pair(),
                                Some(60),
                                true,
                                ToolBlockStatus::Success,
                            );
                        }
                        "compact" => {
                            self.push_message(
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
        // App-level toggles (DESIGN expandable blocks / glyph config). Not bare
        // letters — those must stay available for typing in the editor.
        if matches_key_event(key, "ctrl+t") {
            self.toggle_thinking_blocks();
            return;
        }
        // Alt+E / Alt+G — not Ctrl+E (editor cursorLineEnd) or Ctrl+G (future
        // external editor). App-level toggles stay off the Editor keybinding table.
        if matches_key_event(key, "alt+e") {
            self.toggle_tool_blocks();
            return;
        }
        if matches_key_event(key, "alt+g") {
            self.cycle_glyph_set();
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
