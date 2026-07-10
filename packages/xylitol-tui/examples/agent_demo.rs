use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

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
    Component, CrosstermTerminal, DiffInput, DiffOptions, DiffTheme, ExpandableOutputOptions,
    Focusable, Input, InputEvent, InputListenerResult, Markdown, MarkdownOptions, MarkdownTheme,
    SystemClock, TUI, TreeNode, TreeSelector, TreeSelectorOptions, TreeSelectorTheme,
    apply_background_to_line, highlight_code, matches_key_event, render_diff_lines,
    render_expandable_output, truncate_to_width, visible_width, wrap_text_with_ansi,
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

fn sample_session_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::new("root", "session · demo").with_children([
            TreeNode::new("u1", "user: tighten footer truncation").with_child(
                TreeNode::new("a1", "assistant: plan + tools").with_children([
                    TreeNode::new("t1", "tool: rg -n TreeSelector"),
                    TreeNode::new("a2", "assistant: ship tree slot")
                        .with_annotation("ship")
                        .with_annotation_at("2d ago")
                        .with_child(TreeNode::new("u2", "user: also verify double Esc")),
                ]),
            ),
            TreeNode::new("fork", "user: alternate branch")
                .with_annotation("alt")
                .with_annotation_at("1h ago")
                .with_child(TreeNode::new("af", "assistant: (fork leaf)")),
        ]),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionTreeFilter {
    Default,
    NoTools,
    UserOnly,
    LabeledOnly,
    All,
}

impl SessionTreeFilter {
    const ALL: [Self; 5] = [
        Self::Default,
        Self::NoTools,
        Self::UserOnly,
        Self::LabeledOnly,
        Self::All,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "[default]",
            Self::NoTools => "[no-tools]",
            Self::UserOnly => "[user]",
            Self::LabeledOnly => "[labeled]",
            Self::All => "[all]",
        }
    }

    fn cycle(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn include(self, node: &TreeNode) -> bool {
        let label = node.label.as_str();
        match self {
            // demo default ≈ all (see c456 design.md)
            Self::Default | Self::All => true,
            Self::NoTools => !label.contains("tool:"),
            Self::UserOnly => label.contains("user:"),
            Self::LabeledOnly => node.annotation.is_some(),
        }
    }
}

fn demo_tree_selector(
    roots: Vec<TreeNode>,
    active_id: &str,
    filter: SessionTreeFilter,
) -> TreeSelector {
    TreeSelector::new(
        roots,
        TreeSelectorTheme::default(),
        TreeSelectorOptions {
            max_visible: 10,
            unicode_connectors: true,
            include_node: Some(Box::new(move |n| filter.include(n))),
            active_id: Some(active_id.into()),
            status_suffix: Some(filter.label().into()),
        },
    )
}

fn find_session_node_mut<'a>(roots: &'a mut [TreeNode], id: &str) -> Option<&'a mut TreeNode> {
    for root in roots {
        if root.id == id {
            return Some(root);
        }
        if let Some(n) = find_session_node_mut(&mut root.children, id) {
            return Some(n);
        }
    }
    None
}

fn find_session_node<'a>(roots: &'a [TreeNode], id: &str) -> Option<&'a TreeNode> {
    for root in roots {
        if root.id == id {
            return Some(root);
        }
        if let Some(n) = find_session_node(&root.children, id) {
            return Some(n);
        }
    }
    None
}

fn is_reply_tree_label(label: &str) -> bool {
    label.starts_with("assistant:") || label.starts_with("tool:")
}

fn tree_label_preview(prefix: &str, text: &str) -> String {
    let one = text.lines().next().unwrap_or(text).trim();
    let body = if visible_width(one) > 48 {
        truncate_to_width(one, 48, "…", false)
    } else {
        one.to_string()
    };
    format!("{prefix}{body}")
}

/// Root→target id path in a session tree (inclusive). Demo history travel uses this.
fn path_ids_to(roots: &[TreeNode], target: &str) -> Option<Vec<String>> {
    fn walk(node: &TreeNode, target: &str, path: &mut Vec<String>) -> bool {
        path.push(node.id.clone());
        if node.id == target {
            return true;
        }
        for child in &node.children {
            if walk(child, target, path) {
                return true;
            }
        }
        path.pop();
        false
    }
    let mut path = Vec::new();
    for root in roots {
        if walk(root, target, &mut path) {
            return Some(path);
        }
    }
    None
}

