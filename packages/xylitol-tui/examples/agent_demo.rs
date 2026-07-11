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
    AtPathSource, CompletionContext, CompletionMatch, CompletionSource, SlashCommandSource,
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
    CancellableLoader, Component, CrosstermTerminal, DiffInput, DiffOptions, DiffTheme,
    ExpandableOutputOptions, Focusable, Input, InputEvent, InputListenerResult, Markdown,
    MarkdownTheme, Panel, SystemClock, TUI, TerminalColorScheme, Text, ThemeDetectSources,
    TreeNode, TreeSelector, TreeSelectorOptions, TreeSelectorTheme, TruncateFrom, TruncatedText,
    apply_background_to_line, highlight_code, matches_key_event, parse_osc11_background_color,
    printable_from_key_event, render_diff_lines, render_expandable_output,
    resolve_terminal_color_scheme, truncate_to_width, visible_width, wrap_text_with_ansi,
};

/// Demo slash commands (static; product would load from Driver / protocol).
/// Names omit the leading `/` — Editor's CombinedAutocompleteProvider adds it.
const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("help", "Show key help in transcript"),
    ("md", "Stream full Markdown grammar stub (typewriter)"),
    ("model", "Switch execution model"),
    ("compact", "Compact conversation history"),
    ("export", "Export current session"),
    ("session", "Session management"),
    ("settings", "Open settings panel"),
    ("palette", "Open command plate"),
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

/// Command plate row (c535): id drives routing; label/description feed SelectList.
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
        id: "compact",
        label: "Compact conversation",
        description: "Simulate a context compaction checkpoint",
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

fn cyan(s: &str) -> String {
    format!("\x1b[36m{s}\x1b[39m")
}

