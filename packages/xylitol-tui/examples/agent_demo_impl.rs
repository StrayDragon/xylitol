use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::IsTerminal;
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use xylitol_tui::autocomplete::{AutocompleteItem, AutocompleteSuggestions, SlashCommand};
use xylitol_tui::completion::{
    AtPathSource, CompletionContext, CompletionMatch, CompletionSource, SlashArgCompletionSource,
    SlashCommandSource,
};
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
    CancellableLoader, ChoiceMode, ChoiceOption, ChoicePrompt, ChoicePromptTheme, ChoiceQuestion,
    ChoiceResult, Component, CrosstermTerminal, DiffInput, DiffOptions, DiffTheme,
    ExpandableOutputOptions, Focusable, Input, InputEvent, InputListenerResult, InteractionMode,
    Markdown, MarkdownTheme, Palette, Panel, SystemClock, TUI, Terminal, TerminalColorScheme, Text,
    ThemeDetectSources, ThinkingBorderLevel, TreeNode, TreeSelector, TreeSelectorOptions,
    TreeSelectorTheme, TruncateFrom, TruncatedText, apply_background_to_line,
    apply_thinking_border, bg_rgb, fg_bg_rgb, fg_rgb, matches_key_event, mix_rgb,
    paint_left_rail_line, printable_from_key_event, render_diff_lines, render_expandable_output,
    resolve_terminal_color_scheme, truncate_to_width, visible_width, word_wash_bg,
    wrap_text_with_ansi,
};
#[cfg(test)]
use xylitol_tui::{
    is_osc11_background_color_response, is_terminal_color_reply, parse_osc11_background_color,
    parse_terminal_color_scheme_report,
};

/// Demo slash commands (static; product would load from Driver / protocol).
/// Names omit the leading `/` — Editor's CombinedAutocompleteProvider adds it.
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("help", "Show key help in transcript"),
    ("md", "Stream full Markdown grammar stub (typewriter)"),
    ("theme", "Switch chrome theme: /theme [dark|light|toggle]"),
    (
        "entry-style",
        "Entry paint: /entry-style [rail|wash|toggle] (tools + Ask; product Ask = rail only)",
    ),
    (
        "thinking-level",
        "Cycle editor thinking border (Shift+Tab; or /thinking-level)",
    ),
    ("model", "Switch model: /model <id>"),
    ("compact", "Demo Compacting status → Working (c493)"),
    ("retry", "Demo Retry status → Working (c493)"),
    ("export", "Export current session"),
    ("session", "Session management"),
    ("settings", "Open settings panel"),
    ("palette", "Open command plate (Ctrl+P)"),
    ("diff", "Inject unified + side-by-side diffs"),
];

/// Demo-only `$` CompletionSource (c545) — not product skill semantics.
const DEMO_DOLLAR_SKILLS: &[(&str, &str)] = &[
    ("demo", "c545 stub skill — third CompletionSource"),
    (
        "narrow-clamp-skill-with-a-very-long-identifier",
        "proves popup clamps under narrow width",
    ),
];

/// Stub SKILL.md bodies keyed by demo skill name (A10 inject assert — not product IO).
fn demo_skill_md_body(name: &str) -> Option<&'static str> {
    match name {
        "demo" => Some("# demo\n\nStub SKILL.md body for agent_demo inject assert."),
        "narrow-clamp-skill-with-a-very-long-identifier" => {
            Some("# narrow-clamp\n\nLong-id stub SKILL.md body.")
        }
        _ => None,
    }
}