/// Path to `target`, then linear assistant/tool spine (so travel to a user still shows its reply).
fn travel_path_with_replies(roots: &[TreeNode], target: &str) -> Vec<String> {
    let mut path = path_ids_to(roots, target).unwrap_or_else(|| vec![target.to_string()]);
    let Some(start) = path.last().cloned() else {
        return path;
    };
    let mut cur_id = start;
    loop {
        let Some(node) = find_session_node(roots, &cur_id) else {
            break;
        };
        if node.children.len() != 1 {
            break;
        }
        let child = &node.children[0];
        if !is_reply_tree_label(&child.label) {
            break;
        }
        path.push(child.id.clone());
        cur_id = child.id.clone();
    }
    path
}

/// Built-in payloads for the seed sample tree (live nodes use `history_payloads`).
fn seed_history_entry(id: &str) -> Option<TranscriptEntry> {
    match id {
        "root" => None,
        "u1" => Some(TranscriptEntry::Message {
            role: Role::User,
            text: "tighten footer truncation".into(),
        }),
        "a1" => Some(TranscriptEntry::Message {
            role: Role::Assistant,
            text: "plan + tools — I'll search the tree selector and ship the editor slot.".into(),
        }),
        "t1" => Some(TranscriptEntry::Tool {
            expanded: true,
            status: ToolBlockStatus::Success,
            summary: "rg -n TreeSelector · ok".into(),
            detail: "packages/xylitol-tui/src/components/tree_selector.rs\n(demo history leaf)"
                .into(),
        }),
        "a2" => Some(TranscriptEntry::Message {
            role: Role::Assistant,
            text: "ship tree slot — double Esc replaces the editor; Enter travels here.".into(),
        }),
        "u2" => Some(TranscriptEntry::Message {
            role: Role::User,
            text: "also verify double Esc".into(),
        }),
        "fork" => Some(TranscriptEntry::Message {
            role: Role::User,
            text: "alternate branch".into(),
        }),
        "af" => Some(TranscriptEntry::Message {
            role: Role::Assistant,
            text: "(fork leaf) — history rebuild stops at this node.".into(),
        }),
        _ => None,
    }
}
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

/// Legacy display_diff gutter sample (still exercised). No `--- a/` / `+++ b/` — path is on the header.
fn sample_display_diff() -> String {
    [
        "  10    10 |     Component, CrosstermTerminal,",
        "  11       | -    Focusable, InputEvent,",
        "       11 | +    DiffInput, Focusable, InputEvent,",
    ]
    .join("\n")
}

/// Long tool stdout for expandable viewport (morphology only; content is filler).
fn sample_long_bash_output() -> String {
    let mut lines: Vec<String> = (1..=24)
        .map(|i| format!("(pass) suite-{i:02} · case ok"))
        .collect();
    lines.extend([
        "202 pass".into(),
        "0 fail".into(),
        "454 expect() calls".into(),
        "Ran 202 tests across 33 files. [9.26s]".into(),
        "Took 9.3s".into(),
    ]);
    lines.join("\n")
}

/// Prefer workspace-relative path when under `cwd`; otherwise absolute.
fn format_edit_path(path: impl AsRef<std::path::Path>, cwd: &std::path::Path) -> String {
    let path = path.as_ref();
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    match abs.strip_prefix(cwd) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.display().to_string(),
        _ => abs.display().to_string(),
    }
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