fn green(s: &str) -> String {
    // DESIGN.md colors.success #a6e3a1
    format!("\x1b[38;2;166;227;161m{s}\x1b[39m")
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
    while let Some(node) = find_session_node(roots, &cur_id) {
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

fn demo_markdown_theme() -> MarkdownTheme {
    // DESIGN tokens (Mocha): accent / on-surface / muted / success / warning
    let accent = |s: &str| format!("\x1b[38;2;137;180;250m{s}\x1b[39m");
    let on_surface = |s: &str| format!("\x1b[38;2;205;214;244m{s}\x1b[39m");
    let muted = |s: &str| format!("\x1b[38;2;108;112;134m{s}\x1b[39m");
    let success = |s: &str| format!("\x1b[38;2;166;227;161m{s}\x1b[39m");
    let warning = |s: &str| format!("\x1b[38;2;249;226;175m{s}\x1b[39m");
    let bold_sgr = |s: &str| format!("\x1b[1m{s}\x1b[22m");
    let italic_sgr = |s: &str| format!("\x1b[3m{s}\x1b[23m");
    let underline_sgr = |s: &str| format!("\x1b[4m{s}\x1b[24m");
    MarkdownTheme {
        // Level colors per design/markdown.md — full style here (not nested with theme.bold).
        heading: Box::new(move |level, s| match level {
            1 | 2 => accent(&bold_sgr(&underline_sgr(s))),
            3 | 4 => on_surface(&bold_sgr(s)),
            _ => muted(s),
        }),
        link: Box::new(accent),
        link_url: Box::new(move |s| underline_sgr(&accent(s))),
        code: Box::new(success),
        code_block: Box::new(|s| s.to_string()),
        // DESIGN / c530: no fence chrome (callback unused)
        code_block_border: Box::new(|_| String::new()),
        quote: Box::new(move |s| muted(&italic_sgr(s))),
        quote_border: Box::new(|_| String::new()),
        hr: Box::new(muted),
        list_bullet: Box::new(|s| s.to_string()),
        // B: color + SGR so emphasis survives terminals that ignore bold/italic weight.
        bold: Box::new(move |s| bold_sgr(&accent(s))),
        italic: Box::new(move |s| italic_sgr(&warning(s))),
        strikethrough: Box::new(move |s| format!("\x1b[9m{}\x1b[29m", muted(s))),
        underline: Box::new(underline_sgr),
        highlight_code: Some(Box::new(highlight_code)),
        code_block_indent: Some("  ".into()),
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

二级同样 accent + underline。

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

> 引用第一行，只用 dim/italic，没有竖线装饰。
>
> 引用第二段仍安静。
>
> 引用里也可以有 **（加粗）词** 与 `code`。

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

窄终端下第 3 项应折行且续行保留与 `3. ` 对齐的空格，**不得**二次折行把悬挂缩进冲掉。"
}

/// Full Markdown grammar for c530 (via `/md` or Ctrl+P → md-full).
fn markdown_showcase_seed() -> &'static str {
    markdown_grammar_stub()
}

/// Richer streamed assistant body used when prompt asks for markdown / md.
fn markdown_showcase_stream_focus() -> &'static str {
    "本轮按 c530 打字机流式铺全语法 stub：标题分级、行内标记、链接/图、列表/任务、引用、表、多语言代码。"
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

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Interactive TTY → real `$EDITOR`; harness / non-TTY / explicit stub → stub.
fn prefer_real_external_editor() -> bool {
    if env_flag("XYLITOL_AGENT_DEMO_EDITOR_STUB") {
        return false;
    }
    if env_flag("XYLITOL_AGENT_DEMO_REAL_EDITOR") {
        return true;
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

    // After Ctrl+G sets pending: suspend terminal → `$EDITOR` → restore (pi shape).
    let app_hook = app.clone();
    tui.set_after_dispatch_hook(move |tui| {
        let pending = app_hook.borrow_mut().take_pending_external_editor();
        if !pending {
            return;
        }
        let text = app_hook.borrow().input_text_for_test();
        let outcome = tui.with_terminal_suspended(|| run_external_editor_process(&text));
        match outcome {
            Ok(Some(new_text)) => {
                app_hook.borrow_mut().apply_external_editor_text(new_text);
            }
            Ok(None) => {
                app_hook.borrow_mut().push_system_for_test(
                    "external editor exited non-zero — keeping original text".to_string(),
                );
            }
            Err(err) => {
                app_hook
                    .borrow_mut()
                    .push_system_for_test(format!("external editor failed: {err}"));
            }
        }
    });

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
    System,
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
    /// Typeahead filter for Command plate (SelectList::set_filter).
    palette_filter: String,
    settings_open: bool,
    settings: SettingsList,
    /// Library-atom editor-slot showcase (TruncatedText / CancellableLoader / Panel / Overlay).
    lib_atom: Option<LibAtomKind>,
    /// Live spinner for `LibAtomKind::CancellableLoader` (Esc aborts).
    atom_loader: Option<CancellableLoader>,
    /// Built once when opening `LibAtomKind::Panel`.
    atom_panel: Option<Panel>,
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

    /// Test helper: replace editor text (does not auto-sync bash border).
    pub fn set_editor_text_for_test(&mut self, text: impl Into<String>) {
        self.input.set_text(text.into());
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

    fn open_lib_atom(&mut self, kind: LibAtomKind) {
        self.palette_open = false;
        self.settings_open = false;
        self.tree_open = false;
        self.tree_label_edit = None;
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
                let mut panel = Panel::new(
                    2,
                    1,
                    Some(Box::new(|s: &str| {
                        // surface-container-ish (DESIGN dark)
                        format!("\x1b[48;2;49;50;68m{s}\x1b[49m")
                    })),
                );
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
        self.push_message(
            Role::System,
            format!("forked @ {id} · path: {path_label} · edit & Enter to branch"),
        );
        for node_id in &path {
            if let Some(entry) = self.history_entry_for(node_id) {
                self.transcript.push(entry);
            }
        }

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

    pub fn fork_from_selected_for_test(&mut self) {
        if let Some(id) = self.tree.selected_id().map(str::to_string) {
            self.fork_from_history(&id);
        }
    }

    /// Child count of a session-tree node (harness — fork creates siblings).
    pub fn session_tree_child_count_for_test(&self, id: &str) -> usize {
        find_session_node(&self.session_tree, id)
            .map(|n| n.children.len())
            .unwrap_or(0)
    }

    /// Harness: submit text as if the editor fired on_submit (bypasses paste-burst).
    pub fn submit_text_for_test(&mut self, text: impl Into<String>) {
        self.process_submit(text.into());
    }

    /// Harness: drive one Component tick (script / streams / queues).
    pub fn tick_for_test(&mut self) -> bool {
        self.tick()
    }

    pub fn bash_mode_for_test(&self) -> bool {
        self.bash_mode
    }

    pub fn history_leaf_for_test(&self) -> &str {
        &self.history_leaf_id
    }

    pub fn external_editor_invocations_for_test(&self) -> u32 {
        self.external_editor_invocations
    }

    pub fn open_external_editor_stub_for_test(&mut self) {
        self.open_external_editor_stub();
    }

    pub fn request_external_editor_for_test(&mut self) {
        self.request_external_editor();
    }

    pub fn take_pending_external_editor(&mut self) -> bool {
        let pending = self.pending_external_editor;
        self.pending_external_editor = false;
        pending
    }

    pub fn apply_external_editor_text(&mut self, text: String) {
        self.input.set_text(text);
        self.sync_editor_border();
        self.push_message(
            Role::System,
            "external editor saved — buffer replaced (Ctrl+G)".to_string(),
        );
    }

    pub fn push_system_for_test(&mut self, text: String) {
        self.push_message(Role::System, text);
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

    pub fn sync_editor_border_for_test(&mut self) {
        self.sync_editor_border();
    }

    pub fn theme_mode_for_test(&self) -> TerminalColorScheme {
        self.theme_mode
    }

    pub fn theme_auto_for_test(&self) -> bool {
        self.theme_auto
    }

    pub fn set_theme_auto_for_test(&mut self, enabled: bool) {
        self.theme_auto = enabled;
        if !enabled {
            self.theme_mode = TerminalColorScheme::Dark;
        }
    }

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
        let color_scheme_report =
            scheme_report.and_then(xylitol_tui::parse_terminal_color_scheme_report);
        self.theme_mode = resolve_terminal_color_scheme(ThemeDetectSources {
            explicit: None,
            osc11_background,
            color_scheme_report,
            colorfgbg,
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

    fn theme_label(&self) -> &'static str {
        match self.theme_mode {
            TerminalColorScheme::Dark => "theme:dark",
            TerminalColorScheme::Light => "theme:light",
        }
    }

    /// Muted chrome color — Latte vs Mocha so auto-detect is visible.
    fn muted_paint(&self, s: &str) -> String {
        let (r, g, b) = match self.theme_mode {
            // DESIGN.md colors.muted (Mocha)
            TerminalColorScheme::Dark => (108u8, 112, 134),
            // Catppuccin Latte overlay1-ish
            TerminalColorScheme::Light => (140u8, 143, 161),
        };
        format!("\x1b[38;2;{r};{g};{b}m{s}\x1b[39m")
    }

    fn sync_editor_border(&mut self) {
        let bash = self.input.get_text().trim_start().starts_with('!');
        if bash == self.bash_mode {
            return;
        }
        self.bash_mode = bash;
        if bash {
            self.input.set_border_color(Box::new(green));
        } else {
            self.input.set_border_color(Box::new(dim));
        }
    }

    /// Ctrl+G: external editor — real `$EDITOR` on TTY via TUI suspend; stub in harness.
    fn open_external_editor_stub(&mut self) {
        self.external_editor_invocations = self.external_editor_invocations.saturating_add(1);
        let text = self.input.get_text();
        self.push_message(
            Role::System,
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
        // Pluggable CompletionSources: `/` + `@` + demo `$` stub (c545 open extension).
        input.set_completion_sources(vec![
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
        };
        if app.theme_auto {
            app.refresh_theme_from_env();
        }
        app.seed_transcript();
        app
    }

    fn seed_transcript(&mut self) {
        // Slim chrome (c535): short pointer + compact kit. Full Markdown → plate `/md`.
        self.push_message(
            Role::System,
            "demo · Ctrl+P plate · /md Markdown stream · /help keys · /diff diffs",
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
            "已就绪。用 **Ctrl+P** 打开 Command plate：全语法 Markdown 打字机、流式高亮、更多 Diff/工具。折叠提示在块旁 `(Ctrl+T)` / `(Alt+E)`。",
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
    }

    fn inject_help_keys(&mut self) {
        self.push_message(
            Role::System,
            "keys: Enter submit/steer · Alt+Enter follow-up · /md Markdown stream · Ctrl+P plate · \
             /help · /diff · ! bash · Ctrl+G $EDITOR · double Esc tree · (Ctrl+T) thinking · \
             (Alt+E) tools · (Ctrl+O) tools viewport · Alt+G glyphs · Esc · Ctrl+C",
        );
        self.push_message(
            Role::System,
            "stream plate: md-full · stream-rust/python/typescript/json · diff-sbs · \
             completion-dollar (c545 $) · expandable-head (c550) · playground-sync (c555) · \
             md-list-wrap · narrow-clamp · truncated-text · cancellable-loader · panel · \
             tree (c560) · tool-tints · help-keys · tests · compact",
        );
        self.set_status("Ready");
    }

    fn inject_completion_dollar_tip(&mut self) {
        self.push_message(Role::User, "plate · completion-dollar · c545");
        self.push_message(
            Role::System,
            "c545: `$skill` is an inline reference (like `@path`), not a line-leading slash. \
             Type e.g. `use $` mid-prompt — popup lists stub skills; Tab inserts `$name` and \
             keeps surrounding text. Narrow terminals clamp popup width. `/` and `@` stay \
             independent. Product skill semantics remain out of scope.",
        );
        self.set_status("Type use $ for stub skills");
    }

    fn inject_expandable_head_showcase(&mut self) {
        self.push_message(Role::User, "plate · expandable-head · c550");
        self.push_message(
            Role::System,
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
            Role::System,
            "c555: DESIGN playground is a human preview shell — Agent defaults ignore \
             `src/app/tui/design/playground/`. Tokens SSOT = DESIGN.md frontmatter → \
             `python3 src/app/tui/design/playground/sync_tokens.py` → tokens.css/js. \
             Markdown slot: no `#` titles, links as `text (url)`, bold/italic via style \
             only (optional （加粗）/（斜体） stubs). Runtime check: `/md` in this demo. \
             Also try plate `md-list-wrap` / `narrow-clamp`, playground slot `widgets` (key 8).",
        );
        self.set_status("Ready · try /md for runtime MD");
    }

    fn inject_md_list_wrap_showcase(&mut self) {
        self.push_message(Role::User, "plate · md-list-wrap · list prewrapped");
        self.push_message(
            Role::System,
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
            Role::System,
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
        self.settings_open = true;
        self.set_status("Settings · type zzz for no-match · Esc closes");
    }

    fn inject_truncated_text_atom(&mut self) {
        self.push_message(Role::User, "plate · truncated-text · atom");
        self.push_message(
            Role::System,
            "Library reference: TruncatedText keeps a single line, pads, and ellipsizes to the \
             width budget. Playground: slot Atoms (key 9). Esc closes this slot.",
        );
        self.open_lib_atom(LibAtomKind::TruncatedText);
    }

    fn inject_cancellable_loader_atom(&mut self) {
        self.push_message(Role::User, "plate · cancellable-loader · atom");
        self.push_message(
            Role::System,
            "Library reference: CancellableLoader ticks like Loader; Esc matches \
             `tui.select.cancel` and fires `on_abort`. Playground: slot Atoms (key 9).",
        );
        self.open_lib_atom(LibAtomKind::CancellableLoader);
    }

    fn inject_panel_atom(&mut self) {
        self.push_message(Role::User, "plate · panel · atom");
        self.push_message(
            Role::System,
            "Library reference: Panel (pi Box) pads children and paints an optional background on \
             every line. Playground: slot Atoms (key 9). Esc closes.",
        );
        self.open_lib_atom(LibAtomKind::Panel);
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
            "tool-tints" => self.inject_tool_tint_showcase(),
            "tree" => {
                self.push_message(Role::User, "plate · tree · c560");
                self.push_message(
                    Role::System,
                    "c560: TreeSelector empty/no-match shows a dim hint (not a blank list). \
                     Filter/search keeps the prior selected id when still visible; otherwise \
                     falls back to the first visible row. Type a nonsense search to see empty; \
                     Ctrl+T cycles demo filters.",
                );
                self.tree_open = true;
                self.palette_open = false;
                self.settings_open = false;
                self.close_lib_atom();
                self.set_status("Session tree · c560 empty/selection");
            }
            "help-keys" => self.inject_help_keys(),
            "tests" => {
                self.pending_events
                    .push_back(ScriptEvent::Tool("cargo test -p xylitol-tui --lib".into()));
                self.pending_events.push_back(ScriptEvent::Assistant(
                    "Regression tests are queued. Next step: rerun the PTY smoke against the primary example.".into(),
                ));
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
                    4..=7 => {
                        if current.is_ascii() {
                            self.random_between(1, 3) as usize
                        } else {
                            1
                        }
                    }
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
                        // Always Markdown so streaming code fences highlight as they close
                        // (source fences; rendered output has no fence chrome — c530).
                        let mut md = Markdown::new(text.clone(), 0, 0, demo_markdown_theme(), None);
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
                let narrow = width.min(36).max(12);
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
        let footer_ref =
            if self.palette_open || self.settings_open || self.tree_open || self.lib_atom.is_some()
            {
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
                    // c535 pad4: metadata only — chords live in /help / plate help-keys.
                    "{} · {} · {}{queue_hint}",
                    self.footer_note,
                    self.theme_label(),
                    self.glyph_set.label()
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
                        Role::System,
                        "CancellableLoader · on_abort fired (Esc → tui.select.cancel)",
                    );
                    self.close_lib_atom();
                    self.set_status("Ready");
                }
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
                            | "md-list-wrap"
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
            self.palette_filter.clear();
            self.palette.set_filter("");
            return;
        }
        if matches_key_event(key, "ctrl+s") {
            self.settings_open = true;
            self.palette_open = false;
            self.close_lib_atom();
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