/// Collect `$name` tokens (name = `[A-Za-z0-9_-]+`) in left-to-right order.
fn dollar_skill_names(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
            {
                end += 1;
            }
            if end > start {
                out.push(text[start..end].to_string());
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Paint `$name` with `skill_ref` (bold); leave other text unstyled (A10).
fn highlight_dollar_skill_refs(text: &str, skill_ref: xylitol_tui::RgbColor) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
            {
                end += 1;
            }
            if end > start {
                let token = &text[i..end];
                out.push_str(&bold(&fg_rgb(skill_ref, token)));
                i = end;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Resolve known demo `$` refs → (name, stub SKILL.md body). Unknown `$` skipped.
fn resolve_demo_skill_injections(text: &str) -> Vec<(String, String)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for name in dollar_skill_names(text) {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(body) = demo_skill_md_body(&name) {
            out.push((name, body.to_string()));
        }
    }
    out
}

struct DemoDollarSource;

impl CompletionSource for DemoDollarSource {
    fn id(&self) -> &'static str {
        "demo-dollar"
    }

    fn probe(&self, ctx: &CompletionContext<'_>) -> Option<CompletionMatch> {
        xylitol_tui::extract_dollar_prefix(ctx.before_cursor())
            .map(|prefix| CompletionMatch { prefix })
    }

    fn should_dismiss(&self, ctx: &CompletionContext<'_>, _m: &CompletionMatch) -> bool {
        // Like `@`: bare `$` stays open; dismiss only when the `$…` token is gone.
        xylitol_tui::extract_dollar_prefix(ctx.before_cursor()).is_none()
    }

    fn suggestions(
        &self,
        _ctx: &CompletionContext<'_>,
        m: &CompletionMatch,
    ) -> Option<AutocompleteSuggestions> {
        let needle = m.prefix.strip_prefix('$').unwrap_or("");
        let items: Vec<AutocompleteItem> = DEMO_DOLLAR_SKILLS
            .iter()
            .filter(|(name, _)| name.starts_with(needle))
            .map(|(name, desc)| AutocompleteItem {
                value: format!("${name}"),
                label: (*name).to_string(),
                description: Some((*desc).to_string()),
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
        // Inline replace like `@path`: keep text before the `$…` token.
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

/// Command-plate row (c535): id drives routing; label/description feed SelectList.
#[derive(Debug, Clone, Copy)]
struct DemoPlateItem {
    id: &'static str,
    label: &'static str,
    description: &'static str,
}

/// Demo catalog — Ctrl+P / `/palette` list is generated from this table only.
const DEMO_PLATE: &[DemoPlateItem] = &[
    DemoPlateItem {
        id: "md-full",
        label: "Markdown full grammar (stream)",
        description: "Typewriter-stream every c530 display case (/md)",
    },
    DemoPlateItem {
        id: "stream-rust",
        label: "Stream Rust highlight",
        description: "Scripted turn with streamed Rust fence",
    },
    DemoPlateItem {
        id: "stream-python",
        label: "Stream Python highlight",
        description: "Scripted turn with streamed Python fence",
    },
    DemoPlateItem {
        id: "stream-typescript",
        label: "Stream TypeScript highlight",
        description: "Scripted turn with streamed TypeScript fence",
    },
    DemoPlateItem {
        id: "stream-json",
        label: "Stream JSON highlight",
        description: "Scripted turn with streamed JSON fence",
    },
    DemoPlateItem {
        id: "diff-sbs",
        label: "Diff unified + side-by-side",
        description: "CJK/empty-half edges + unified/SBS/edit (c540)",
    },
    DemoPlateItem {
        id: "completion-dollar",
        label: "Completion $ stub (c545)",
        description: "Inline $skill like @path — type use $ in editor",
    },
    DemoPlateItem {
        id: "expandable-head",
        label: "Expandable Head viewport (c550)",
        description: "Read-style tool: first-N + more-lines hint below",
    },
    DemoPlateItem {
        id: "playground-sync",
        label: "DESIGN playground sync (c555)",
        description: "Tip: sync_tokens.py + MD slot aligns with /md",
    },
    DemoPlateItem {
        id: "md-list-wrap",
        label: "Markdown list wrap (prewrapped)",
        description: "Nested lists + hanging indent; single wrap pass",
    },
    DemoPlateItem {
        id: "narrow-clamp",
        label: "Narrow width clamp (widgets)",
        description: "Settings/Input/Loader empty+narrow — library reference",
    },
    DemoPlateItem {
        id: "truncated-text",
        label: "TruncatedText atom",
        description: "Single-line ellipsis + padding — library reference",
    },
    DemoPlateItem {
        id: "cancellable-loader",
        label: "CancellableLoader atom",
        description: "Esc aborts spinner — library reference",
    },
    DemoPlateItem {
        id: "panel",
        label: "Panel atom (Box)",
        description: "Padding + background around children — library reference",
    },
    DemoPlateItem {
        id: "ask-single",
        label: "Ask · 1题单选",
        description: "ask wrapper face · Single + Other · Esc skip",
    },
    DemoPlateItem {
        id: "ask-multi",
        label: "Ask · 1题多选",
        description: "ask wrapper face · Multi + Other · Esc skip",
    },
    DemoPlateItem {
        id: "ask-tabs",
        label: "Ask · Tabs+Review",
        description: "≥2 questions · Enter advance · ←→ edit · Review",
    },
    DemoPlateItem {
        id: "ask-tool",
        label: "Ask · fake tool call",
        description: "Simulate builtin ask → ChoicePrompt → tool JSON result",
    },
    DemoPlateItem {
        id: "tool-tints",
        label: "Tool status tints",
        description: "Success / error / long bash tool blocks",
    },
    DemoPlateItem {
        id: "tree",
        label: "Open session tree",
        description: "Tree empty/no-match + selection stable on filter (c560)",
    },
    DemoPlateItem {
        id: "theme-toggle",
        label: "Toggle theme dark ↔ light",
        description: "Cycle palette (/theme toggle)",
    },
    DemoPlateItem {
        id: "thinking-level",
        label: "Cycle thinking border level",
        description: "Editor border color off→…→max (Shift+Tab)",
    },
    DemoPlateItem {
        id: "help-keys",
        label: "Key help",
        description: "Dump chords into transcript (/help)",
    },
    DemoPlateItem {
        id: "tests",
        label: "Run regression tests",
        description: "Queue fake cargo test tool + note",
    },
    DemoPlateItem {
        id: "compact-status",
        label: "Compaction status (c493)",
        description: "Status Compacting → ScrollNotice → Working (Alt+K)",
    },
    DemoPlateItem {
        id: "retry-status",
        label: "AutoRetry status (c493)",
        description: "Status Retry 1/3 → fail note → Working (Alt+Y)",
    },
    DemoPlateItem {
        id: "compact",
        label: "Compact conversation (legacy note)",
        description: "Alias → same as compact-status",
    },
];

fn demo_plate_select_items() -> Vec<SelectItem> {
    DEMO_PLATE
        .iter()
        .map(|p| SelectItem::new(p.id, p.label).with_description(p.description))
        .collect()
}

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

/// Demo model catalog for [`SlashArgCompletionSource`] (`/model <id>`).
const DEMO_MODELS: &[(&str, &str)] = &[
    ("sonnet-4", "anthropic"),
    ("claude-sonnet-4", "anthropic"),
    ("deepseek-v4-flash", "opencode-go"),
    ("deepseek/deepseek-v4-pro", "commandcode"),
    ("grok-4.5:slow", "cursor"),
    ("gpt-5", "openai-compat"),
    ("gpt-5-mini", "openai-compat"),
];

fn demo_model_catalog() -> Vec<(String, String)> {
    DEMO_MODELS
        .iter()
        .map(|(id, provider)| ((*id).to_string(), (*provider).to_string()))
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
            TreeNode::new("u1", "tighten footer truncation")
                .with_kind("user")
                .with_child(
                    TreeNode::new("a1", "plan + tools")
                        .with_kind("assistant")
                        .with_children([
                            TreeNode::new("t1", "rg -n TreeSelector").with_kind("tool"),
                            TreeNode::new("a2", "ship tree slot")
                                .with_kind("assistant")
                                .with_annotation("ship")
                                .with_annotation_at("2d ago")
                                .with_child(
                                    TreeNode::new("u2", "also verify double Esc").with_kind("user"),
                                ),
                        ]),
                ),
            TreeNode::new("fork", "alternate branch")
                .with_kind("user")
                .with_annotation("alt")
                .with_annotation_at("1h ago")
                .with_child(TreeNode::new("af", "(fork leaf)").with_kind("assistant")),
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
        let kind = node.kind.as_deref().unwrap_or("");
        match self {
            // demo default ≈ all (see c456 design.md)
            Self::Default | Self::All => true,
            Self::NoTools => kind != "tool",
            Self::UserOnly => kind == "user",
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

/// `None` = not found; `Some(None)` = `target` is a forest root; `Some(Some(id))` = parent id.
fn parent_id_of(roots: &[TreeNode], target: &str) -> Option<Option<String>> {
    fn walk(node: &TreeNode, target: &str, parent: Option<&str>) -> Option<Option<String>> {
        if node.id == target {
            return Some(parent.map(str::to_string));
        }
        for child in &node.children {
            if let Some(found) = walk(child, target, Some(&node.id)) {
                return Some(found);
            }
        }
        None
    }
    for root in roots {
        if let Some(found) = walk(root, target, None) {
            return Some(found);
        }
    }
    None
}

fn tree_label_preview(text: &str) -> String {
    let one = text.lines().next().unwrap_or(text).trim();
    if visible_width(one) > 48 {
        truncate_to_width(one, 48, "…", false)
    } else {
        one.to_string()
    }
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

#[cfg(test)]
impl ToolBlockStatus {
    /// RGB from active palette (`tool-*-bg`).
    pub fn rgb(self, palette: &Palette) -> (u8, u8, u8) {
        let c = match self {
            Self::Pending => palette.tool_pending_bg,
            Self::Success => palette.tool_success_bg,
            Self::Error => palette.tool_error_bg,
        };
        (c.r, c.g, c.b)
    }
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

/// c540: SBS with CJK so visible_width budgeting is obvious in demo.
fn sample_sbs_cjk_pair() -> DiffInput {
    DiffInput::LinePair {
        old: "标题：验收路径\n说明：窄宽折行\n".into(),
        new: "标题：发布路径\n说明：窄宽折行\n".into(),
        path: Some("说明.md".into()),
    }
}

/// c540: delete-only hunk → empty right half (no fake line number).
fn sample_sbs_empty_half_pair() -> DiffInput {
    DiffInput::LinePair {
        old: "only_old_line\n".into(),
        new: "\n".into(),
        path: Some("orphan.rs".into()),
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

/// Long file-style body for Head viewport (c550) — first lines stay visible when collapsed.
fn sample_long_read_output() -> String {
    let mut lines: Vec<String> = vec![
        "// packages/xylitol-tui/src/components/expandable_output.rs".into(),
        "pub fn render_expandable_output(...) -> Vec<String> {".into(),
        "    // Head keeps the first N visual lines".into(),
    ];
    lines.extend((4..=28).map(|i| format!("    // body line {i}")));
    lines.push("}".into());
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

fn demo_markdown_theme(scheme: TerminalColorScheme) -> MarkdownTheme {
    Palette::from(scheme).markdown_theme()
}

/// Richer streamed assistant body used when prompt asks for markdown / md.
fn markdown_showcase_stream_focus() -> &'static str {
    "本轮按 c530 打字机流式铺全语法 stub：标题分级、行内标记、链接/图、列表/任务、引用、表、多语言代码。"
}

/// Diff theme for demo blocks embedded in a tool wash (pi edit path).
///
/// Fg polarity + word spans on a **bright red/green wash** mixed from the block
/// bg toward polarity (not reverse, not a darker-only shade).
/// No row `diff-*-bg` — the expandable shell's `tool-*-bg` owns the block wash.
fn demo_diff_theme(scheme: TerminalColorScheme, block_bg: xylitol_tui::RgbColor) -> DiffTheme {
    let p = Palette::from(scheme);
    let added = p.diff_added;
    let removed = p.diff_removed;
    let context = p.diff_context;
    let word_added_bg = word_wash_bg(block_bg, added);
    let word_removed_bg = word_wash_bg(block_bg, removed);
    DiffTheme {
        added: Box::new(move |s| fg_rgb(added, s)),
        removed: Box::new(move |s| fg_rgb(removed, s)),
        context: Box::new(move |s| fg_rgb(context, s)),
        gutter: Box::new(move |s| fg_rgb(context, s)),
        meta: Box::new(move |s| fg_rgb(context, s)),
        word_change_added: Box::new(move |s| fg_bg_rgb(added, word_added_bg, block_bg, s)),
        word_change_removed: Box::new(move |s| fg_bg_rgb(removed, word_removed_bg, block_bg, s)),
        added_line_bg: Box::new(|s| s.to_string()),
        removed_line_bg: Box::new(|s| s.to_string()),
        highlight_line: Box::new(|s| s.to_string()),
    }
}

/// Full Markdown grammar stub for c530 / c535 — streamed via plate `md-full` or `/md`.
/// Source keeps fences so syntect can highlight; display has no fence chrome.
fn markdown_grammar_stub() -> &'static str {
    "\
# Markdown grammar stub (c530)

一级标题：色 + bold + underline，**（加粗）没有**井号前缀。

正文混排：**（加粗）强调**、*（斜体）语气*、`inline code`、~~strikethrough~~，以及 **（加粗）里的 `code`**。

## 二级标题

二级 accent + bold（无 underline）。

### 三级标题

三级 on-surface + bold。

#### 四级标题

##### 五级标题

###### 六级标题

### 链接与图片

行内链接：[docs](https://example.com/md) 与 [empty-ish](https://example.com/x)。

裸 URL：https://example.com/raw

图片（alt + url）：![diagram](https://example.com/a.png)

无 alt：![](https://example.com/blank.png)

### 列表

有序：

1. 有序一项
2. 有序二项
   - 嵌套无序
     1. 再嵌套有序
3. 有序三项含 [链接](https://example.com/list) 与 `code`

无序：

- 顶层 bullet
- 另一 bullet
  - 子项
  - 子项含 **（加粗）嵌套强调**

### 任务列表

- [ ] 未完成：终端验收
- [x] 已完成：解析 prompt
- [ ] 含 `inline` 与 [link](https://example.com/task)
  - [x] 嵌套已完成
  - [ ] 嵌套未完成

### 引用

> 引用第一行：muted + italic，左侧 `│ ` 竖线同色。
>
> 引用第二段仍安静。
>
> 引用里也可以有 **（加粗）词** 与 `code`。
>
> ```rust
> fn nested_in_quote() {
>     // must highlight inside │ gutter
>     println!(\"hi\");
> }
> ```
>
> ### 引用内标题
>
> | col | n |
> |-----|---|
> | x   | 1 |
>
> > 嵌套引用应叠两道 │

### 表格（空格对齐，无盒线）

| Name | Age | Role |
|------|-----|------|
| alice | 30 | eng |
| bob | 28 | design |
| cara | 31 | docs |

宽单元格：

| Feature | Status | Notes |
|---------|--------|-------|
| wrap | ok | long unbroken token-should-still-paint |
| cjk | ok | 中文列宽 |

### 代码块（源有 fence，显示无围栏）

```rust
fn demo(path: &str) -> bool {
    // syntect when highlight feature is on
    !path.is_empty()
}
```

```python
def accept(prompt: str) -> bool:
    # streamed / showcase
    return bool(prompt)
```

```typescript
function accept(prompt: string): boolean {
  return prompt.length > 0;
}
```

```json
{ \"ok\": true, \"note\": \"no fence chrome\", \"n\": 3 }
```

行内后再跟一段普通段落，确认块后恢复 body 色。

---

短 HR（非全宽墙）。

完：打字机流式应逐段重绘标题 / 列表 / 表 / 高亮；粗体斜体靠 SGR，语义靠（加粗）/（斜体）标注。"
}

/// Focused nested-list sample for plate `md-list-wrap` (library reference).
fn markdown_list_wrap_stub() -> &'static str {
    "\
### 列表嵌套 · 悬挂缩进（prewrapped）

1. 有序一项
2. 有序二项
   - 嵌套无序
     1. 再嵌套有序
3. 有序三项含 [链接](https://example.com/list) 与 `code`

4. 列表内块级

   > 列表下的引用应带 │

   ### 列表下标题

   | k | v |
   |---|---|
   | a | 1 |

窄终端下第 3 项应折行且续行保留与 `3. ` 对齐的空格，**不得**二次折行把悬挂缩进冲掉。"
}

/// Full Markdown grammar for c530 (via `/md` or Ctrl+P → md-full).
fn markdown_showcase_seed() -> &'static str {
    markdown_grammar_stub()
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

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn print_application_owned_acceptance_checklist() {
    eprintln!(
        "\
xylitol-tui agent_demo_alt · ApplicationOwned (alt-screen)
验收清单（c2070）：
  1. 终端进 alt-buffer（退出后主屏历史应恢复）
  2. transcript 内拖选高亮；松手默认 OSC52 复制
  3. 拖到顶/底可越界续选；滚轮滚应用视口（非终端 scrollback）
  4. 底部输入/footer dock 不可作 transcript 选区起点
  5. Ctrl+G 外编 suspend/resume 后仍保持 ApplicationOwned
  6. Editor 多行独立拖选高亮；松手 OSC52（与 transcript 选区隔离）
  7. 松手复制成功后 dock 内输入上方出现「Copied」约 2s（不进 transcript）
退出：Ctrl+C（空编辑器）或 /exit
"
    );
}

/// Interactive TTY → real `$EDITOR`; harness / non-TTY / explicit stub → stub.
///
/// Under `cfg(test)` (agent_demo included by `agent_demo_test`), never auto-prefer
/// real editor from inherited stdin TTY — otherwise interactive `just qa` flakes
/// while CI/pipe runs pass. Force real path with `XYLITOL_AGENT_DEMO_REAL_EDITOR=1`.
fn prefer_real_external_editor() -> bool {
    if env_flag("XYLITOL_AGENT_DEMO_EDITOR_STUB") {
        return false;
    }
    if env_flag("XYLITOL_AGENT_DEMO_REAL_EDITOR") {
        return true;
    }
    if cfg!(test) {
        return false;
    }
    std::io::stdin().is_terminal()
}

fn resolve_external_editor_command() -> Option<String> {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if cfg!(windows) {
                Some("notepad".into())
            } else {
                Some("nano".into())
            }
        })
}

/// Write `initial` to a tempfile, spawn `$VISUAL`/`$EDITOR`, return new text on
/// exit 0 (pi-compatible). Terminal must already be suspended by the caller.
fn run_external_editor_process(initial: &str) -> Result<Option<String>, String> {
    let editor_cmd = resolve_external_editor_command()
        .ok_or_else(|| "no editor configured (set VISUAL/EDITOR)".to_string())?;
    let path = std::env::temp_dir().join(format!(
        "xylitol-demo-editor-{}-{}.md",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    std::fs::write(&path, initial).map_err(|e| format!("write tempfile: {e}"))?;

    println!("Launching external editor: {editor_cmd}");
    println!("Demo will resume when the editor exits.");

    let mut parts = editor_cmd.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| "empty editor command".to_string())?;
    let mut args: Vec<&str> = parts.collect();
    let path_str = path.to_string_lossy();
    args.push(path_str.as_ref());

    let status = Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("spawn {program}: {e}"))?;

    let result = if status.success() {
        let new_content = std::fs::read_to_string(&path).map_err(|e| format!("read back: {e}"))?;
        // Match pi: drop a single trailing newline from the file.
        Some(
            new_content
                .strip_suffix('\n')
                .unwrap_or(&new_content)
                .to_string(),
        )
    } else {
        None
    };
    let _ = std::fs::remove_file(&path);
    Ok(result)
}

/// Shared demo entry. Binaries pick the mode at compile/link time — no env switch.
///
/// - `agent_demo` → [`InteractionMode::Inline`]
/// - `agent_demo_alt` → [`InteractionMode::ApplicationOwned`]
pub fn run(mode: InteractionMode) -> Result<(), Box<dyn std::error::Error>> {
    let defs = create_default_definitions();
    set_keybindings(KeybindingsManager::new(defs, HashMap::new()));

    let application_owned = mode.is_application_owned();
    if application_owned && std::io::stderr().is_terminal() {
        print_application_owned_acceptance_checklist();
    }

    let term = CrosstermTerminal::new()?;
    let mut tui = if application_owned {
        TUI::with_interaction_mode(term, InteractionMode::ApplicationOwned)
    } else {
        TUI::new(term)
    };
    // Inline lab only: `XYLITOL_TUI_MOUSE=1` → EnableMouseCapture.
    // ApplicationOwned enables mouse via begin_application_owned_session (inside start).
    if !application_owned && xylitol_tui::env_requests_mouse_capture() {
        tui.enable_mouse_capture();
    }
    if application_owned {
        // First-frame floor; refined from FakeCodingAgentApp dock measure.
        tui.set_dock_rows(8);
    }
    let quit_flag = Arc::new(AtomicBool::new(false));
    let initial_prompt = std::env::var("XYLITOL_AGENT_DEMO_INITIAL_PROMPT")
        .unwrap_or_else(|_| "tighten footer truncation and add a PTY acceptance test".into());

    let app = Rc::new(RefCell::new(FakeCodingAgentApp::new_with_prompt(
        quit_flag.clone(),
        &initial_prompt,
    )));
    FakeCodingAgentApp::install_input_listeners(&app, &mut tui);

    // Theme auto uses COLORFGBG only under crossterm — do NOT write OSC11/CSI
    // queries here: replies land on stdin as garbage keys (can fake Ctrl+G → $EDITOR).

    // After Ctrl+G sets pending: suspend terminal → `$EDITOR` → restore (pi shape).
    let app_hook = app.clone();
    tui.set_after_dispatch_hook(move |tui| {
        if tui.application_session_active() {
            let rows = tui.terminal.rows();
            app_hook.borrow_mut().set_term_rows_for_mouse(rows);
            let dock = app_hook.borrow().last_dock_rows();
            tui.set_dock_rows(dock);
            // Editor independent selection copy (ptim13) → same OSC52 flush path.
            let editor_clip = app_hook.borrow_mut().take_editor_clipboard();
            if !editor_clip.is_empty() {
                tui.enqueue_clipboard_sequences(editor_clip);
            }
            if tui.take_copy_notice() {
                app_hook.borrow_mut().arm_copy_notice();
            }
        }
        let pending = app_hook.borrow_mut().take_pending_external_editor();
        if !pending {
            return;
        }
        let text = app_hook.borrow().input.get_expanded_text();
        let outcome = tui.with_terminal_suspended(|| run_external_editor_process(&text));
        match outcome {
            Ok(Some(new_text)) => {
                app_hook.borrow_mut().apply_external_editor_text(new_text);
            }
            Ok(None) => {
                app_hook.borrow_mut().push_system(
                    "external editor exited non-zero — keeping original text".to_string(),
                );
            }
            Err(err) => {
                app_hook
                    .borrow_mut()
                    .push_system(format!("external editor failed: {err}"));
            }
        }
    });

    tui.add_child(Box::new(SharedFakeCodingAgentApp(app.clone())));
    tui.set_focus(Some(0));
    if application_owned {
        let cols = tui.terminal.columns() as usize;
        let rows = tui.terminal.rows();
        app.borrow_mut().set_term_rows_for_mouse(rows);
        let _ = app.borrow_mut().render(cols.max(1));
        tui.set_dock_rows(app.borrow().last_dock_rows());
    }
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

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        self.0.borrow().input_wants_rerender(event)
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        self.0.borrow_mut().take_editor_clipboard()
    }

    fn dock_rows_hint(&self) -> Option<usize> {
        Some(self.0.borrow().last_dock_rows())
    }

    fn wants_pointer_motion(&self) -> bool {
        self.0.borrow().input.is_selection_dragging()
    }

    fn clear_pointer_selection(&mut self) -> bool {
        Component::clear_pointer_selection(&mut self.0.borrow_mut().input)
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

/// Library-atom showcases that replace the editor slot (like Settings / plate).
#[derive(Clone, Copy, PartialEq, Eq)]
enum LibAtomKind {
    TruncatedText,
    CancellableLoader,
    Panel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    ScrollNotice,
}

/// App-layer glyph config (DESIGN.md): no font probing — env / Alt+G only.
/// (Ctrl+G is external-editor stub — see `open_external_editor_stub`.)
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryStyle {
    /// Full-row tool/user wash (legacy pi-ish).
    Wash,
    /// 1-cell status bg strip on tools/diff/bash; user/assistant/thinking flush.
    Rail,
}

impl EntryStyle {
    fn from_env() -> Self {
        match std::env::var("XYLITOL_AGENT_DEMO_ENTRY_STYLE")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some("rail") | Some("RAIL") => Self::Rail,
            _ => Self::Wash,
        }
    }

    fn cycle(self) -> Self {
        match self {
            Self::Wash => Self::Rail,
            Self::Rail => Self::Wash,
        }
    }

    /// Slash / footer token (`rail` | `wash`). Demo-native; product chrome vocab is separate.
    fn label(self) -> &'static str {
        match self {
            Self::Wash => "wash",
            Self::Rail => "rail",
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
    /// Ask tool: human summary in scrollback with fixed left rail (not tool wash).
    Ask {
        expanded: bool,
        summary: String,
        detail_lines: Vec<String>,
        /// pending | answered | skipped — drives rail color.
        phase: AskPhase,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AskPhase {
    Waiting,
    Answered,
    Skipped,
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
    /// Demo chrome status (c493 Compacting / Retry).
    SetStatus(String),
    /// Demo scrollback ScrollNotice.
    PushScrollNotice(String),
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
    /// Typeahead filter for command plate (SelectList::set_filter).
    palette_filter: String,
    settings_open: bool,
    settings: SettingsList,
    /// Library-atom editor-slot showcase (TruncatedText / CancellableLoader / Panel).
    lib_atom: Option<LibAtomKind>,
    /// Live spinner for `LibAtomKind::CancellableLoader` (Esc aborts).
    atom_loader: Option<CancellableLoader>,
    /// Built once when opening `LibAtomKind::Panel`.
    atom_panel: Option<Panel>,
    /// Pending ask-tool demo: complete tool block when ChoicePrompt finishes.
    ask_tool_pending_idx: Option<usize>,
    choice_prompt: Option<ChoicePrompt>,
    /// Shared slot filled by ChoicePrompt on_done.
    choice_pending: Option<Rc<RefCell<Option<ChoiceResult>>>>,
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
    /// Editor `!` prefix → bash-mode border (c457).
    bash_mode: bool,
    /// Ctrl+G external-editor stub invocation count (harness).
    external_editor_invocations: u32,
    /// Set by Ctrl+G when real editor path is chosen; consumed by TUI after-dispatch hook.
    pending_external_editor: bool,
    /// Opt-in theme auto-detect (`XYLITOL_AGENT_DEMO_THEME_AUTO=1` or harness).
    theme_auto: bool,
    /// Resolved Dark/Light token set (c458).
    theme_mode: TerminalColorScheme,
    /// Entry paint: full-row wash (legacy) vs left rail (product default; demo may differ).
    entry_style: EntryStyle,
    /// Editor thinking-level border (c1140); bash success border still wins while `!`.
    thinking_border_level: ThinkingBorderLevel,
    /// Last submit's resolved `$skill` → stub SKILL.md bodies (demo inject assert).
    last_skill_injections: Vec<(String, String)>,
    /// ApplicationOwned dock rows from last paint (status + editor slot + footer).
    last_dock_rows: usize,
    /// Status band height inside the dock (for remapping screen → editor-local).
    last_status_rows: usize,
    /// Editor slot height inside the dock (borders included).
    last_editor_rows: usize,
    /// Last known terminal rows (ApplicationOwned mouse remap).
    term_rows: u16,
    /// ApplicationOwned copy-success cue above the editor (`Copied`, ~2s TTL).
    copy_notice_until: Option<Instant>,
}

impl FakeCodingAgentApp {
    #[cfg(test)]
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

    #[cfg(test)]
    /// Test helper: current editor text (collapsed markers).
    pub fn input_text_for_test(&self) -> String {
        self.input.get_text()
    }

    #[cfg(test)]
    /// Test helper: editor text with `[paste #N …]` expanded (Ctrl+G / submit parity).
    pub fn input_expanded_text_for_test(&self) -> String {
        self.input.get_expanded_text()
    }

    #[cfg(test)]
    /// Test helper: footer metadata line (`cwd · model`).
    pub fn footer_note_for_test(&self) -> &str {
        &self.footer_note
    }

    /// ApplicationOwned dock rows measured on the last render (status + editor + footer).
    pub fn last_dock_rows(&self) -> usize {
        self.last_dock_rows.max(1)
    }

    /// Update terminal size used to remap ApplicationOwned mouse into the editor.
    pub fn set_term_rows_for_mouse(&mut self, rows: u16) {
        self.term_rows = rows.max(1);
    }

    /// Drain OSC52 sequences produced by Editor copy-on-release.
    pub fn take_editor_clipboard(&mut self) -> Vec<String> {
        self.input.take_pending_clipboard()
    }

    /// Remap absolute ApplicationOwned screen mouse → Editor via canonical origin path
    /// ([`xylitol_tui::editor_screen_origin`] + [`Editor::set_screen_origin`]).
    fn handle_editor_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;
        use xylitol_tui::{editor_screen_origin, mouse_in_dock};
        // Only left-button selection traffic; ignore wheel over dock here.
        if !matches!(
            mouse.kind,
            MouseEventKind::Down(crossterm::event::MouseButton::Left)
                | MouseEventKind::Drag(crossterm::event::MouseButton::Left)
                | MouseEventKind::Up(crossterm::event::MouseButton::Left)
                | MouseEventKind::Moved
        ) {
            return;
        }
        // Selectors replace the editor slot — don't steal their mouse.
        if self.palette_open
            || self.settings_open
            || self.tree_open
            || self.lib_atom.is_some()
            || self.choice_prompt.is_some()
        {
            return;
        }
        let dragging = self.input.is_selection_dragging();
        let dock = self.last_dock_rows.max(1);
        if !dragging && !mouse_in_dock(mouse.row, self.term_rows, dock) {
            return;
        }
        let (origin_row, origin_col) =
            editor_screen_origin(self.term_rows, dock, self.last_status_rows);
        // Clicks on the status band (not dragging) stay out of the editor.
        if !dragging && mouse.row < origin_row {
            return;
        }
        self.input.set_screen_origin(origin_row, origin_col);
        // Absolute screen coords — Editor subtracts origin in handle_input.
        self.input.handle_input(InputEvent::Mouse(mouse));
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        self.input.input_wants_rerender(event)
    }

    /// Arm the ApplicationOwned «Copied» dock cue (~2s). Not a ScrollNotice / transcript line.
    pub fn arm_copy_notice(&mut self) {
        self.copy_notice_until = Some(Instant::now() + Duration::from_millis(2000));
    }

    fn copy_notice_visible(&self) -> bool {
        self.copy_notice_until
            .is_some_and(|until| Instant::now() < until)
    }

    #[cfg(test)]
    /// Test helper: replace editor text (does not auto-sync bash border).
    pub fn set_editor_text_for_test(&mut self, text: impl Into<String>) {
        self.input.set_text(text.into());
    }

    #[cfg(test)]
    pub fn status_text_for_test(&self) -> &str {
        &self.status_text
    }

    #[cfg(test)]
    pub fn set_status_for_test(&mut self, text: impl Into<String>) {
        self.set_status(text);
    }

    #[cfg(test)]
    /// Harness: status stack above the editor (idle blank / busy blank+spinner).
    pub fn status_lines_for_test(&mut self, width: usize) -> Vec<String> {
        self.status_lines(width)
    }

    #[cfg(test)]
    pub fn clear_scheduled_actions_for_test(&mut self) {
        self.scheduled_actions.clear();
    }

    #[cfg(test)]
    /// Stop idle fallback turns from interfering with harness injects.
    pub fn freeze_script_for_test(&mut self) {
        self.auto_started = true;
        self.scripted_turn = 99;
        self.pending_events.clear();
    }

    #[cfg(test)]
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

    #[cfg(test)]
    /// Harness: push an already-finished tool (header has cmd; detail has no `$` echo).
    pub fn push_tool_for_test(&mut self, summary: impl Into<String>, detail: impl Into<String>) {
        self.push_tool(summary, detail, ToolBlockStatus::Success);
    }

    #[cfg(test)]
    /// Harness: transcript length (index of next push).
    pub fn transcript_len_for_test(&self) -> usize {
        self.transcript.len()
    }

    #[cfg(test)]
    /// Rendered transcript lines (including block spacers) for harness asserts.
    pub fn transcript_render_lines_for_test(&self, width: usize) -> Vec<String> {
        self.transcript_lines(width)
    }

    #[cfg(test)]
    /// Replace transcript with two short messages (block-gap tests).
    pub fn seed_two_user_blocks_for_test(&mut self) {
        self.transcript.clear();
        self.push_message(Role::User, "block-alpha");
        self.push_message(Role::User, "block-beta");
    }

    #[cfg(test)]
    /// Clear all transcript entries (harness).
    pub fn clear_transcript_for_test(&mut self) {
        self.transcript.clear();
    }

    #[cfg(test)]
    /// Harness: global tool-output viewport expand (Ctrl+O).
    pub fn tools_output_expanded_for_test(&self) -> bool {
        self.tools_output_expanded
    }

    #[cfg(test)]
    pub fn set_tools_output_expanded_for_test(&mut self, expanded: bool) {
        self.tools_output_expanded = expanded;
    }

    #[cfg(test)]
    /// Harness: append to a Tool detail (streaming viewport).
    pub fn append_tool_detail_for_test(&mut self, index: usize, chunk: impl Into<String>) {
        self.append_tool_detail_at(index, &chunk.into());
    }

    #[cfg(test)]
    /// Harness: flip a specific tool/diff entry to success.
    pub fn complete_tool_at_for_test(&mut self, index: usize) {
        self.set_tool_status_at(index, ToolBlockStatus::Success);
    }

    #[cfg(test)]
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
        if self.choice_prompt.is_some() {
            // Forward Esc into ChoicePrompt (cancel) via handle_input path.
            return false;
        }
        // CancellableLoader Esc is handled in handle_input (component on_abort).
        if matches!(self.lib_atom, Some(LibAtomKind::CancellableLoader)) {
            return false;
        }
        if self.lib_atom.is_some() {
            self.close_lib_atom();
            self.set_status("Ready");
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
        self.close_lib_atom();
        self.close_choice_prompt();
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

    fn close_lib_atom(&mut self) {
        self.lib_atom = None;
        self.atom_loader = None;
        self.atom_panel = None;
    }

    fn close_choice_prompt(&mut self) {
        self.choice_prompt = None;
        self.choice_pending = None;
    }

    fn choice_theme(&self) -> ChoicePromptTheme {
        let mut theme = self.palette().choice_prompt_theme();
        // Demo-only: wash = pi-style flush ChoicePrompt; product Ask stays rail.
        if matches!(self.entry_style, EntryStyle::Wash) {
            theme.rail = None;
        }
        theme
    }

    fn open_choice_prompt(&mut self, questions: Vec<ChoiceQuestion>) {
        self.palette_open = false;
        self.settings_open = false;
        self.tree_open = false;
        self.close_lib_atom();
        let pending: Rc<RefCell<Option<ChoiceResult>>> = Rc::new(RefCell::new(None));
        let slot = pending.clone();
        let prompt = ChoicePrompt::new(questions, self.choice_theme(), move |r| {
            *slot.borrow_mut() = Some(r);
        });
        self.choice_prompt = Some(prompt);
        self.choice_pending = Some(pending);
        self.set_status("Ask · Esc skip");
    }

    fn apply_choice_result(&mut self, result: ChoiceResult) {
        self.close_choice_prompt();
        let summary = result.human_summary_line();
        let detail_lines = result.human_detail_lines();
        if let Some(idx) = self.ask_tool_pending_idx.take() {
            if let Some(TranscriptEntry::Ask {
                summary: slot_sum,
                detail_lines: slot_det,
                expanded,
                phase,
            }) = self.transcript.get_mut(idx)
            {
                *slot_sum = summary;
                *slot_det = detail_lines;
                *expanded = false;
                *phase = if result.is_skipped() {
                    AskPhase::Skipped
                } else {
                    AskPhase::Answered
                };
            } else {
                self.transcript.push(TranscriptEntry::Ask {
                    expanded: false,
                    summary,
                    detail_lines,
                    phase: if result.is_skipped() {
                        AskPhase::Skipped
                    } else {
                        AskPhase::Answered
                    },
                });
            }
            self.set_status("Ready");
            return;
        }
        self.transcript.push(TranscriptEntry::Ask {
            expanded: false,
            summary,
            detail_lines,
            phase: if result.is_skipped() {
                AskPhase::Skipped
            } else {
                AskPhase::Answered
            },
        });
        self.set_status("Ready");
    }

    fn take_choice_result(&mut self) -> Option<ChoiceResult> {
        self.choice_pending
            .as_ref()
            .and_then(|p| p.borrow_mut().take())
    }

    fn open_lib_atom(&mut self, kind: LibAtomKind) {
        self.palette_open = false;
        self.settings_open = false;
        self.tree_open = false;
        self.tree_label_edit = None;
        self.close_choice_prompt();
        self.close_lib_atom();
        self.lib_atom = Some(kind);
        match kind {
            LibAtomKind::CancellableLoader => {
                let mut loader = CancellableLoader::new(
                    Box::new(cyan),
                    Box::new(dim),
                    "working — Esc aborts".into(),
                    // Full braille cycle (10 frames). A truncated prefix looks like
                    // the spinner only completes half a turn.
                    Some(LoaderIndicatorOptions::default()),
                );
                loader.on_abort = Some(Box::new(|| {}));
                self.atom_loader = Some(loader);
                self.set_status("CancellableLoader · Esc abort");
            }
            LibAtomKind::Panel => {
                let bg = self.palette().tool_pending_bg;
                let mut panel = Panel::new(2, 1, Some(Box::new(move |s: &str| bg_rgb(bg, s))));
                panel.add_child(Box::new(Text::new(
                    "Panel · padding + background".into(),
                    0,
                    0,
                )));
                panel.add_child(Box::new(Text::new(
                    "children inherit content width".into(),
                    0,
                    0,
                )));
                self.atom_panel = Some(panel);
                self.set_status("Panel · Esc closes");
            }
            LibAtomKind::TruncatedText => {
                self.set_status("TruncatedText · Esc closes");
            }
        }
    }

    fn history_entry_for(&self, id: &str) -> Option<TranscriptEntry> {
        self.history_payloads
            .get(id)
            .cloned()
            .or_else(|| seed_history_entry(id))
    }

    /// Append a child under the current history leaf and advance the leaf.
    fn grow_session_tree(&mut self, id: String, kind: &str, label: String, entry: TranscriptEntry) {
        let parent = self.history_leaf_id.clone();
        let node = TreeNode::new(id.clone(), label).with_kind(kind);
        if let Some(parent_node) = find_session_node_mut(&mut self.session_tree, &parent) {
            parent_node.children.push(node);
        } else if let Some(root) = self.session_tree.first_mut() {
            root.children.push(node);
        } else {
            self.session_tree.push(node);
        }
        self.history_payloads.insert(id.clone(), entry);
        self.history_leaf_id = id;
    }

    fn alloc_node_id(&mut self, kind: &str) -> String {
        self.next_node_seq += 1;
        format!("live-{kind}-{}", self.next_node_seq)
    }

    /// Enter on session tree: pi `navigateTree` morphology.
    /// - `kind=user`: leaf = parent; user body prefills editor; transcript = path to parent.
    /// - otherwise: leaf = id; rebuild path to id; do not prefill user body.
    pub fn travel_to_history(&mut self, id: &str) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.active_stream_entry = None;
        self.steer_queue.clear();
        // Keep follow-ups — they are for after idle, independent of travel.

        let is_user = find_session_node(&self.session_tree, id).and_then(|n| n.kind.as_deref())
            == Some("user");

        if is_user {
            let parent = parent_id_of(&self.session_tree, id)
                .flatten()
                .unwrap_or_else(|| id.to_string());
            let path = if parent == id {
                Vec::new()
            } else {
                path_ids_to(&self.session_tree, &parent).unwrap_or_default()
            };
            let path_label = if path.is_empty() {
                "(root)".to_string()
            } else {
                path.join(" → ")
            };

            self.transcript.clear();
            for node_id in &path {
                if let Some(entry) = self.history_entry_for(node_id) {
                    self.transcript.push(entry);
                }
            }
            // Trailing notice (above input) — same shape as product travel.
            self.push_message(
                Role::ScrollNotice,
                format!("history @ {id} · leaf={parent} · path: {path_label}"),
            );

            if let Some(TranscriptEntry::Message {
                role: Role::User,
                text,
            }) = self.history_entry_for(id)
            {
                let prefill = text
                    .strip_prefix("[steer] ")
                    .unwrap_or(text.as_str())
                    .to_string();
                self.input.set_text(prefill);
            } else {
                self.input.set_text(String::new());
            }

            self.history_leaf_id = parent;
        } else {
            let path = path_ids_to(&self.session_tree, id).unwrap_or_else(|| vec![id.to_string()]);
            let path_label = path.join(" → ");

            self.transcript.clear();
            for node_id in &path {
                if let Some(entry) = self.history_entry_for(node_id) {
                    self.transcript.push(entry);
                }
            }
            self.push_message(
                Role::ScrollNotice,
                format!("history @ {id} · path: {path_label}"),
            );

            self.input.set_text(String::new());
            self.history_leaf_id = id.to_string();
        }

        self.close_session_tree(); // Ready — banner lives in transcript, not a spinning status
    }

    /// Shift+F: fork at selected node (same session). Unlike travel, does **not** follow
    /// the linear reply spine — leaf stays on `id` so the next submit becomes a sibling branch.
    pub fn fork_from_history(&mut self, id: &str) {
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.active_stream_entry = None;
        self.steer_queue.clear();

        let path = path_ids_to(&self.session_tree, id).unwrap_or_else(|| vec![id.to_string()]);
        let path_label = path.join(" → ");

        self.transcript.clear();
        for node_id in &path {
            if let Some(entry) = self.history_entry_for(node_id) {
                self.transcript.push(entry);
            }
        }
        // Trailing notice (above input) — same shape as travel / product fork notes.
        self.push_message(
            Role::ScrollNotice,
            format!("forked @ {id} · path: {path_label} · edit & Enter to branch"),
        );

        // Prefill editor from user payload at the fork point (pi /fork morphology).
        if let Some(TranscriptEntry::Message {
            role: Role::User,
            text,
        }) = self.history_entry_for(id)
        {
            let prefill = text
                .strip_prefix("[steer] ")
                .unwrap_or(text.as_str())
                .to_string();
            self.input.set_text(prefill);
        } else {
            self.input.set_text(String::new());
        }

        self.history_leaf_id = id.to_string();
        self.close_session_tree();
    }

    #[cfg(test)]
    pub fn fork_from_selected_for_test(&mut self) {
        if let Some(id) = self.tree.selected_id().map(str::to_string) {
            self.fork_from_history(&id);
        }
    }

    #[cfg(test)]
    /// Child count of a session-tree node (harness — fork creates siblings).
    pub fn session_tree_child_count_for_test(&self, id: &str) -> usize {
        find_session_node(&self.session_tree, id)
            .map(|n| n.children.len())
            .unwrap_or(0)
    }

    #[cfg(test)]
    /// Harness: submit text as if the editor fired on_submit (bypasses paste-burst).
    pub fn submit_text_for_test(&mut self, text: impl Into<String>) {
        self.process_submit(text.into());
    }

    #[cfg(test)]
    /// Harness: drive one Component tick (script / streams / queues).
    pub fn tick_for_test(&mut self) -> bool {
        self.tick()
    }

    #[cfg(test)]
    pub fn bash_mode_for_test(&self) -> bool {
        self.bash_mode
    }

    #[cfg(test)]
    pub fn history_leaf_for_test(&self) -> &str {
        &self.history_leaf_id
    }

    #[cfg(test)]
    pub fn external_editor_invocations_for_test(&self) -> u32 {
        self.external_editor_invocations
    }

    #[cfg(test)]
    pub fn open_external_editor_stub_for_test(&mut self) {
        self.open_external_editor_stub();
    }

    #[cfg(test)]
    pub fn request_external_editor_for_test(&mut self) {
        self.request_external_editor();
    }

    pub fn take_pending_external_editor(&mut self) -> bool {
        let pending = self.pending_external_editor;
        self.pending_external_editor = false;
        pending
    }

    /// c493: Compacting status + ScrollNotice, then restore Working.
    fn demo_compaction_status(&mut self) {
        self.set_status("Compacting");
        self.push_message(
            Role::ScrollNotice,
            "compaction: auto: demo 90% of window (c493 chrome)",
        );
        self.schedule_from_now(
            36,
            TimedAction::PushScrollNotice("compaction complete".into()),
        );
        self.schedule_from_now(36, TimedAction::SetStatus("Working".into()));
        self.schedule_from_now(72, TimedAction::SetStatus("Ready".into()));
        self.push_message(
            Role::ScrollNotice,
            "watch status: Compacting (spinner) → Working → Ready · Alt+K / plate compact-status",
        );
    }

    #[cfg(test)]
    /// Harness: run c493 compaction status demo.
    pub fn demo_compaction_status_for_test(&mut self) {
        self.demo_compaction_status();
    }

    #[cfg(test)]
    /// Harness: run c493 retry status demo.
    pub fn demo_retry_status_for_test(&mut self) {
        self.demo_retry_status();
    }

    /// c493: Retry n/m then fail note + Working.
    fn demo_retry_status(&mut self) {
        self.set_status("Retry 1/3");
        self.push_message(
            Role::ScrollNotice,
            "auto-retry start · attempt 1/3 (c493 chrome)",
        );
        self.schedule_from_now(
            40,
            TimedAction::PushScrollNotice("retry failed (attempt 1)".into()),
        );
        self.schedule_from_now(40, TimedAction::SetStatus("Working".into()));
        self.schedule_from_now(80, TimedAction::SetStatus("Ready".into()));
        self.push_message(
            Role::ScrollNotice,
            "watch status: Retry 1/3 → Working → Ready · Alt+Y / plate retry-status",
        );
    }

    pub fn apply_external_editor_text(&mut self, text: String) {
        // `Editor::set_text` places the cursor at buffer end (pi setText parity).
        self.input.set_text(text);
        self.sync_editor_border();
        self.push_message(
            Role::ScrollNotice,
            "external editor saved — buffer replaced (Ctrl+G)".to_string(),
        );
    }

    #[cfg(test)]
    /// Test helper: editor cursor `(line, col)` after external-editor writeback.
    pub fn editor_cursor_for_test(&self) -> (usize, usize) {
        self.input.cursor_position()
    }

    pub fn push_system(&mut self, text: String) {
        self.push_message(Role::ScrollNotice, text);
    }

    /// Ctrl+G: real `$EDITOR` when TTY (or REAL_EDITOR=1); else harness-safe stub.
    fn request_external_editor(&mut self) {
        if prefer_real_external_editor() {
            self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
            self.pending_external_editor = true;
            return;
        }
        self.open_external_editor_stub();
    }

    #[cfg(test)]
    pub fn sync_editor_border_for_test(&mut self) {
        self.sync_editor_border();
    }

    #[cfg(test)]
    pub fn editor_render_for_test(&mut self, width: usize) -> Vec<String> {
        self.input.render(width)
    }

    #[cfg(test)]
    pub fn theme_mode_for_test(&self) -> TerminalColorScheme {
        self.theme_mode
    }

    #[cfg(test)]
    /// Harness: stub SKILL.md bodies injected on last user submit (A10 demo path).
    pub fn last_skill_injections_for_test(&self) -> &[(String, String)] {
        &self.last_skill_injections
    }

    #[cfg(test)]
    pub fn theme_auto_for_test(&self) -> bool {
        self.theme_auto
    }

    #[cfg(test)]
    pub fn set_theme_auto_for_test(&mut self, enabled: bool) {
        self.theme_auto = enabled;
        if !enabled {
            self.theme_mode = TerminalColorScheme::Dark;
        }
    }

    /// Active semantic palette (Dark=Mocha DESIGN, Light=Latte).
    pub fn palette(&self) -> Palette {
        Palette::from(self.theme_mode)
    }

    #[cfg(test)]
    /// Harness: apply OSC11 / COLORFGBG / CSI997 sources when auto is on.
    pub fn apply_theme_detect_for_test(
        &mut self,
        osc11_response: Option<&str>,
        colorfgbg: Option<&str>,
        scheme_report: Option<&str>,
    ) {
        if !self.theme_auto {
            self.theme_mode = TerminalColorScheme::Dark;
            return;
        }
        let osc11_background = osc11_response.and_then(parse_osc11_background_color);
        let color_scheme_report = scheme_report.and_then(parse_terminal_color_scheme_report);
        self.theme_mode = resolve_terminal_color_scheme(ThemeDetectSources {
            explicit: None,
            osc11_background,
            color_scheme_report,
            colorfgbg,
        });
    }

    #[cfg(test)]
    /// Host-driven live reply (OSC11 or CSI 997). No-op unless `theme_auto`.
    pub fn feed_terminal_color_reply(&mut self, data: &str) {
        if !self.theme_auto || !is_terminal_color_reply(data) {
            return;
        }
        let osc11_background = if is_osc11_background_color_response(data) {
            parse_osc11_background_color(data)
        } else {
            None
        };
        let color_scheme_report = parse_terminal_color_scheme_report(data);
        let colorfgbg = std::env::var("COLORFGBG").ok();
        self.theme_mode = resolve_terminal_color_scheme(ThemeDetectSources {
            explicit: None,
            osc11_background,
            color_scheme_report,
            colorfgbg: colorfgbg.as_deref(),
        });
    }

    fn refresh_theme_from_env(&mut self) {
        if !self.theme_auto {
            self.theme_mode = TerminalColorScheme::Dark;
            return;
        }
        let colorfgbg = std::env::var("COLORFGBG").ok();
        self.theme_mode = resolve_terminal_color_scheme(ThemeDetectSources {
            explicit: None,
            osc11_background: None,
            color_scheme_report: None,
            colorfgbg: colorfgbg.as_deref(),
        });
    }

    /// Explicit theme switch (disables auto-detect so COLORFGBG does not fight).
    pub fn apply_theme_command(&mut self, scheme: TerminalColorScheme) {
        self.theme_auto = false;
        self.theme_mode = scheme;
        self.refresh_editor_border_theme();
        self.push_message(
            Role::ScrollNotice,
            format!(
                "theme → {} (explicit; auto off). Try /theme dark|light|toggle",
                self.theme_label()
            ),
        );
        self.set_status(format!("Ready · {}", self.theme_label()));
    }

    fn cycle_theme(&mut self) {
        let next = match self.theme_mode {
            TerminalColorScheme::Dark => TerminalColorScheme::Light,
            TerminalColorScheme::Light => TerminalColorScheme::Dark,
        };
        self.apply_theme_command(next);
    }

    /// Parse `/theme` / `:theme` [dark|light|toggle]. Returns true if consumed.
    fn try_theme_command(&mut self, last_line: &str) -> bool {
        let body = last_line
            .strip_prefix('/')
            .or_else(|| last_line.strip_prefix(':'))
            .unwrap_or(last_line);
        let mut parts = body.split_whitespace();
        let Some(cmd) = parts.next() else {
            return false;
        };
        if !cmd.eq_ignore_ascii_case("theme") {
            return false;
        }
        match parts.next().unwrap_or("toggle") {
            "dark" => self.apply_theme_command(TerminalColorScheme::Dark),
            "light" => self.apply_theme_command(TerminalColorScheme::Light),
            "toggle" | "cycle" => self.cycle_theme(),
            other => {
                self.push_message(
                    Role::ScrollNotice,
                    format!("unknown theme arg `{other}` · use /theme [dark|light|toggle]"),
                );
                self.set_status("Ready");
            }
        }
        true
    }

    /// Parse `/entry-style` [rail|wash|toggle] — left-bg rail vs full wash.
    fn try_entry_style_command(&mut self, last_line: &str) -> bool {
        let body = last_line
            .strip_prefix('/')
            .or_else(|| last_line.strip_prefix(':'))
            .unwrap_or(last_line);
        let mut parts = body.split_whitespace();
        let Some(cmd) = parts.next() else {
            return false;
        };
        if !(cmd.eq_ignore_ascii_case("entry-style") || cmd.eq_ignore_ascii_case("entrystyle")) {
            return false;
        }
        match parts.next().unwrap_or("toggle") {
            "rail" => self.entry_style = EntryStyle::Rail,
            "wash" => self.entry_style = EntryStyle::Wash,
            "toggle" | "cycle" => self.entry_style = self.entry_style.cycle(),
            other => {
                self.push_message(
                    Role::ScrollNotice,
                    format!("unknown entry-style `{other}` · use /entry-style [rail|wash|toggle]"),
                );
                self.set_status("Ready");
                return true;
            }
        }
        self.push_message(
            Role::ScrollNotice,
            format!(
                "entry-style → {} (rail = left bg strip, wash = full-row / pi flush; applies to tools + Ask; demo-only)",
                self.entry_style.label()
            ),
        );
        self.set_status(format!("Ready · {}", self.entry_style.label()));
        true
    }

    /// Apply `/model <id>` (SetModel-style). Bare `/model` tips without changing footer.
    fn try_model_command(&mut self, last_line: &str) -> bool {
        let body = last_line
            .strip_prefix('/')
            .or_else(|| last_line.strip_prefix(':'))
            .unwrap_or(last_line);
        let mut parts = body.split_whitespace();
        let Some(cmd) = parts.next() else {
            return false;
        };
        if !cmd.eq_ignore_ascii_case("model") {
            return false;
        }
        match parts.next() {
            None => {
                self.push_message(
                    Role::ScrollNotice,
                    "model · pick an id from the list (`/model` or `/model <prefix>`) then Enter",
                );
                self.set_status("Ready");
            }
            Some(id) => {
                self.footer_note = format!("~/xylitol · {id}");
                self.push_message(Role::ScrollNotice, format!("model → {id}"));
                self.set_status(format!("Ready · {id}"));
            }
        }
        true
    }

    /// Re-apply editor border colors from the active palette (ignores bash early-return).
    fn refresh_editor_border_theme(&mut self) {
        let bash = self.input.get_text().trim_start().starts_with('!');
        self.bash_mode = bash;
        if bash {
            let success = self.palette().success;
            self.input
                .set_border_color(Box::new(move |s| fg_rgb(success, s)));
        } else {
            let palette = self.palette();
            apply_thinking_border(&mut self.input, &palette, self.thinking_border_level);
        }
    }

    fn theme_label(&self) -> &'static str {
        match self.theme_mode {
            TerminalColorScheme::Dark => "theme:dark",
            TerminalColorScheme::Light => "theme:light",
        }
    }

    /// Muted chrome from active palette.
    fn muted_paint(&self, s: &str) -> String {
        fg_rgb(self.palette().muted, s)
    }

    fn sync_editor_border(&mut self) {
        let bash = self.input.get_text().trim_start().starts_with('!');
        if bash == self.bash_mode {
            return;
        }
        self.bash_mode = bash;
        if bash {
            let success = self.palette().success;
            self.input
                .set_border_color(Box::new(move |s| fg_rgb(success, s)));
        } else {
            let palette = self.palette();
            apply_thinking_border(&mut self.input, &palette, self.thinking_border_level);
        }
    }

    /// Cycle thinking border level (c1140). Non-bash applies immediately.
    pub fn cycle_thinking_border_level(&mut self) {
        self.thinking_border_level = self.thinking_border_level.cycle_next();
        if !self.bash_mode {
            let palette = self.palette();
            apply_thinking_border(&mut self.input, &palette, self.thinking_border_level);
        }
        let level = self.thinking_border_level.as_str();
        self.push_message(Role::ScrollNotice, format!("thinking-border → {level}"));
        self.set_status(format!("Ready · thinking:{level}"));
    }

    #[cfg(test)]
    pub fn thinking_border_level_for_test(&self) -> ThinkingBorderLevel {
        self.thinking_border_level
    }

    #[cfg(test)]
    pub fn set_thinking_border_level_for_test(&mut self, level: ThinkingBorderLevel) {
        self.thinking_border_level = level;
        self.refresh_editor_border_theme();
    }

    /// Parse `/thinking-level` / `:thinking-level` [cycle]. Returns true if consumed.
    fn try_thinking_level_command(&mut self, last_line: &str) -> bool {
        let body = last_line
            .strip_prefix('/')
            .or_else(|| last_line.strip_prefix(':'))
            .unwrap_or(last_line);
        let mut parts = body.split_whitespace();
        let Some(cmd) = parts.next() else {
            return false;
        };
        if !cmd.eq_ignore_ascii_case("thinking-level") {
            return false;
        }
        match parts.next() {
            None | Some("cycle" | "toggle" | "next") => self.cycle_thinking_border_level(),
            Some(other) => {
                if let Some(level) = ThinkingBorderLevel::parse(other) {
                    self.thinking_border_level = level;
                    self.refresh_editor_border_theme();
                    self.push_message(
                        Role::ScrollNotice,
                        format!("thinking-border → {}", level.as_str()),
                    );
                    self.set_status(format!("Ready · thinking:{}", level.as_str()));
                } else {
                    self.push_message(
                        Role::ScrollNotice,
                        format!(
                            "unknown thinking-level `{other}` · use /thinking-level or off|minimal|low|medium|high|xhigh|max"
                        ),
                    );
                    self.set_status("Ready");
                }
            }
        }
        true
    }

    /// Ctrl+G: external editor — real `$EDITOR` on TTY via TUI suspend; stub in harness.
    fn open_external_editor_stub(&mut self) {
        self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
        // Expand paste markers so stub / harness see real content (pi getExpandedText).
        let text = self.input.get_expanded_text();
        self.push_message(
            Role::ScrollNotice,
            format!(
                "external editor stub (Ctrl+G) · {} chars · $EDITOR not spawned",
                text.len()
            ),
        );
        if text.is_empty() {
            self.input.set_text("# $EDITOR stub\n".to_string());
        } else if !text.contains("$EDITOR stub") {
            self.input
                .set_text(format!("{}\n# $EDITOR stub", text.trim_end()));
        }
        self.sync_editor_border();
    }

    #[cfg(test)]
    pub fn steer_queue_len_for_test(&self) -> usize {
        self.steer_queue.len()
    }

    #[cfg(test)]
    pub fn follow_up_queue_len_for_test(&self) -> usize {
        self.follow_up_queue.len()
    }

    #[cfg(test)]
    pub fn enqueue_follow_up_for_test(&mut self, text: impl Into<String>) {
        self.enqueue_follow_up(text.into());
    }

    #[cfg(test)]
    pub fn travel_to_history_for_test(&mut self, id: &str) {
        self.travel_to_history(id);
    }

    #[cfg(test)]
    /// Whether a label substring appears anywhere in the live session tree (harness).
    pub fn session_tree_contains_label_for_test(&self, needle: &str) -> bool {
        fn walk(nodes: &[TreeNode], needle: &str) -> bool {
            nodes
                .iter()
                .any(|n| n.label.contains(needle) || walk(&n.children, needle))
        }
        walk(&self.session_tree, needle)
    }

    #[cfg(test)]
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
                TranscriptEntry::Ask {
                    summary,
                    detail_lines,
                    ..
                } => {
                    out.push_str(summary);
                    out.push('\n');
                    for line in detail_lines {
                        out.push_str(line);
                        out.push('\n');
                    }
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

    #[cfg(test)]
    pub fn tree_open_for_test(&self) -> bool {
        self.tree_open
    }

    #[cfg(test)]
    pub fn tree_fold_selected_for_test(&mut self) {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        self.tree.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Left,
            KeyModifiers::CONTROL,
        )));
    }

    #[cfg(test)]
    pub fn tree_is_folded_for_test(&self, id: &str) -> bool {
        self.tree.is_folded(id)
    }

    #[cfg(test)]
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

    #[cfg(test)]
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
        self.push_message(Role::ScrollNotice, "stream aborted");
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
        // Pluggable CompletionSources: `/model <id>` + `/` + `@` + demo `$` stub.
        input.set_completion_sources(vec![
            Box::new(
                SlashArgCompletionSource::new("model", demo_model_catalog())
                    .with_id("model-id")
                    .with_bare_command(true),
            ),
            Box::new(SlashCommandSource::new(slash_commands())),
            Box::new(AtPathSource::new(cwd.clone())),
            Box::new(DemoDollarSource),
        ]);
        input.set_text(initial_prompt.to_string());

        let palette = SelectList::new(
            demo_plate_select_items(),
            8,
            SelectListTheme {
                selected_prefix: Box::new(cyan),
                selected_text: Box::new(selected_text),
                description: Box::new(dim),
                scroll_info: Box::new(dim),
                no_match: Box::new(red),
            },
            SelectListLayoutOptions {
                min_primary_column_width: Some(24),
                max_primary_column_width: Some(40),
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
                enable_search: true,
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
            palette_filter: String::new(),
            settings_open: false,
            settings,
            lib_atom: None,
            atom_loader: None,
            atom_panel: None,
            choice_prompt: None,
            choice_pending: None,
            ask_tool_pending_idx: None,
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
            bash_mode: false,
            external_editor_invocations: 0,
            pending_external_editor: false,
            theme_auto: std::env::var("XYLITOL_AGENT_DEMO_THEME_AUTO")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            theme_mode: TerminalColorScheme::Dark,
            entry_style: EntryStyle::from_env(),
            thinking_border_level: ThinkingBorderLevel::Medium,
            last_skill_injections: Vec::new(),
            last_dock_rows: 8,
            last_status_rows: 1,
            last_editor_rows: 3,
            term_rows: 24,
            copy_notice_until: None,
        };
        if app.theme_auto {
            app.refresh_theme_from_env();
        }
        app.refresh_editor_border_theme();
        app.seed_transcript();
        app
    }

    fn seed_transcript(&mut self) {
        // Slim chrome (c535): short pointer + compact kit. Full Markdown → plate `/md`.
        self.push_message(
            Role::ScrollNotice,
            "demo · /entry-style rail|wash · ! bash · busy Enter=steer · Alt+Enter=follow-up · /help",
        );
        self.push_message(
            Role::User,
            "Collapse examples into one fake coding-agent demo and keep foot interaction stable.",
        );
        self.push_thinking(
            "Plan: keep a compact kit in seed (thinking/tools/diff); full Markdown grammar streams from plate.\n\nKeep transcript in scrollback; mark the editor as the operation zone with borders.",
        );
        self.push_message(
            Role::Assistant,
            "Ready. **Ctrl+P** opens the command plate: full Markdown typewriter, streamed highlight, more Diff/tools. Collapse hints: `(Ctrl+T)` / `(Alt+E)`.",
        );
        self.push_tool(
            "read packages/xylitol-tui/examples/agent_demo.rs · 42ms · 790 lines",
            "ok — opened agent_demo.rs\n(preview) FakeCodingAgentApp + scripted turn harness",
            ToolBlockStatus::Success,
        );
        self.push_tool(
            "$ bun test (timeout 120s) · ok",
            sample_long_bash_output(),
            ToolBlockStatus::Success,
        );
        let demo_rs = format_edit_path("packages/xylitol-tui/examples/agent_demo.rs", &self.cwd);
        let ui_root = format_edit_path("src/app/tui/ui_root.rs", &self.cwd);
        self.push_diff_ex(
            format!("edited {demo_rs} (+2 -2) unified edit-format"),
            sample_unified_pair(),
            None,
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
        self.push_tool(
            "cargo test -p xylitol-tui --test missing · fail",
            "error — test binary `missing` not found (demo stub)",
            ToolBlockStatus::Error,
        );
        self.push_tool(
            "! echo rail-bg-strip",
            "rail-bg-strip\n(demo interactive bang · editor ! border when typing)",
            ToolBlockStatus::Success,
        );
    }

    fn inject_help_keys(&mut self) {
        self.push_message(
            Role::ScrollNotice,
            "keys: Enter submit/steer · Alt+Enter follow-up · /md Markdown stream · Ctrl+P plate · \
             /theme [dark|light|toggle] · /entry-style rail|wash · Shift+Tab thinking-border · /help · /diff · ! bash · Ctrl+G $EDITOR · double Esc tree · \
             (Ctrl+T) thinking · (Alt+E) tools · (Ctrl+O) tools viewport · Alt+G glyphs · \
             Alt+K compact-status · Alt+Y retry-status · Esc · Ctrl+C",
        );
        self.push_message(
            Role::ScrollNotice,
            "stream plate: md-full · stream-rust/python/typescript/json · diff-sbs · \
             completion-dollar (c545 $) · expandable-head (c550) · playground-sync (c555) · \
             md-list-wrap · narrow-clamp · truncated-text · cancellable-loader · panel · \
             ask-single · ask-multi · ask-tabs · ask-tool · tree (c560) · tool-tints · theme-toggle · \
             help-keys · tests · compact-status · retry-status",
        );
        self.set_status("Ready");
    }

    fn inject_completion_dollar_tip(&mut self) {
        self.push_message(Role::User, "plate · completion-dollar · A10 skill-ref");
        self.push_message(
            Role::User,
            "Please run $demo and also $narrow-clamp-skill-with-a-very-long-identifier together.",
        );
        self.push_message(
            Role::ScrollNotice,
            "A10 preview: `$name` tokens in the user row above use skill-ref (mauve/purple). \
             Submit a line with `$demo` — stub SKILL.md is recorded for inject assert \
             (no per-skill tint blocks; no status count). Type `use $` for completion popup \
             (c545).",
        );
        // Drive inject path so plate alone proves resolve without Enter.
        self.last_skill_injections = resolve_demo_skill_injections(
            "Please run $demo and also $narrow-clamp-skill-with-a-very-long-identifier together.",
        );
        // A10: no status/footer skills:N — inject evidence is `last_skill_injections` only.
        self.set_status("Ready");
    }

    fn inject_expandable_head_showcase(&mut self) {
        self.push_message(Role::User, "plate · expandable-head · c550");
        self.push_message(
            Role::ScrollNotice,
            "c550: Read-style tools use TruncateFrom::Head — first N lines stay on top; \
             dim `more lines` hint sits below (not above like Tail/earlier). Ctrl+O expands \
             the viewport; width 0/1 stays safe.",
        );
        self.push_tool(
            "Read expandable_output.rs · ok",
            sample_long_read_output(),
            ToolBlockStatus::Success,
        );
        // Keep viewport collapsed so the more-lines hint is visible.
        self.tools_output_expanded = false;
        self.set_status("Head viewport · Ctrl+O to expand");
    }

    fn inject_playground_sync_tip(&mut self) {
        self.push_message(Role::User, "plate · playground-sync · c555");
        self.push_message(
            Role::ScrollNotice,
            "This demo (`just demo-tui` / `agent_demo`) is the **package** Inline harness. \
             ApplicationOwned: `just demo-tui-alt-screen` (`agent_demo_alt`). \
             It MAY differ from product chrome / copy. Product visual SSOT = \
             `src/app/tui/DESIGN.md` + `design/` (+ static `design/playground/` — Agents ignore \
             by default). Tokens: DESIGN.md → `just sync-tui-tokens` (`sync_tokens.py`) → \
             tokens.css/js; keep `Palette` aligned. Runtime MD: `/md` · plates `md-list-wrap` / \
             `narrow-clamp`.",
        );
        self.set_status("Ready · try /md for runtime MD");
    }

    fn inject_md_list_wrap_showcase(&mut self) {
        self.push_message(Role::User, "plate · md-list-wrap · list prewrapped");
        self.push_message(
            Role::ScrollNotice,
            "Library reference: Markdown list/table/quote rows are `prewrapped` — one hanging-indent \
             wrap pass, no second outer wrap that flushes continuation lines. Nested ordered markers \
             use pulldown `List(Some(n))`. Shrink the terminal and watch item 3 keep spaces under `3. `. \
             Full grammar: `/md`. Playground visual: slot Markdown + Widgets.",
        );
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.scheduled_tail_tick = self.script_tick;
        self.active_stream_entry = None;
        self.auto_started = true;
        self.scripted_turn = self.scripted_turn.max(2);
        self.set_status("Streaming list wrap sample");
        self.queue_markdown_typewriter(markdown_list_wrap_stub());
    }

    fn inject_narrow_clamp_showcase(&mut self) {
        self.push_message(Role::User, "plate · narrow-clamp · widgets");
        self.push_message(
            Role::ScrollNotice,
            "Library reference: SelectList / SettingsList / Input / Loader clamp every row to the \
             width budget (including width 0/1). Settings search empty → `No matching settings`; \
             SelectList filter → `No matching items`; Input prompt clips instead of overflowing; \
             Loader clamps Text padding. Try: type `zzz` here in Settings search, or Ctrl+P then \
             filter `zzz`. Playground: slot Widgets (key 8).",
        );
        // Rebuild settings with search so library users can demo empty-match live.
        self.settings = SettingsList::new(
            vec![
                SettingItem {
                    id: "model".into(),
                    label: "Model".into(),
                    description: Some("execution model — long enough to exercise truncate".into()),
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
                cursor: "> ".into(),
                hint: Box::new(dim),
            },
            |_id: &str, _val: &str| {},
            || {},
            SettingsListOptions {
                enable_search: true,
            },
        );
        self.palette_open = false;
        self.tree_open = false;
        self.close_lib_atom();
        self.close_choice_prompt();
        self.settings_open = true;
        self.set_status("Settings · type zzz for no-match · Esc closes");
    }

    fn inject_truncated_text_atom(&mut self) {
        self.push_message(Role::User, "plate · truncated-text · atom");
        self.push_message(
            Role::ScrollNotice,
            "Library reference: TruncatedText keeps a single line, pads, and ellipsizes to the \
             width budget. Playground: slot Atoms (key 9). Esc closes this slot.",
        );
        self.open_lib_atom(LibAtomKind::TruncatedText);
    }

    fn inject_cancellable_loader_atom(&mut self) {
        self.push_message(Role::User, "plate · cancellable-loader · atom");
        self.push_message(
            Role::ScrollNotice,
            "Library reference: CancellableLoader ticks like Loader; Esc matches \
             `tui.select.cancel` and fires `on_abort`. Playground: slot Atoms (key 9).",
        );
        self.open_lib_atom(LibAtomKind::CancellableLoader);
    }

    fn inject_panel_atom(&mut self) {
        self.push_message(Role::User, "plate · panel · atom");
        self.push_message(
            Role::ScrollNotice,
            "Library reference: Panel (pi Box) pads children and paints an optional background on \
             every line. Playground: slot Atoms (key 9). Esc closes.",
        );
        self.open_lib_atom(LibAtomKind::Panel);
    }

    fn inject_ask_single(&mut self) {
        self.push_message(Role::User, "plate · ask-single");
        self.push_message(
            Role::ScrollNotice,
            "Ask wrapper · 1×Single（无 Tabs）· Other 默认开 · Esc→skipped. Playground: ?slot=ask",
        );
        self.open_choice_prompt(vec![ChoiceQuestion {
            id: "fork".into(),
            label: "Fork".into(),
            prompt: "实现分叉：先修哪条路径？".into(),
            mode: ChoiceMode::Single,
            options: vec![
                ChoiceOption::new("slice", "最小可运行切片")
                    .with_description("先打通一条能跑的路径，再补合约与打磨。")
                    .recommended(),
                ChoiceOption::new("bdd", "先写合约/BDD")
                    .with_description("先钉 MUST/场景，再实现。适合边界已清。"),
                ChoiceOption::new("ux", "先打磨 Tabs UX")
                    .with_description("先把问卷交互做顺手，再接产品工具。"),
            ],
            allow_other: true,
        }]);
    }

    fn inject_ask_multi(&mut self) {
        self.push_message(Role::User, "plate · ask-multi");
        self.push_message(
            Role::ScrollNotice,
            "Ask wrapper · 1×Multi（无 Tabs）· Space 勾选 · Esc skip",
        );
        self.open_choice_prompt(vec![ChoiceQuestion {
            id: "clarify".into(),
            label: "Clarify".into(),
            prompt: "这轮要先澄清哪些点？".into(),
            mode: ChoiceMode::Multi,
            options: vec![
                ChoiceOption::new("scope", "目标范围")
                    .with_description("这轮要交付什么、不做什么。"),
                ChoiceOption::new("acceptance", "验收标准")
                    .with_description("怎样算完成：测、手验、演示。"),
                ChoiceOption::new("fork", "实现分叉").with_description("多条路径时先定优先级。"),
            ],
            allow_other: true,
        }]);
    }

    fn inject_ask_tabs(&mut self) {
        self.push_message(Role::User, "plate · ask-tabs");
        self.push_message(
            Role::ScrollNotice,
            "Ask wrapper · ≥2 Tabs：Enter 推进 · ←→ 回退修正 · Review 提交 · Esc skip",
        );
        self.open_choice_prompt(vec![
            ChoiceQuestion {
                id: "scope".into(),
                label: "Scope".into(),
                prompt: "本轮范围？".into(),
                mode: ChoiceMode::Single,
                options: vec![
                    ChoiceOption::new("pkg", "仅包"),
                    ChoiceOption::new("demo", "包+demo"),
                ],
                allow_other: true,
            },
            ChoiceQuestion {
                id: "checks".into(),
                label: "Checks".into(),
                prompt: "需要哪些验收？".into(),
                mode: ChoiceMode::Multi,
                options: vec![
                    ChoiceOption::new("unit", "单测"),
                    ChoiceOption::new("harness", "harness"),
                    ChoiceOption::new("manual", "手验 demo"),
                ],
                allow_other: true,
            },
            ChoiceQuestion {
                id: "priority".into(),
                label: "Priority".into(),
                prompt: "优先级？".into(),
                mode: ChoiceMode::Single,
                options: vec![
                    ChoiceOption::new("p0", "P0 现在"),
                    ChoiceOption::new("p1", "P1 本周"),
                ],
                allow_other: true,
            },
        ]);
    }

    fn inject_ask_tool(&mut self) {
        self.push_message(Role::User, "plate · ask-tool（假工具调用）");
        self.push_message(
            Role::Assistant,
            "I'll use ask to clarify the implementation fork before coding.",
        );
        let idx = self.transcript.len();
        self.transcript.push(TranscriptEntry::Ask {
            expanded: false,
            summary: "Ask · 等待回答…".into(),
            detail_lines: Vec::new(),
            phase: AskPhase::Waiting,
        });
        self.ask_tool_pending_idx = Some(idx);
        self.set_status("Ask · waiting");
        self.open_choice_prompt(vec![ChoiceQuestion {
            id: "fork".into(),
            label: "Fork".into(),
            prompt: "实现分叉：先修哪条路径？".into(),
            mode: ChoiceMode::Single,
            options: vec![
                ChoiceOption::new("slice", "最小可运行切片")
                    .with_description("先打通一条能跑的路径，再补合约与打磨。适合想尽快看到反馈。")
                    .recommended(),
                ChoiceOption::new("bdd", "先写合约/BDD")
                    .with_description("先钉 MUST/场景，再实现。适合边界已清、怕返工。"),
                ChoiceOption::new("ux", "先打磨 Tabs UX")
                    .with_description("先把问卷交互做顺手，再接产品工具。适合形态未定。"),
            ],
            allow_other: true,
        }]);
    }

    fn inject_diff_showcase(&mut self) {
        let demo_rs = format_edit_path("packages/xylitol-tui/examples/agent_demo.rs", &self.cwd);
        let ui_root = format_edit_path("src/app/tui/ui_root.rs", &self.cwd);
        self.push_message(Role::User, "plate · diff-sbs · c540 edges");
        self.push_diff_ex(
            format!("edited {demo_rs} (+2 -2) unified edit-format"),
            sample_unified_pair(),
            None,
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
            "edited 说明.md (+1 -1) side-by-side CJK (c540)",
            sample_sbs_cjk_pair(),
            Some(40),
            true,
            ToolBlockStatus::Success,
        );
        self.push_diff_ex(
            "edited orphan.rs (−1) side-by-side empty half (c540)",
            sample_sbs_empty_half_pair(),
            Some(40),
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
        self.set_status("Ready");
    }

    fn inject_tool_tint_showcase(&mut self) {
        self.push_message(Role::User, "plate · tool-tints");
        self.push_thinking(
            "Plan: show tool status tints and long bash collapse. Toggle with (Ctrl+T)/(Alt+E).",
        );
        self.push_tool(
            "read packages/xylitol-tui/examples/agent_demo.rs · 42ms · 790 lines",
            "ok — opened agent_demo.rs\n(preview) FakeCodingAgentApp + scripted turn harness",
            ToolBlockStatus::Success,
        );
        self.push_tool(
            "$ bun test (timeout 120s) · ok",
            sample_long_bash_output(),
            ToolBlockStatus::Success,
        );
        self.push_tool(
            "cargo test -p xylitol-tui --test missing · fail",
            "error — test binary `missing` not found (demo stub)",
            ToolBlockStatus::Error,
        );
        self.set_status("Ready");
    }

    /// Typewriter-stream the full grammar stub (steadier pace than normal reply jitter).
    fn stream_markdown_grammar(&mut self) {
        self.push_message(Role::User, "/md · request Markdown showcase");
        self.pending_events.clear();
        self.scheduled_actions.clear();
        self.scheduled_tail_tick = self.script_tick;
        self.active_stream_entry = None;
        // Prevent idle `tick` fallback (scripted_turn 0/1 tool wave) from interleaving.
        self.auto_started = true;
        self.scripted_turn = self.scripted_turn.max(2);
        self.set_status("Drafting reply");
        self.queue_markdown_typewriter(markdown_grammar_stub());
    }

    fn run_demo_plate(&mut self, id: &str) {
        match id {
            "md-full" => self.stream_markdown_grammar(),
            "stream-rust" => self.commit_user_turn("stream rust highlight".into()),
            "stream-python" => self.commit_user_turn("stream python highlight".into()),
            "stream-typescript" => self.commit_user_turn("stream typescript highlight".into()),
            "stream-json" => self.commit_user_turn("stream json highlight".into()),
            "diff-sbs" => self.inject_diff_showcase(),
            "completion-dollar" => self.inject_completion_dollar_tip(),
            "expandable-head" => self.inject_expandable_head_showcase(),
            "playground-sync" => self.inject_playground_sync_tip(),
            "md-list-wrap" => self.inject_md_list_wrap_showcase(),
            "narrow-clamp" => self.inject_narrow_clamp_showcase(),
            "truncated-text" => self.inject_truncated_text_atom(),
            "cancellable-loader" => self.inject_cancellable_loader_atom(),
            "panel" => self.inject_panel_atom(),
            "ask-single" => self.inject_ask_single(),
            "ask-multi" => self.inject_ask_multi(),
            "ask-tabs" => self.inject_ask_tabs(),
            "ask-tool" => self.inject_ask_tool(),
            "tool-tints" => self.inject_tool_tint_showcase(),
            "tree" => {
                self.push_message(Role::User, "plate · tree · c560");
                self.push_message(
                    Role::ScrollNotice,
                    "c560: TreeSelector empty/no-match shows a dim hint (not a blank list). \
                     Filter/search keeps the prior selected id when still visible; otherwise \
                     falls back to the first visible row. Type a nonsense search to see empty; \
                     Ctrl+T cycles demo filters.",
                );
                self.tree_open = true;
                self.palette_open = false;
                self.settings_open = false;
                self.close_lib_atom();
                self.close_choice_prompt();
                self.set_status("Session tree · c560 empty/selection");
            }
            "help-keys" => self.inject_help_keys(),
            "theme-toggle" => self.cycle_theme(),
            "thinking-level" => self.cycle_thinking_border_level(),
            "tests" => {
                self.pending_events
                    .push_back(ScriptEvent::Tool("cargo test -p xylitol-tui --lib".into()));
                self.pending_events.push_back(ScriptEvent::Assistant(
                    "Regression tests are queued. Next step: rerun the PTY smoke against the primary example.".into(),
                ));
            }
            "compact-status" | "compact" => self.demo_compaction_status(),
            "retry-status" => self.demo_retry_status(),
            _ => {}
        }
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
                    | TranscriptEntry::Ask { expanded: true, .. }
            )
        });
        for entry in &mut self.transcript {
            match entry {
                TranscriptEntry::Tool { expanded, .. }
                | TranscriptEntry::Diff { expanded, .. }
                | TranscriptEntry::Ask { expanded, .. } => {
                    *expanded = !any_expanded;
                }
                _ => {}
            }
        }
    }

    fn cycle_glyph_set(&mut self) {
        self.glyph_set = self.glyph_set.cycle();
        self.push_message(
            Role::ScrollNotice,
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
            self.palette_filter.clear();
            self.palette.set_filter("");
            return;
        }

        if last_line == "/md" || last_line == ":md" {
            self.input.set_text(String::new());
            self.stream_markdown_grammar();
            // Do not advance_script — that would enqueue the idle fallback tool wave
            // on top of the Markdown typewriter.
            return;
        }

        if last_line == "/help" || last_line == ":help" {
            self.input.set_text(String::new());
            self.inject_help_keys();
            return;
        }

        if last_line == "/diff" || last_line == ":diff" {
            self.input.set_text(String::new());
            self.inject_diff_showcase();
            return;
        }

        if self.try_theme_command(last_line) {
            self.input.set_text(String::new());
            return;
        }

        if self.try_entry_style_command(last_line) {
            self.input.set_text(String::new());
            return;
        }

        if self.try_thinking_level_command(last_line) {
            self.input.set_text(String::new());
            return;
        }

        if self.try_model_command(last_line) {
            self.input.set_text(String::new());
            return;
        }

        if last_line == "/compact" || last_line == ":compact" {
            self.input.set_text(String::new());
            self.demo_compaction_status();
            return;
        }
        if last_line == "/retry" || last_line == ":retry" {
            self.input.set_text(String::new());
            self.demo_retry_status();
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
                "Working" | "Thinking" | "Drafting reply" | "Running tools" | "Compacting"
            )
            || self.status_text.starts_with("Retry ")
    }

    fn enqueue_steer(&mut self, text: String) {
        self.input.set_text(String::new());
        self.steer_queue.push_back(text.clone());
        self.push_message(
            Role::ScrollNotice,
            format!("steer queued ({}) · {text}", self.steer_queue.len()),
        );
        // Grow tree now so the steer is visible in the session graph without aborting tools.
        let id = self.alloc_node_id("u");
        self.grow_session_tree(
            id,
            "user",
            tree_label_preview(&format!("[steer] {text}")),
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
            Role::ScrollNotice,
            format!("follow-up queued ({}) · {text}", self.follow_up_queue.len()),
        );
    }

    fn commit_user_turn(&mut self, trimmed: String) {
        self.last_submitted = trimmed.clone();
        self.last_skill_injections = resolve_demo_skill_injections(&trimmed);
        self.push_message(Role::User, trimmed.clone());
        let user_id = self.alloc_node_id("u");
        self.grow_session_tree(
            user_id,
            "user",
            tree_label_preview(&trimmed),
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
                Role::ScrollNotice,
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
                Role::ScrollNotice,
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
            "Working" | "Thinking" | "Drafting reply" | "Running tools" | "Compacting"
        ) || self.status_text.starts_with("Retry ");
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

    /// Irregular typewriter for Markdown stubs — slower, hitchy, more human.
    fn queue_markdown_typewriter(&mut self, text: &str) {
        // Spin-up pause before first glyph.
        let start_delay = self.jitter_ticks(10, 22);
        self.schedule_after_ticks(start_delay, TimedAction::StreamStart(StreamKind::Assistant));
        let chars: Vec<char> = text.chars().collect();
        let mut index = 0usize;
        while index < chars.len() {
            let current = chars[index];
            let take = if current == '\n' {
                1
            } else {
                match self.random_between(0, 11) {
                    0 => self.random_between(8, 16) as usize, // rare burst
                    1..=3 => self.random_between(3, 6) as usize,
                    4..=7 if current.is_ascii() => self.random_between(1, 3) as usize,
                    4..=7 => 1,
                    _ => 1, // drip
                }
            };
            let end = (index + take).min(chars.len());
            let chunk: String = chars[index..end].iter().collect();
            // Mostly short delays; occasional hitch / think-pause (esp. after newline).
            let chunk_delay = if current == '\n' {
                self.jitter_ticks(4, 14)
            } else {
                match self.random_between(0, 9) {
                    0 => self.random_between(12, 28), // long hitch
                    1..=2 => self.random_between(6, 12),
                    _ => self.random_between(2, 6),
                }
            };
            self.schedule_after_ticks(
                chunk_delay,
                TimedAction::StreamChunk(StreamKind::Assistant, chunk),
            );
            index = end;
        }
        let finish_delay = self.jitter_ticks(6, 16);
        self.schedule_after_ticks(
            finish_delay,
            TimedAction::StreamFinish(StreamKind::Assistant),
        );
    }

    /// Pick streamed code sample: keyword wins; else rotate rust→python→ts→json.
    /// Source still uses fenced Markdown so syntect can highlight when the block closes;
    /// rendered output has no fence chrome (c530).
    fn pick_stream_fence(&mut self, prompt: &str) -> (&'static str, &'static str) {
        let p = prompt.to_ascii_lowercase();
        let (lang, focus) = if p.contains("python") || p.contains("py ") {
            (
                "python",
                "下面流式吐一段 Python 代码块，对照其它语言看多语言高亮（显示无围栏）。",
            )
        } else if p.contains("typescript")
            || p.contains(".ts")
            || p.split_whitespace().any(|w| w == "ts" || w == "tsx")
        {
            (
                "typescript",
                "下面流式吐一段 TypeScript 代码块，验收 syntect 在 TS 上的着色。",
            )
        } else if p.contains("json") {
            (
                "json",
                "下面流式吐一段 JSON 代码块，看结构字面量高亮是否干净。",
            )
        } else if p.contains("rust") || p.contains("highlight") || p.contains("stream") {
            (
                "rust",
                "下面会流式吐出一段 Rust 代码块，验收 syntect 在未闭合→闭合过程中的表现。",
            )
        } else if p.contains("cjk") || p.contains("emoji") {
            (
                "rust",
                "我会先盯住 CJK/emoji 的宽度预算，再看真实终端回放。",
            )
        } else if p.contains("palette") || p.contains("command") {
            ("rust", "我会先看 overlay 覆盖语义，再补 PTY/tmux smoke。")
        } else if p.contains("markdown") || p.contains("md ") || p.contains("/md") {
            ("rust", markdown_showcase_stream_focus())
        } else {
            let langs = ["rust", "python", "typescript", "json"];
            let lang = langs[self.fence_rotate % langs.len()];
            self.fence_rotate = self.fence_rotate.wrapping_add(1);
            (
                lang,
                "我会先复现主流程；本轮流式代码语言会轮换，方便肉眼对比高亮。",
            )
        };

        let fence = match lang {
            "python" => {
                "```python\ndef accept(prompt: str) -> bool:\n    # streamed block — highlight when fence closes\n    return bool(prompt)\n```"
            }
            "typescript" => {
                "```typescript\nfunction accept(prompt: string): boolean {\n  // streamed block — highlight when fence closes\n  return prompt.length > 0;\n}\n```"
            }
            "json" => {
                "```json\n{\n  \"accept\": true,\n  \"note\": \"streamed block — highlight when fence closes\"\n}\n```"
            }
            _ => {
                "```rust\nfn accept(prompt: &str) -> bool {\n    // streamed block — highlight when fence closes\n    !prompt.is_empty()\n}\n```"
            }
        };
        (focus, fence)
    }

    fn build_assistant_reply(&mut self, prompt: &str) -> String {
        let p = prompt.to_ascii_lowercase();
        if p.contains("markdown") || p.contains("md showcase") || p.contains("full md") {
            return format!(
                "收到，我已经接住 `{prompt}`。\n\n{}",
                markdown_showcase_seed()
            );
        }

        let (focus, fence) = self.pick_stream_fence(prompt);

        let closing = if self.random_between(0, 1) == 0 {
            "这段回复现在就是用打字机式流式输出。"
        } else {
            "接下来会按流式打字机节奏把结果一点点吐出来。"
        };
        format!(
            "收到，我已经接住 `{prompt}`。\n\n\
             ## 本轮\n\n\
             {focus}\n\n\
             ### 清单\n\n\
             1. 先排查提交路径\n\
             - 再补真实终端 smoke\n\
             - [ ] 终端验收\n\
             - [x] 解析 prompt\n\n\
             详见 [harness notes](https://example.com/harness)。\n\n\
             > 流式过程中未闭合代码块可能尚未高亮；闭合后 syntect 着色。\n\n\
             {fence}\n\n\
             | Step | Status | Note |\n\
             |------|--------|------|\n\
             | parse | ok | — |\n\
             | paint | streaming | no fence chrome |\n\n\
             ~~旧文案~~ 已替换。\n\n\
             {closing}"
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
                    "assistant",
                    tree_label_preview(&text),
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
                TimedAction::SetStatus(text) => self.set_status(text),
                TimedAction::PushScrollNotice(text) => self.push_message(Role::ScrollNotice, text),
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
                    "tool",
                    tree_label_preview(&text),
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
                    "tool",
                    tree_label_preview(&summary),
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
                    "tool",
                    tree_label_preview(&summary),
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
                        "assistant",
                        tree_label_preview(&text),
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
            Role::User => {
                // Palette mauve — matches DESIGN user token (not hardcoded ANSI 35).
                fg_rgb(self.palette().user, self.glyph_set.user())
            }
            Role::Assistant => String::new(),
            Role::ScrollNotice => dim(self.glyph_set.system()),
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

    /// Left layout via package `paint_left_rail_line` (rail + gutter + content).
    fn paint_rail_line(line: &str, width: usize, rgb: xylitol_tui::RgbColor) -> String {
        paint_left_rail_line(line, width, rgb)
    }

    fn push_entry_block(
        &self,
        lines: &mut Vec<String>,
        content: &[String],
        width: usize,
        rgb: xylitol_tui::RgbColor,
    ) {
        match self.entry_style {
            EntryStyle::Wash => {
                let paint_bg = |line: &str| {
                    apply_background_to_line(&Self::fit(line, width), width, &|s| bg_rgb(rgb, s))
                };
                lines.push(paint_bg(""));
                for line in content {
                    lines.push(paint_bg(line));
                }
                lines.push(paint_bg(""));
            }
            EntryStyle::Rail => {
                for line in content {
                    lines.push(Self::paint_rail_line(line, width, rgb));
                }
            }
        }
    }

    /// Status rail: mix surface←status so a 1-cell strip reads clearly without neon wash.
    fn rail_rgb_for_tool(&self, status: ToolBlockStatus) -> xylitol_tui::RgbColor {
        let p = self.palette();
        let vivid = match status {
            ToolBlockStatus::Pending => p.accent,
            ToolBlockStatus::Success => p.success,
            ToolBlockStatus::Error => p.error,
        };
        mix_rgb(p.surface, vivid, 0.72)
    }

    fn transcript_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let g = self.glyph_set;
        let spacer = |w: usize| format!("{}\x1b[49m", " ".repeat(w.max(1)));
        // Rail+gutter = 2 cols for tool/diff/bash. User / assistant / thinking stay flush.
        let rail_inner = match self.entry_style {
            EntryStyle::Rail => width.saturating_sub(2).max(1),
            EntryStyle::Wash => width,
        };
        let mut need_spacer = false;
        for entry in &self.transcript {
            if need_spacer {
                lines.push(spacer(width));
            }
            need_spacer = true;
            match entry {
                TranscriptEntry::Message { role, text } => {
                    if matches!(role, Role::Assistant) {
                        // Flush left — no rail indent (rail on tools only).
                        let mut md = Markdown::new(
                            text.clone(),
                            0,
                            0,
                            demo_markdown_theme(self.theme_mode),
                            None,
                        );
                        for line in md.render(width) {
                            lines.push(Self::fit(&line, width));
                        }
                    } else if matches!(role, Role::ScrollNotice) {
                        let prefix = self.role_prefix(*role);
                        let raw = if prefix.is_empty() {
                            text.clone()
                        } else {
                            format!("{prefix} {text}")
                        };
                        Self::push_wrapped(&mut lines, &raw, width);
                    } else {
                        // User: only ❯ + body — no left bg rail / no extra gutter.
                        let prefix = self.role_prefix(*role);
                        let body = if matches!(role, Role::User) {
                            let p = self.palette();
                            highlight_dollar_skill_refs(text, p.skill_ref)
                        } else {
                            text.clone()
                        };
                        let raw = if prefix.is_empty() {
                            body
                        } else {
                            format!("{prefix} {body}")
                        };
                        match self.entry_style {
                            EntryStyle::Wash => {
                                let content = wrap_text_with_ansi(&raw, width);
                                self.push_entry_block(
                                    &mut lines,
                                    &content,
                                    width,
                                    self.palette().user_message_bg,
                                );
                            }
                            EntryStyle::Rail => {
                                Self::push_wrapped(&mut lines, &raw, width);
                            }
                        }
                    }
                }
                TranscriptEntry::Thinking { expanded, body } => {
                    // Flush like assistant — thinking is content; rail is for tools/bash/diff only.
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
                    // Rail: no ⚙ — product expandable uses Name/path only; wash keeps demo glyph.
                    let header = match self.entry_style {
                        EntryStyle::Rail => {
                            format!("{marker} {summary}  {}", key_hint("Alt+E"))
                        }
                        EntryStyle::Wash => {
                            format!("{marker} {} {summary}  {}", g.tool(), key_hint("Alt+E"))
                        }
                    };
                    let mut block = Vec::new();
                    Self::push_wrapped(&mut block, &header, rail_inner);
                    if *expanded {
                        let from = if summary.starts_with("Read ") {
                            TruncateFrom::Head
                        } else {
                            TruncateFrom::Tail
                        };
                        let opts = ExpandableOutputOptions {
                            max_preview_lines: self.tools_output_max_lines,
                            from,
                            expand_hint: "ctrl+o to expand".into(),
                            hint_style: Some(dim),
                        };
                        for line in render_expandable_output(
                            detail,
                            rail_inner,
                            self.tools_output_expanded,
                            &opts,
                        ) {
                            block.push(line);
                        }
                    }
                    let rgb = match self.entry_style {
                        EntryStyle::Wash => match status {
                            ToolBlockStatus::Pending => self.palette().tool_pending_bg,
                            ToolBlockStatus::Success => self.palette().tool_success_bg,
                            ToolBlockStatus::Error => self.palette().tool_error_bg,
                        },
                        EntryStyle::Rail => self.rail_rgb_for_tool(*status),
                    };
                    self.push_entry_block(&mut lines, &block, width, rgb);
                }
                TranscriptEntry::Diff {
                    expanded,
                    status,
                    summary,
                    input,
                    side_by_side_min_width,
                } => {
                    let marker = if *expanded { g.unfold() } else { g.fold() };
                    let header = match self.entry_style {
                        EntryStyle::Rail => {
                            format!("{marker} {summary}  {}", key_hint("Alt+E"))
                        }
                        EntryStyle::Wash => {
                            format!("{marker} {} {summary}  {}", g.tool(), key_hint("Alt+E"))
                        }
                    };
                    let mut block = Vec::new();
                    Self::push_wrapped(&mut block, &header, rail_inner);
                    let wash_rgb = match status {
                        ToolBlockStatus::Pending => self.palette().tool_pending_bg,
                        ToolBlockStatus::Success => self.palette().tool_success_bg,
                        ToolBlockStatus::Error => self.palette().tool_error_bg,
                    };
                    if *expanded {
                        let theme = demo_diff_theme(self.theme_mode, wash_rgb);
                        let opts = DiffOptions {
                            word_level: true,
                            side_by_side_min_width: *side_by_side_min_width,
                            ..DiffOptions::default()
                        };
                        let rendered = render_diff_lines(input, rail_inner, &theme, &opts);
                        if !rendered.is_empty() {
                            block.push(String::new());
                            for line in rendered {
                                block.push(Self::fit(&line, rail_inner));
                            }
                        }
                    }
                    let rgb = match self.entry_style {
                        EntryStyle::Wash => wash_rgb,
                        EntryStyle::Rail => self.rail_rgb_for_tool(*status),
                    };
                    self.push_entry_block(&mut lines, &block, width, rgb);
                }
                TranscriptEntry::Ask {
                    expanded,
                    summary,
                    detail_lines,
                    phase,
                } => {
                    let marker = if *expanded { g.unfold() } else { g.fold() };
                    let accent = self.palette().accent;
                    let ask = bold(&fg_rgb(accent, "Ask"));
                    let rest = summary.strip_prefix("Ask").unwrap_or(summary);
                    let hint = key_hint("Alt+E");
                    let fixed =
                        visible_width(marker) + 1 + visible_width("Ask") + 2 + visible_width(&hint);
                    let rest_budget = rail_inner.saturating_sub(fixed).max(4);
                    let rest_fit = if visible_width(rest) <= rest_budget {
                        rest.to_string()
                    } else {
                        truncate_to_width(rest, rest_budget, "…", false)
                    };
                    let header = format!("{marker} {ask}{rest_fit}  {}", dim(&hint));
                    let mut block = vec![Self::fit(&header, rail_inner)];
                    if *expanded {
                        for line in detail_lines {
                            block.push(Self::fit(&dim(line), rail_inner));
                        }
                    }
                    let vivid = match phase {
                        AskPhase::Waiting => self.palette().accent,
                        AskPhase::Answered => self.palette().success,
                        AskPhase::Skipped => self.palette().muted,
                    };
                    // Demo honors /entry-style (rail|wash). Product src app: rail only.
                    let rgb = match self.entry_style {
                        EntryStyle::Rail => mix_rgb(self.palette().surface, vivid, 0.72),
                        EntryStyle::Wash => mix_rgb(self.palette().surface, vivid, 0.22),
                    };
                    self.push_entry_block(&mut lines, &block, width, rgb);
                }
            }
        }
        lines
    }

    /// Status / breathing room above the editor (pi `statusContainer`).
    ///
    /// - Busy: `Loader::render` → leading blank + spinner (keep both; do not strip).
    /// - Idle: one blank so input is never flush against transcript.
    fn status_lines(&mut self, width: usize) -> Vec<String> {
        if self.spinner_active() {
            let mut lines: Vec<String> = self
                .loader
                .render(width)
                .into_iter()
                .map(|line| Self::fit(&line, width))
                .collect();
            // Busy: keep spinner; append cue so it still sits above the editor.
            if self.copy_notice_visible() {
                lines.push(Self::fit(&dim("Copied"), width));
            }
            return lines;
        }
        // Idle: reuse the blank status row so ApplicationOwned dock height stays stable.
        if self.copy_notice_visible() {
            vec![Self::fit(&dim("Copied"), width)]
        } else {
            vec![String::new()]
        }
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
        if self.choice_prompt.is_some() {
            return self.render_choice_slot(width);
        }
        if let Some(kind) = self.lib_atom {
            return self.render_lib_atom_slot(width, kind);
        }
        self.sync_editor_border();
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
            dim(
                " Type search · ←→ page · Ctrl/Alt+←→ fold · Shift+L label · Shift+T time · Shift+F fork",
            )
        } else {
            dim(&format!(" Search: {search}"))
        };
        lines.push(Self::fit(&search_line, width));
        lines.push(Self::fit(
            &dim(" Up/Down  Enter travel  Shift+F fork  Esc close/clear  (double Esc)  Ctrl+D/T/U/L/A filter"),
            width,
        ));
        for line in self.tree.render(width) {
            lines.push(Self::fit(&line, width));
        }
        lines
    }

    fn render_palette_slot(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(Self::fit(&bold(" Command Plate"), width));
        lines.push(Self::fit(
            &dim(" Type to filter · Up/Down  Enter run  Esc close"),
            width,
        ));
        let filter_echo = if self.palette_filter.is_empty() {
            format!("{}{}", dim("> "), cyan("█"))
        } else {
            format!("{}{}{}", dim("> "), self.palette_filter, cyan("█"))
        };
        lines.push(Self::fit(&filter_echo, width));
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

    fn render_choice_slot(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let accent = self.palette().accent;
        // Brand on scrollback `Ask · …` only — no duplicate slot caption.
        let rail = match self.entry_style {
            EntryStyle::Rail => Some(accent),
            EntryStyle::Wash => None,
        };
        if let Some(ref mut prompt) = self.choice_prompt {
            prompt.set_rail(rail);
            for line in prompt.render(width) {
                lines.push(Self::fit(&line, width));
            }
        }
        lines
    }

    fn render_lib_atom_slot(&mut self, width: usize, kind: LibAtomKind) -> Vec<String> {
        let mut lines = Vec::new();
        match kind {
            LibAtomKind::TruncatedText => {
                lines.push(Self::fit(&bold(" TruncatedText"), width));
                lines.push(Self::fit(
                    &dim(" single-line · ellipsis · padding · Esc close"),
                    width,
                ));
                let long = "packages/xylitol-tui/examples/agent_demo.rs · very-long-identifier-for-ellipsis";
                let mut full = TruncatedText::new(long.into(), 1, 0);
                for line in full.render(width) {
                    lines.push(Self::fit(&line, width));
                }
                let narrow = width.clamp(12, 36);
                lines.push(Self::fit(&dim(&format!(" @width={narrow}")), width));
                let mut clipped = TruncatedText::new(long.into(), 0, 0);
                for line in clipped.render(narrow) {
                    lines.push(Self::fit(&line, width));
                }
            }
            LibAtomKind::CancellableLoader => {
                lines.push(Self::fit(&bold(" CancellableLoader"), width));
                lines.push(Self::fit(
                    &dim(" Esc → tui.select.cancel → on_abort · Esc closes slot"),
                    width,
                ));
                if let Some(ref mut loader) = self.atom_loader {
                    for line in loader.render(width) {
                        lines.push(Self::fit(&line, width));
                    }
                }
            }
            LibAtomKind::Panel => {
                lines.push(Self::fit(&bold(" Panel (pi Box)"), width));
                lines.push(Self::fit(
                    &dim(" padding_x/y + optional bg on every line · Esc close"),
                    width,
                ));
                if let Some(ref mut panel) = self.atom_panel {
                    for line in panel.render(width) {
                        lines.push(Self::fit(&line, width));
                    }
                }
            }
        }
        lines
    }

    fn queue_strip_lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for text in &self.steer_queue {
            Self::push_wrapped(&mut lines, &dim(&format!("Steering: {text}")), width);
        }
        for text in &self.follow_up_queue {
            Self::push_wrapped(&mut lines, &dim(&format!("Follow-up: {text}")), width);
        }
        if !self.steer_queue.is_empty() || !self.follow_up_queue.is_empty() {
            Self::push_wrapped(
                &mut lines,
                &dim("↳ Alt+Up to edit all queued messages"),
                width,
            );
        }
        lines
    }
}

impl Component for FakeCodingAgentApp {
    fn render(&mut self, width: usize) -> Vec<String> {
        // Minimal stack: transcript → queue strip → status → editor → footer.
        let mut lines = Vec::new();
        lines.extend(self.transcript_lines(width));
        lines.extend(self.queue_strip_lines(width));
        let status = self.status_lines(width);
        let editor = self.render_editor_slot(width);
        // ApplicationOwned dock excludes transcript+queue (c2070 / ptim06).
        self.last_status_rows = status.len();
        self.last_editor_rows = editor.len();
        self.last_dock_rows = status.len().saturating_add(editor.len()).saturating_add(1);
        // Canonical ApplicationOwned editor hit-test origin (ptim14) — same helper as product.
        let (origin_row, origin_col) = xylitol_tui::editor_screen_origin(
            self.term_rows,
            self.last_dock_rows,
            self.last_status_rows,
        );
        self.input.set_screen_origin(origin_row, origin_col);
        lines.extend(status);
        lines.extend(editor);
        let footer_owned;
        let footer_ref = if self.palette_open
            || self.settings_open
            || self.tree_open
            || self.lib_atom.is_some()
            || self.choice_prompt.is_some()
        {
            "esc close · ↑↓ · Enter"
        } else {
            // Compact cue strip — full list is in the seed ScrollNotice.
            // Queue chrome = mid strip only; no footer q:sN|fM badge.
            footer_owned = format!(
                // c535 pad4: metadata only — chords live in /help / plate help-keys.
                "{} · {} · {} · entry:{}",
                self.footer_note,
                self.theme_label(),
                self.glyph_set.label(),
                self.entry_style.label()
            );
            footer_owned.as_str()
        };
        lines.push(Self::fit(&self.muted_paint(footer_ref), width));
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
            InputEvent::Mouse(mouse) => {
                self.handle_editor_mouse(*mouse);
                return;
            }
        };

        if matches_key_event(key, "ctrl+c") {
            self.on_ctrl_c();
            return;
        }

        if matches!(self.lib_atom, Some(LibAtomKind::CancellableLoader))
            && matches_key_event(key, "escape")
        {
            if let Some(ref mut loader) = self.atom_loader {
                loader.handle_input(InputEvent::Key(*key));
                if loader.aborted() {
                    self.push_message(
                        Role::ScrollNotice,
                        "CancellableLoader · on_abort fired (Esc → tui.select.cancel)",
                    );
                    self.close_lib_atom();
                    self.set_status("Ready");
                }
            }
            return;
        }

        if self.choice_prompt.is_some() {
            if let Some(ref mut prompt) = self.choice_prompt {
                prompt.handle_input(event);
            }
            if let Some(result) = self.take_choice_result() {
                self.apply_choice_result(result);
            }
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

        if matches_key_event(key, "ctrl+g") {
            self.request_external_editor();
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
            if matches_key_event(key, "shift+f") {
                let id = self.tree.selected_id().unwrap_or("?").to_string();
                self.fork_from_history(&id);
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
                    let id = item.value.clone();
                    self.palette_open = false;
                    self.palette_filter.clear();
                    self.palette.set_filter("");
                    self.run_demo_plate(&id);
                    // md-full / stream-* schedule their own work; skip idle fallback.
                    if !matches!(
                        id.as_str(),
                        "md-full"
                            | "stream-rust"
                            | "stream-python"
                            | "stream-typescript"
                            | "stream-json"
                            | "tree"
                            | "narrow-clamp"
                            | "truncated-text"
                            | "cancellable-loader"
                            | "panel"
                            | "ask-single"
                            | "ask-multi"
                            | "ask-tabs"
                            | "ask-tool"
                            | "md-list-wrap"
                            | "theme-toggle"
                            | "thinking-level"
                    ) {
                        self.advance_script();
                    }
                } else {
                    self.palette_open = false;
                    self.palette_filter.clear();
                    self.palette.set_filter("");
                }
            } else if matches_key_event(key, "backspace") {
                self.palette_filter.pop();
                self.palette.set_filter(&self.palette_filter);
            } else if let Some(text) = printable_from_key_event(key) {
                self.palette_filter.push_str(&text);
                self.palette.set_filter(&self.palette_filter);
            }
            return;
        }

        if self.settings_open {
            // Forward all keys (search printable, backspace, arrows, Enter/Space,
            // Esc via SettingsList cancel) — not only up/down/enter. Otherwise
            // typing a filter like `zzz` is swallowed and the list never empties.
            self.settings.handle_input(event);
            return;
        }

        if self.lib_atom.is_some() {
            // Atoms own the editor slot; printable must not leak into Editor.
            return;
        }

        if matches_key_event(key, "ctrl+p") {
            self.palette_open = true;
            self.settings_open = false;
            self.close_lib_atom();
            self.close_choice_prompt();
            self.palette_filter.clear();
            self.palette.set_filter("");
            return;
        }
        if matches_key_event(key, "ctrl+s") {
            self.settings_open = true;
            self.palette_open = false;
            self.close_lib_atom();
            self.close_choice_prompt();
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
        // Shift+Tab — pi `app.thinking.cycle` (product will wire in c1150; demo first).
        if matches_key_event(key, "shift+tab") {
            self.cycle_thinking_border_level();
            return;
        }
        if matches_key_event(key, "ctrl+t") {
            self.toggle_thinking_blocks();
            return;
        }
        // Alt+E / Alt+G — not Ctrl+E (editor cursorLineEnd) or Ctrl+G (external
        // editor stub). App-level toggles stay off the Editor keybinding table.
        if matches_key_event(key, "alt+e") {
            self.toggle_tool_blocks();
            return;
        }
        if matches_key_event(key, "alt+g") {
            self.cycle_glyph_set();
            return;
        }
        if matches_key_event(key, "alt+k") {
            self.demo_compaction_status();
            return;
        }
        if matches_key_event(key, "alt+y") {
            self.demo_retry_status();
            return;
        }

        self.input.handle_input(event);
        let submitted = { self.submit_slot.borrow_mut().take() };
        if let Some(text) = submitted {
            self.process_submit(text);
        }
    }

    fn invalidate(&mut self) {}

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        self.input.input_wants_rerender(event)
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        self.take_editor_clipboard()
    }

    fn dock_rows_hint(&self) -> Option<usize> {
        Some(self.last_dock_rows())
    }

    fn wants_pointer_motion(&self) -> bool {
        self.input.is_selection_dragging()
    }

    fn clear_pointer_selection(&mut self) -> bool {
        Component::clear_pointer_selection(&mut self.input)
    }

    fn tick(&mut self) -> bool {
        let mut changed = false;
        self.script_tick = self.script_tick.saturating_add(1);

        if let Some(until) = self.copy_notice_until
            && Instant::now() >= until
        {
            self.copy_notice_until = None;
            changed = true;
        }

        changed |= self.input.tick();

        if self.spinner_active()
            && self.last_tick_at.elapsed().as_millis() >= self.loader.interval_ms() as u128
        {
            self.loader.tick();
            self.last_tick_at = Instant::now();
            changed = true;
        }

        if let Some(ref mut loader) = self.atom_loader
            && self.last_tick_at.elapsed().as_millis() >= loader.interval_ms() as u128
        {
            loader.tick();
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