/// Mocha Diff theme (DESIGN.md): row tint + brighter word tint (not reverse white).
fn demo_diff_theme() -> DiffTheme {
    // fg
    const ADDED: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);
    const REMOVED: (u8, u8, u8) = (0xf3, 0x8b, 0xa8);
    const CONTEXT: (u8, u8, u8) = (0x6c, 0x70, 0x86);
    // row bg
    const ADDED_BG: (u8, u8, u8) = (0x1e, 0x2b, 0x22);
    const REMOVED_BG: (u8, u8, u8) = (0x2b, 0x1e, 0x24);
    // word bg (stronger)
    const ADDED_WORD: (u8, u8, u8) = (0x2d, 0x4a, 0x35);
    const REMOVED_WORD: (u8, u8, u8) = (0x4a, 0x2d, 0x35);

    let fg = |rgb: (u8, u8, u8)| {
        move |s: &str| format!("\x1b[38;2;{};{};{}m{s}\x1b[39m", rgb.0, rgb.1, rgb.2)
    };
    let line_bg = |rgb: (u8, u8, u8)| {
        move |s: &str| format!("\x1b[48;2;{};{};{}m{s}\x1b[49m", rgb.0, rgb.1, rgb.2)
    };
    // Word tint restores row bg (not 49m) so the line wash stays continuous.
    let word = |fg_rgb: (u8, u8, u8), word_bg: (u8, u8, u8), row_bg: (u8, u8, u8)| {
        move |s: &str| {
            format!(
                "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{s}\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
                word_bg.0,
                word_bg.1,
                word_bg.2,
                fg_rgb.0,
                fg_rgb.1,
                fg_rgb.2,
                fg_rgb.0,
                fg_rgb.1,
                fg_rgb.2,
                row_bg.0,
                row_bg.1,
                row_bg.2,
            )
        }
    };

    DiffTheme {
        added: Box::new(fg(ADDED)),
        removed: Box::new(fg(REMOVED)),
        context: Box::new(fg(CONTEXT)),
        gutter: Box::new(fg(CONTEXT)),
        meta: Box::new(fg(CONTEXT)),
        word_change_added: Box::new(word(ADDED, ADDED_WORD, ADDED_BG)),
        word_change_removed: Box::new(word(REMOVED, REMOVED_WORD, REMOVED_BG)),
        added_line_bg: Box::new(line_bg(ADDED_BG)),
        removed_line_bg: Box::new(line_bg(REMOVED_BG)),
        highlight_line: Box::new(|s| s.to_string()),
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

#[derive(Clone)]
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
    /// Long bash stdout streamed into Tool detail (expandable viewport demo).
    StreamingBash {
        summary: String,
        lines: Vec<String>,
    },
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
    /// Append to a Tool entry's detail (streaming bash/tool output; viewport sticks to tail).
    AppendToolDetail {
        index: usize,
        chunk: String,
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
    /// Double-Esc session tree (c454/c456).
    tree_open: bool,
    tree: TreeSelector,
    tree_filter: SessionTreeFilter,
    /// When set, tree slot shows annotation editor instead of browse chrome.
    tree_label_edit: Option<(String, Input)>,
    /// Live session graph (starts as sample; grows on submit / assistant finish).
    session_tree: Vec<TreeNode>,
    /// Payloads for live nodes (seed ids use [`seed_history_entry`]).
    history_payloads: HashMap<String, TranscriptEntry>,
    /// Monotonic id suffix for live tree nodes (`live-u-1`, `live-a-2`, …).
    next_node_seq: u64,
    /// Current history leaf (tree `active_id` / Enter travel target). Demo only.
    history_leaf_id: String,
    /// Steer: Enter while busy — applied when the current turn finishes (demo).
    steer_queue: VecDeque<String>,
    /// Follow-up: Alt+Enter — applied only when fully idle.
    follow_up_queue: VecDeque<String>,
    last_esc_at: Option<Instant>,
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
    /// Workspace root for Edit path display (`format_edit_path`) and `@` completion.
    cwd: std::path::PathBuf,
    /// Global tool-output viewport (pi `toolsExpanded` / Ctrl+O). Collapsed = last N
    /// visual lines + `... (N earlier lines, ctrl+o to expand)`; expanded = full detail.
    tools_output_expanded: bool,
    /// Max visual lines when collapsed (pi bash tool = 5).
    tools_output_max_lines: usize,
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

    /// Harness: transcript length (index of next push).
    pub fn transcript_len_for_test(&self) -> usize {
        self.transcript.len()
    }

    /// Harness: global tool-output viewport expand (Ctrl+O).
    pub fn tools_output_expanded_for_test(&self) -> bool {
        self.tools_output_expanded
    }

    pub fn set_tools_output_expanded_for_test(&mut self, expanded: bool) {
        self.tools_output_expanded = expanded;
    }

    /// Harness: append to a Tool detail (streaming viewport).
    pub fn append_tool_detail_for_test(&mut self, index: usize, chunk: impl Into<String>) {
        self.append_tool_detail_at(index, &chunk.into());
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

    /// Esc: close overlays/tree; abort stream; double-Esc (empty editor) opens tree.
    /// Returns true if the event was consumed.
    pub fn on_escape(&mut self) -> bool {
        if self.tree_open {
            if self.tree_label_edit.take().is_some() {
                return true;
            }
            if self.tree.clear_search_if_any() {
                return true;
            }
            self.close_session_tree();
            return true;
        }
        if self.palette_open || self.settings_open {
            self.palette_open = false;
            self.settings_open = false;
            return true;
        }
        if self.active_stream_entry.is_some() || !self.scheduled_actions.is_empty() {
            self.abort_active_stream();
            return true;
        }
        if self.input.get_text().is_empty() {
            let now = Instant::now();
            if let Some(prev) = self.last_esc_at
                && now.duration_since(prev) < Duration::from_millis(500)
            {
                self.last_esc_at = None;
                self.open_session_tree();
                return true;
            }
            self.last_esc_at = Some(now);
        } else {
            self.last_esc_at = None;
        }
        false
    }

    pub fn open_session_tree(&mut self) {
        self.palette_open = false;
        self.settings_open = false;
        self.tree_filter = SessionTreeFilter::Default;
        self.tree_label_edit = None;
        self.tree = demo_tree_selector(
            self.session_tree.clone(),
            &self.history_leaf_id,
            self.tree_filter,
        );
        self.tree_open = true;
        self.set_status("Session tree");
    }

    pub fn close_session_tree(&mut self) {
        self.tree_open = false;
        self.tree_label_edit = None;
        self.set_status("Ready");
    }

    fn history_entry_for(&self, id: &str) -> Option<TranscriptEntry> {
        self.history_payloads
            .get(id)
            .cloned()
            .or_else(|| seed_history_entry(id))
    }

    /// Append a child under the current history leaf and advance the leaf.
    fn grow_session_tree(&mut self, id: String, label: String, entry: TranscriptEntry) {
        let parent = self.history_leaf_id.clone();
        if let Some(node) = find_session_node_mut(&mut self.session_tree, &parent) {
            node.children.push(TreeNode::new(id.clone(), label));
        } else if let Some(root) = self.session_tree.first_mut() {
            root.children.push(TreeNode::new(id.clone(), label));
        } else {
            self.session_tree.push(TreeNode::new(id.clone(), label));
        }
        self.history_payloads.insert(id.clone(), entry);
        self.history_leaf_id = id;
    }

    fn alloc_node_id(&mut self, kind: &str) -> String {
        self.next_node_seq += 1;
        format!("live-{kind}-{}", self.next_node_seq)
    }

    /// Enter on session tree: rebuild transcript along root→id (+ linear reply spine).
    pub fn travel_to_history(&mut self, id: &str) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.active_stream_entry = None;
        self.steer_queue.clear();
        // Keep follow-ups — they are for after idle, independent of travel.

        let path = travel_path_with_replies(&self.session_tree, id);
        let path_label = path.join(" → ");
        let leaf = path.last().cloned().unwrap_or_else(|| id.to_string());

        self.transcript.clear();
        self.push_message(Role::System, format!("history @ {id} · path: {path_label}"));
        for node_id in &path {
            if let Some(entry) = self.history_entry_for(node_id) {
                self.transcript.push(entry);
            }
        }

        self.history_leaf_id = leaf;
        self.close_session_tree(); // Ready — banner lives in transcript, not a spinning status
    }

    /// Harness: submit text as if the editor fired on_submit (bypasses paste-burst).
    pub fn submit_text_for_test(&mut self, text: impl Into<String>) {
        self.process_submit(text.into());
    }

    /// Harness: drive one Component tick (script / streams / queues).
    pub fn tick_for_test(&mut self) -> bool {
        self.tick()
    }

    pub fn history_leaf_for_test(&self) -> &str {
        &self.history_leaf_id
    }

    pub fn steer_queue_len_for_test(&self) -> usize {
        self.steer_queue.len()
    }

    pub fn follow_up_queue_len_for_test(&self) -> usize {
        self.follow_up_queue.len()
    }

    pub fn enqueue_follow_up_for_test(&mut self, text: impl Into<String>) {
        self.enqueue_follow_up(text.into());
    }

    pub fn travel_to_history_for_test(&mut self, id: &str) {
        self.travel_to_history(id);
    }

    /// Whether a label substring appears anywhere in the live session tree (harness).
    pub fn session_tree_contains_label_for_test(&self, needle: &str) -> bool {
        fn walk(nodes: &[TreeNode], needle: &str) -> bool {
            nodes
                .iter()
                .any(|n| n.label.contains(needle) || walk(&n.children, needle))
        }
        walk(&self.session_tree, needle)
    }

    /// Flattened plain text from transcript messages/tool summaries (harness).
    pub fn transcript_plain_for_test(&self) -> String {
        let mut out = String::new();
        for entry in &self.transcript {
            match entry {
                TranscriptEntry::Message { text, .. } => {
                    out.push_str(text);
                    out.push('\n');
                }
                TranscriptEntry::Thinking { body, .. } => {
                    out.push_str(body);
                    out.push('\n');
                }
                TranscriptEntry::Tool {
                    summary, detail, ..
                } => {
                    out.push_str(summary);
                    out.push('\n');
                    out.push_str(detail);
                    out.push('\n');
                }
                TranscriptEntry::Diff { summary, .. } => {
                    out.push_str(summary);
                    out.push('\n');
                }
            }
        }
        out
    }

    fn begin_tree_label_edit(&mut self) {
        let Some(id) = self.tree.selected_id().map(str::to_string) else {
            return;
        };
        let current = self.tree.annotation_of(&id).unwrap_or("").to_string();
        let mut input = Input::new();
        input.set_value(current);
        self.tree_label_edit = Some((id, input));
    }

    fn commit_tree_label_edit(&mut self) {
        let Some((id, input)) = self.tree_label_edit.take() else {
            return;
        };
        let text = input.value().trim().to_string();
        let ann = if text.is_empty() { None } else { Some(text) };
        self.tree.set_annotation(&id, ann.clone());
        if ann.is_some() {
            self.tree.set_annotation_at(&id, Some("just now".into()));
        } else {
            self.tree.set_annotation_at(&id, None);
        }
    }

    fn apply_tree_filter(&mut self, filter: SessionTreeFilter) {
        self.tree_filter = filter;
        self.tree
            .set_include_node(Some(Box::new(move |n| filter.include(n))));
        self.tree.set_status_suffix(Some(filter.label().into()));
    }

    fn cycle_tree_filter(&mut self) {
        self.apply_tree_filter(self.tree_filter.cycle());
    }

    pub fn tree_open_for_test(&self) -> bool {
        self.tree_open
    }

    pub fn tree_fold_selected_for_test(&mut self) {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        self.tree.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::CONTROL,
        )));
    }

    pub fn tree_is_folded_for_test(&self, id: &str) -> bool {
        self.tree.is_folded(id)
    }

    pub fn tree_select_id_for_test(&mut self, id: &str) {
        if let Some(idx) = self.tree.filtered_nodes().iter().position(|n| n.id == id) {
            // Move selection by repeated down/up from 0
            while self.tree.selected_id() != Some(id) {
                let cur = self.tree.selected_id().unwrap_or("");
                let cur_i = self
                    .tree
                    .filtered_nodes()
                    .iter()
                    .position(|n| n.id == cur)
                    .unwrap_or(0);
                if cur_i < idx {
                    self.tree
                        .handle_input(InputEvent::Key(crossterm::event::KeyEvent::new(
                            crossterm::event::KeyCode::Down,
                            crossterm::event::KeyModifiers::NONE,
                        )));
                } else if cur_i > idx {
                    self.tree
                        .handle_input(InputEvent::Key(crossterm::event::KeyEvent::new(
                            crossterm::event::KeyCode::Up,
                            crossterm::event::KeyModifiers::NONE,
                        )));
                } else {
                    break;
                }
            }
        }
    }

    /// Harness: clear editor then open tree (skips double-Esc timing).
    pub fn open_session_tree_for_test(&mut self) {
        self.input.set_text(String::new());
        self.open_session_tree();
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
            Box::new(AtPathSource::new(cwd.clone())),
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
            tree_open: false,
            tree: demo_tree_selector(sample_session_tree(), "u2", SessionTreeFilter::Default),
            tree_filter: SessionTreeFilter::Default,
            tree_label_edit: None,
            session_tree: sample_session_tree(),
            history_payloads: HashMap::new(),
            next_node_seq: 0,
            history_leaf_id: "u2".into(),
            steer_queue: VecDeque::new(),
            follow_up_queue: VecDeque::new(),
            last_esc_at: None,
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
            cwd,
            tools_output_expanded: false,
            tools_output_max_lines: 5,
        };
        app.seed_transcript();
        app
    }

    fn seed_transcript(&mut self) {
        // One-shot help — fold keys live on blocks as `(Ctrl+T)` / `(Alt+E)`.
        self.push_message(
            Role::System,
            "keys: Enter submit/steer · Alt+Enter follow-up · double Esc tree · Enter travel (+reply) · /cmds · @path · (Ctrl+P)/(Ctrl+S) · (Alt+G) · (Ctrl+O tools) · Esc · (Ctrl+C)",
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
        // Long bash-style output: collapsed viewport shows last N + ctrl+o hint (pi).
        self.push_tool(
            "$ bun test (timeout 120s) · ok",
            sample_long_bash_output(),
            ToolBlockStatus::Success,
        );
        // Seed blocks start expanded so SBS / edit / gutter are visible without Alt+E.
        let demo_rs = format_edit_path("packages/xylitol-tui/examples/agent_demo.rs", &self.cwd);
        let ui_root = format_edit_path("src/app/tui/ui_root.rs", &self.cwd);
        self.push_diff_ex(
            format!("edited {demo_rs} (+2 -2) unified edit-format"),
            sample_unified_pair(),
            None, // always unified — Edit tool path
            true,
            ToolBlockStatus::Success,
        );
        self.push_diff_ex(
            format!("edited {ui_root} (+2 -2) side-by-side (optional)"),
            sample_sbs_pair(),
            Some(60),
            true,
            ToolBlockStatus::Success,
        );
        self.push_diff_ex(
            format!("edited {demo_rs} (display_diff gutter)"),
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

        // Busy Enter = steer (do not abort the in-flight turn).
        if self.is_turn_busy() {
            self.enqueue_steer(trimmed);
            return;
        }

        self.commit_user_turn(trimmed);
    }

    /// Alt+Enter: queue until idle, or submit immediately when idle.
    fn process_follow_up(&mut self, text: String) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if self.is_turn_busy() {
            self.enqueue_follow_up(trimmed);
            return;
        }
        self.commit_user_turn(trimmed);
    }

    fn is_turn_busy(&self) -> bool {
        self.script_work_pending()
            || self.any_pending_tool_blocks()
            || matches!(
                self.status_text.as_str(),
                "Working" | "Thinking" | "Drafting reply" | "Running tools"
            )
    }

    fn enqueue_steer(&mut self, text: String) {
        self.input.set_text(String::new());
        self.steer_queue.push_back(text.clone());
        self.push_message(
            Role::System,
            format!("steer queued ({}) · {text}", self.steer_queue.len()),
        );
        // Grow tree now so the steer is visible in the session graph without aborting tools.
        let id = self.alloc_node_id("u");
        self.grow_session_tree(
            id,
            tree_label_preview("user: ", &format!("[steer] {text}")),
            TranscriptEntry::Message {
                role: Role::User,
                text: format!("[steer] {text}"),
            },
        );
        self.push_message(Role::User, format!("[steer] {text}"));
    }

    fn enqueue_follow_up(&mut self, text: String) {
        self.input.set_text(String::new());
        self.follow_up_queue.push_back(text.clone());
        self.push_message(
            Role::System,
            format!("follow-up queued ({}) · {text}", self.follow_up_queue.len()),
        );
    }

    fn commit_user_turn(&mut self, trimmed: String) {
        self.last_submitted = trimmed.clone();
        self.push_message(Role::User, trimmed.clone());
        let user_id = self.alloc_node_id("u");
        self.grow_session_tree(
            user_id,
            tree_label_preview("user: ", &trimmed),
            TranscriptEntry::Message {
                role: Role::User,
                text: trimmed.clone(),
            },
        );
        self.input.set_text(String::new());
        self.auto_started = true;
        self.scripted_turn = self.scripted_turn.max(2);
        self.queue_simulated_turn(&trimmed);
    }

    /// After a turn goes idle: drain steer first, then one follow-up.
    fn drain_message_queues(&mut self) {
        if self.is_turn_busy() {
            return;
        }
        if let Some(text) = self.steer_queue.pop_front() {
            self.push_message(
                Role::System,
                format!("steer apply · {} remaining", self.steer_queue.len()),
            );
            // Steer node already grown at enqueue time — just run the turn from current leaf.
            self.last_submitted = text.clone();
            self.auto_started = true;
            self.scripted_turn = self.scripted_turn.max(2);
            self.queue_simulated_turn(&text);
            return;
        }
        if let Some(text) = self.follow_up_queue.pop_front() {
            self.push_message(
                Role::System,
                format!("follow-up apply · {} remaining", self.follow_up_queue.len()),
            );
            self.commit_user_turn(text);
        }
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status_text = text.into();
        self.loader.set_message(self.status_text.clone());
    }

    fn spinner_active(&self) -> bool {
        // Only agent-busy labels spin. Informational statuses (Session tree, history @ …)
        // must not keep the loader alive after the work is done.
        let busy_label = matches!(
            self.status_text.as_str(),
            "Working" | "Thinking" | "Drafting reply" | "Running tools"
        );
        busy_label
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
                summary: format!(
                    "edit {} (+1 -1)",
                    format_edit_path("src/app/tui/ui_root.rs", &self.cwd)
                ),
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
        if kind == StreamKind::Assistant
            && let Some(index) = self.active_stream_entry
            && let Some(TranscriptEntry::Message {
                role: Role::Assistant,
                text,
            }) = self.transcript.get(index)
        {
            let text = text.clone();
            if !text.trim().is_empty() {
                let id = self.alloc_node_id("a");
                self.grow_session_tree(
                    id,
                    tree_label_preview("assistant: ", &text),
                    TranscriptEntry::Message {
                        role: Role::Assistant,
                        text,
                    },
                );
            }
        }
        self.active_stream_entry = None;
        if kind == StreamKind::Assistant {
            self.set_status("Ready");
            self.drain_message_queues();
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
                TimedAction::AppendToolDetail { index, chunk } => {
                    self.append_tool_detail_at(index, &chunk);
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
                        | TimedAction::AppendToolDetail { .. }
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
            self.drain_message_queues();
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

    fn append_tool_detail_at(&mut self, index: usize, chunk: &str) {
        if let Some(TranscriptEntry::Tool { detail, .. }) = self.transcript.get_mut(index) {
            detail.push_str(chunk);
        }
    }

    fn toggle_tools_output_expanded(&mut self) {
        self.tools_output_expanded = !self.tools_output_expanded;
    }

    /// Schedule a streaming bash tool: detail grows line-by-line (collapsed viewport sticks to tail).
    fn push_streaming_bash_tool(&mut self, summary: impl Into<String>, lines: &[String]) {
        self.set_status("Working");
        let summary = summary.into();
        self.recent_tools.insert(0, summary.clone());
        self.recent_tools.truncate(4);
        let index = self.transcript.len();
        self.push_tool(
            format!("{summary} · running"),
            String::new(),
            ToolBlockStatus::Pending,
        );
        let mut delay = 2u64;
        for line in lines {
            self.schedule_from_now(
                delay,
                TimedAction::AppendToolDetail {
                    index,
                    chunk: format!("{line}\n"),
                },
            );
            delay = delay.saturating_add(1);
        }
        self.schedule_from_now(
            delay.saturating_add(2),
            TimedAction::SetToolStatus {
                index,
                status: ToolBlockStatus::Success,
            },
        );
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
                ScriptEvent::StreamingBash {
                    summary: "$ git commit --dry-run (stream)".into(),
                    lines: (1..=18)
                        .map(|i| format!("check step-{i:02}........................Passed"))
                        .chain([
                            "Command exited with code 0".into(),
                            "Took 1.8s".into(),
                        ])
                        .collect(),
                },
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
                    detail.clone(),
                    ToolBlockStatus::Pending,
                );
                let tool_id = self.alloc_node_id("t");
                self.grow_session_tree(
                    tool_id,
                    tree_label_preview("tool: ", &text),
                    TranscriptEntry::Tool {
                        expanded: true,
                        status: ToolBlockStatus::Success,
                        summary: format!("{text} · ok"),
                        detail,
                    },
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
            ScriptEvent::StreamingBash { summary, lines } => {
                self.push_streaming_bash_tool(summary.clone(), &lines);
                let detail = lines.join("\n");
                let tool_id = self.alloc_node_id("t");
                self.grow_session_tree(
                    tool_id,
                    tree_label_preview("tool: ", &summary),
                    TranscriptEntry::Tool {
                        expanded: false,
                        status: ToolBlockStatus::Success,
                        summary: summary.clone(),
                        detail,
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
                    input.clone(),
                    None,
                    true,
                    ToolBlockStatus::Pending,
                );
                let tool_id = self.alloc_node_id("t");
                self.grow_session_tree(
                    tool_id,
                    tree_label_preview("tool: ", &summary),
                    TranscriptEntry::Diff {
                        expanded: true,
                        status: ToolBlockStatus::Success,
                        summary: format!("{summary} · ok"),
                        input,
                        side_by_side_min_width: None,
                    },
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
                self.push_message(Role::Assistant, text.clone());
                if !text.trim().is_empty() {
                    let id = self.alloc_node_id("a");
                    self.grow_session_tree(
                        id,
                        tree_label_preview("assistant: ", &text),
                        TranscriptEntry::Message {
                            role: Role::Assistant,
                            text,
                        },
                    );
                }
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
                        let opts = ExpandableOutputOptions {
                            max_preview_lines: self.tools_output_max_lines,
                            expand_hint: "ctrl+o to expand".into(),
                            hint_style: Some(dim),
                            ..ExpandableOutputOptions::default()
                        };
                        for line in render_expandable_output(
                            detail,
                            width,
                            self.tools_output_expanded,
                            &opts,
                        ) {
                            block.push(Self::fit(&line, width));
                        }
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
                        let theme = demo_diff_theme();
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
        if self.tree_open {
            return self.render_tree_slot(width);
        }
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

    fn render_tree_slot(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(Self::fit(&bold(" Session tree"), width));
        if let Some((_, ref mut input)) = self.tree_label_edit {
            lines.push(Self::fit(
                &dim(" Label edit · Enter save · Esc cancel"),
                width,
            ));
            for line in input.render(width) {
                lines.push(Self::fit(&line, width));
            }
            return lines;
        }
        let search = self.tree.search_query();
        let search_line = if search.is_empty() {
            dim(" Type search · ←→ page · Ctrl/Alt+←→ fold · Shift+L label · Shift+T time")
        } else {
            dim(&format!(" Search: {search}"))
        };
        lines.push(Self::fit(&search_line, width));
        lines.push(Self::fit(
            &dim(" Up/Down  Enter travel  Esc close/clear  (double Esc)  Ctrl+D/T/U/L/A filter"),
            width,
        ));
        for line in self.tree.render(width) {
            lines.push(Self::fit(&line, width));
        }
        lines
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
        let footer_ref = if self.palette_open || self.settings_open || self.tree_open {
            "esc close · ↑↓ · Enter"
        } else {
            let queue_hint = match (self.steer_queue.len(), self.follow_up_queue.len()) {
                (0, 0) => String::new(),
                (s, 0) => format!(" · steer:{s}"),
                (0, f) => format!(" · follow-up:{f}"),
                (s, f) => format!(" · steer:{s} follow-up:{f}"),
            };
            // Compact cue strip — full list is in the seed system line.
            footer_owned = format!(
                // Keep cue strip short — narrow terminals (80 cols) still fit.
                "{} · {}{queue_hint} · /@ (Ctrl+P)/(Ctrl+S) (Alt+G) (Ctrl+O tools)",
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

        if matches_key_event(key, "alt+enter") {
            let text = self.input.get_text();
            self.process_follow_up(text);
            return;
        }

        if self.tree_open {
            if self.tree_label_edit.is_some() {
                if matches_key_event(key, "enter") {
                    self.commit_tree_label_edit();
                    return;
                }
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    input.handle_input(event);
                }
                return;
            }
            if matches_key_event(key, "ctrl+d") {
                self.apply_tree_filter(SessionTreeFilter::Default);
                return;
            }
            if matches_key_event(key, "ctrl+t") {
                self.apply_tree_filter(SessionTreeFilter::NoTools);
                return;
            }
            if matches_key_event(key, "ctrl+u") {
                self.apply_tree_filter(SessionTreeFilter::UserOnly);
                return;
            }
            if matches_key_event(key, "ctrl+l") {
                self.apply_tree_filter(SessionTreeFilter::LabeledOnly);
                return;
            }
            if matches_key_event(key, "ctrl+a") {
                self.apply_tree_filter(SessionTreeFilter::All);
                return;
            }
            if matches_key_event(key, "ctrl+o") {
                self.cycle_tree_filter();
                return;
            }
            if matches_key_event(key, "shift+l") {
                self.begin_tree_label_edit();
                return;
            }
            if matches_key_event(key, "shift+t") {
                self.tree.toggle_annotation_timestamps();
                return;
            }
            if matches_key_event(key, "enter") {
                let id = self.tree.selected_id().unwrap_or("?").to_string();
                self.travel_to_history(&id);
                return;
            }
            self.tree.handle_input(event);
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
            // Tree-open path handled above; here tools viewport expand (pi).
            self.toggle_tools_output_expanded();
            return;
        }
        if matches_key_event(key, "ctrl+shift+o") {
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
        if !self.is_turn_busy() {
            self.drain_message_queues();
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
