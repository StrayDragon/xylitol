//! ChoicePrompt — AskQuestion-style single/multi choice with optional Other + multi-question tabs.
//!
//! Inline Component (editor-slot friendly). Not an overlay dashboard.

use std::collections::HashMap;

use crate::components::input::Input;
use crate::keybindings::with_keybindings;
use crate::keys::{matches_key_event, printable_from_key_event};
use crate::terminal_colors::RgbColor;
use crate::theme::paint_left_rail_line;
use crate::tui::{Component, InputEvent};
use crate::utils::{truncate_to_width, visible_width};
use crossterm::event::KeyCode;

/// Min total width before option list + right description column (Claude-like).
const SIDE_DESC_MIN_WIDTH: usize = 88;

/// Per-question selection semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceMode {
    /// Mutually exclusive; cursor row is the pending answer.
    Single,
    /// Space toggles checkmarks; Enter submits checked set.
    Multi,
}

#[derive(Clone, Debug)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
    /// Optional easy explanation / example for humans (Claude-style).
    /// Agent/tool payloads MAY omit this; UI still works with label-only options.
    pub description: Option<String>,
    /// Optional recommended badge (accent chip).
    pub recommended: bool,
}

impl ChoiceOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            recommended: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

#[derive(Clone, Debug)]
pub struct ChoiceQuestion {
    pub id: String,
    /// Short tab label (multi-question).
    pub label: String,
    pub prompt: String,
    pub mode: ChoiceMode,
    pub options: Vec<ChoiceOption>,
    pub allow_other: bool,
}

#[derive(Clone, Debug)]
pub struct ChoiceAnswer {
    pub question_id: String,
    pub values: Vec<String>,
    pub labels: Vec<String>,
    pub was_custom: bool,
}

/// How the prompt closed (ask-tool: skip is a successful structured outcome).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceStatus {
    Answered,
    /// User skipped (Esc / Skip chrome). Maps to tool `{ "status": "skipped" }`.
    Skipped,
}

#[derive(Clone, Debug)]
pub struct ChoiceResult {
    pub answers: Vec<ChoiceAnswer>,
    /// Legacy alias for Trust gate: `true` iff [`Self::status`] is [`ChoiceStatus::Skipped`].
    pub cancelled: bool,
    pub status: ChoiceStatus,
}

impl ChoiceResult {
    pub fn is_skipped(&self) -> bool {
        self.status == ChoiceStatus::Skipped
    }

    /// Compact JSON-ish payload for the future builtin `ask` tool (demo / product).
    pub fn to_ask_payload_json(&self) -> String {
        match self.status {
            ChoiceStatus::Skipped => r#"{"status":"skipped","answers":[]}"#.to_string(),
            ChoiceStatus::Answered => {
                let parts: Vec<String> = self
                    .answers
                    .iter()
                    .map(|a| {
                        let values = a
                            .values
                            .iter()
                            .map(|v| format!("\"{}\"", escape_json_str(v)))
                            .collect::<Vec<_>>()
                            .join(",");
                        let labels = a
                            .labels
                            .iter()
                            .map(|v| format!("\"{}\"", escape_json_str(v)))
                            .collect::<Vec<_>>()
                            .join(",");
                        format!(
                            "{{\"id\":\"{}\",\"values\":[{values}],\"labels\":[{labels}],\"was_custom\":{}}}",
                            escape_json_str(&a.question_id),
                            if a.was_custom { "true" } else { "false" }
                        )
                    })
                    .collect();
                format!(
                    "{{\"status\":\"answered\",\"answers\":[{}]}}",
                    parts.join(",")
                )
            }
        }
    }

    /// Human-facing one-line summary for TUI scrollback (no JSON).
    pub fn human_summary_line(&self) -> String {
        match self.status {
            ChoiceStatus::Skipped => "Ask · 已跳过 · 按已有信息继续".into(),
            ChoiceStatus::Answered => {
                if self.answers.is_empty() {
                    return "Ask · 已答".into();
                }
                if self.answers.len() == 1 {
                    let a = &self.answers[0];
                    let labels = a.labels.join(", ");
                    if labels.is_empty() {
                        "Ask · 已选".into()
                    } else {
                        format!("Ask · 已选  {labels}")
                    }
                } else {
                    let parts: Vec<String> = self
                        .answers
                        .iter()
                        .map(|a| {
                            let labels = a.labels.join(", ");
                            if labels.is_empty() {
                                a.question_id.clone()
                            } else {
                                format!("{}: {labels}", a.question_id)
                            }
                        })
                        .collect();
                    format!("Ask · {}", parts.join(" · "))
                }
            }
        }
    }

    /// Expandable detail rows (labels only; no raw JSON).
    pub fn human_detail_lines(&self) -> Vec<String> {
        match self.status {
            ChoiceStatus::Skipped => vec!["（用户跳过本题）".into()],
            ChoiceStatus::Answered => self
                .answers
                .iter()
                .map(|a| {
                    let labels = a.labels.join(", ");
                    if labels.is_empty() {
                        format!("{} → （空）", a.question_id)
                    } else {
                        format!("{} → {labels}", a.question_id)
                    }
                })
                .collect(),
        }
    }
}

fn escape_json_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub struct ChoicePromptTheme {
    pub title: Box<dyn Fn(&str) -> String>,
    pub prompt: Box<dyn Fn(&str) -> String>,
    pub selected: Box<dyn Fn(&str) -> String>,
    pub normal: Box<dyn Fn(&str) -> String>,
    pub muted: Box<dyn Fn(&str) -> String>,
    pub tab_active: Box<dyn Fn(&str) -> String>,
    pub tab_idle: Box<dyn Fn(&str) -> String>,
    pub hint: Box<dyn Fn(&str) -> String>,
    /// When set, every rendered line gets a fixed left status rail + gutter.
    pub rail: Option<RgbColor>,
}

impl Default for ChoicePromptTheme {
    fn default() -> Self {
        Self {
            title: Box::new(|s| format!("\x1b[1m{s}\x1b[22m")),
            prompt: Box::new(|s| s.to_string()),
            selected: Box::new(|s| format!("\x1b[1m\x1b[36m{s}\x1b[39m\x1b[22m")),
            normal: Box::new(|s| s.to_string()),
            muted: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            tab_active: Box::new(|s| format!("\x1b[36m{s}\x1b[39m")),
            tab_idle: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            hint: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            rail: None,
        }
    }
}

struct QuestionState {
    /// Multi: checked option indices (not including Other).
    checked: Vec<bool>,
    /// Single: selected option index (or Other index).
    single_idx: usize,
    other_text: String,
}

/// Ask-style prompt: single/multi per question, optional Other, multi-question tabs.
pub struct ChoicePrompt {
    questions: Vec<ChoiceQuestion>,
    theme: ChoicePromptTheme,
    /// 0..questions.len()-1 = question tabs; questions.len() = Submit (multi-q only).
    tab: usize,
    cursor: usize,
    other_focused: bool,
    other_input: Input,
    states: Vec<QuestionState>,
    answers: HashMap<String, ChoiceAnswer>,
    on_done: Box<dyn Fn(ChoiceResult)>,
    finished: bool,
    /// Review: first Enter with unanswered tabs arms skip; second Enter confirms skip.
    skip_confirm_pending: bool,
}

impl ChoicePrompt {
    pub fn new(
        questions: Vec<ChoiceQuestion>,
        theme: ChoicePromptTheme,
        on_done: impl Fn(ChoiceResult) + 'static,
    ) -> Self {
        assert!(!questions.is_empty(), "ChoicePrompt needs ≥1 question");
        let states = questions
            .iter()
            .map(|q| QuestionState {
                checked: vec![false; q.options.len()],
                single_idx: 0,
                other_text: String::new(),
            })
            .collect();
        Self {
            questions,
            theme,
            tab: 0,
            cursor: 0,
            other_focused: false,
            other_input: Input::new(),
            states,
            answers: HashMap::new(),
            on_done: Box::new(on_done),
            finished: false,
            skip_confirm_pending: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Demo / host may toggle left rail vs flush (product Ask stays rail-on).
    pub fn set_rail(&mut self, rail: Option<RgbColor>) {
        self.theme.rail = rail;
    }

    fn is_multi_questionnaire(&self) -> bool {
        self.questions.len() > 1
    }

    fn on_submit_tab(&self) -> bool {
        self.is_multi_questionnaire() && self.tab >= self.questions.len()
    }

    fn current_question(&self) -> Option<&ChoiceQuestion> {
        self.questions.get(self.tab)
    }

    fn row_count(&self, q: &ChoiceQuestion) -> usize {
        q.options.len() + usize::from(q.allow_other)
    }

    fn other_row(&self, q: &ChoiceQuestion) -> Option<usize> {
        q.allow_other.then_some(q.options.len())
    }

    fn clamp_cursor(&mut self) {
        if self.on_submit_tab() {
            self.cursor = 0;
            return;
        }
        let Some(q) = self.current_question() else {
            return;
        };
        let n = self.row_count(q);
        if n == 0 {
            self.cursor = 0;
        } else {
            self.cursor = self.cursor.min(n - 1);
        }
    }

    fn finish(&mut self, skipped: bool) {
        if self.finished {
            return;
        }
        self.finished = true;
        let mut answers: Vec<ChoiceAnswer> = self
            .questions
            .iter()
            .filter_map(|q| self.answers.get(&q.id).cloned())
            .collect();
        let (status, cancelled) = if skipped {
            answers.clear();
            (ChoiceStatus::Skipped, true)
        } else {
            (ChoiceStatus::Answered, false)
        };
        (self.on_done)(ChoiceResult {
            answers,
            cancelled,
            status,
        });
    }

    fn blur_other(&mut self) {
        if self.other_focused {
            let text = self.other_input.value().to_string();
            if let Some(st) = self.states.get_mut(self.tab) {
                st.other_text = text;
            }
            self.other_focused = false;
        }
    }

    fn focus_other(&mut self) {
        let Some(q) = self.current_question() else {
            return;
        };
        if !q.allow_other {
            return;
        }
        self.cursor = q.options.len();
        let existing = self
            .states
            .get(self.tab)
            .map(|s| s.other_text.clone())
            .unwrap_or_default();
        self.other_input = Input::new();
        if !existing.is_empty() {
            // Input has no set_value in all versions — type via assigning if available
            self.other_input = Input::with_value(existing);
        }
        self.other_focused = true;
    }

    /// Returns `true` when an answer was stored.
    fn save_current_answer(&mut self) -> bool {
        let Some(q) = self.questions.get(self.tab).cloned() else {
            return false;
        };
        let st = &self.states[self.tab];
        let mut values = Vec::new();
        let mut labels = Vec::new();
        let mut was_custom = false;

        match q.mode {
            ChoiceMode::Single => {
                if Some(st.single_idx) == self.other_row(&q) {
                    let text = if self.other_focused {
                        self.other_input.value().to_string()
                    } else {
                        st.other_text.clone()
                    };
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        values.push(text.clone());
                        labels.push(text);
                        was_custom = true;
                    }
                } else if let Some(opt) = q.options.get(st.single_idx) {
                    values.push(opt.value.clone());
                    labels.push(opt.label.clone());
                }
            }
            ChoiceMode::Multi => {
                for (i, opt) in q.options.iter().enumerate() {
                    if st.checked.get(i).copied().unwrap_or(false) {
                        values.push(opt.value.clone());
                        labels.push(opt.label.clone());
                    }
                }
                let text = if self.other_focused {
                    self.other_input.value().to_string()
                } else {
                    st.other_text.clone()
                };
                let text = text.trim().to_string();
                if q.allow_other && !text.is_empty() {
                    values.push(text.clone());
                    labels.push(text);
                    was_custom = true;
                }
            }
        }

        if values.is_empty() {
            return false;
        }
        self.answers.insert(
            q.id.clone(),
            ChoiceAnswer {
                question_id: q.id,
                values,
                labels,
                was_custom,
            },
        );
        true
    }

    fn advance_after_answer(&mut self) {
        self.blur_other();
        if !self.is_multi_questionnaire() {
            self.finish(false);
            return;
        }
        if self.tab + 1 < self.questions.len() {
            self.tab += 1;
            self.cursor = 0;
        } else {
            self.tab = self.questions.len(); // Submit
            self.cursor = 0;
        }
    }

    fn fit(line: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        if visible_width(line) <= width {
            line.to_string()
        } else {
            truncate_to_width(line, width, "...", false)
        }
    }

    fn unanswered_labels(&self) -> Vec<String> {
        self.questions
            .iter()
            .filter(|q| !self.answers.contains_key(&q.id))
            .map(|q| q.label.clone())
            .collect()
    }

    fn chrome_hint(&self, on_review: bool, multi: bool, allow_other: bool) -> String {
        if on_review {
            if self.skip_confirm_pending {
                return "再按 Enter 确认 Skip · ← 回去填写 · Esc skip".into();
            }
            return "Enter 提交 · ← 回上一题修正 · Esc skip".into();
        }
        let mut parts = Vec::new();
        if self.is_multi_questionnaire() {
            parts.push("←→ 切题");
        }
        parts.push("↑↓");
        if multi {
            parts.push("Space");
        }
        parts.push("Enter");
        if allow_other {
            parts.push("Tab→Other");
        }
        parts.push("Esc skip");
        parts.join(" · ")
    }

    fn inner_width(&self, width: usize) -> usize {
        if self.theme.rail.is_some() {
            width.saturating_sub(2).max(1)
        } else {
            width.max(1)
        }
    }

    fn with_rail(&self, lines: Vec<String>, width: usize) -> Vec<String> {
        match self.theme.rail {
            Some(rgb) => lines
                .into_iter()
                .map(|l| paint_left_rail_line(&l, width, rgb))
                .collect(),
            None => lines,
        }
    }

    fn format_option_row(
        &self,
        mode: ChoiceMode,
        selected: bool,
        checked: bool,
        label: &str,
        recommended: bool,
    ) -> String {
        let rec = if recommended { " · 推荐" } else { "" };
        let body = match mode {
            ChoiceMode::Single => {
                let prefix = if selected { "→ " } else { "  " };
                format!("{prefix}{label}{rec}")
            }
            ChoiceMode::Multi => {
                let box_ = if checked { "[x]" } else { "[ ]" };
                let prefix = if selected { "→ " } else { "  " };
                format!("{prefix}{box_} {label}{rec}")
            }
        };
        if selected {
            (self.theme.selected)(&body)
        } else {
            (self.theme.normal)(&body)
        }
    }
}

fn wrap_plain(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
            continue;
        }
        let trial = format!("{cur} {word}");
        if visible_width(&trial) <= width {
            cur = trial;
        } else {
            out.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

impl Component for ChoicePrompt {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.finished {
            return lines;
        }
        let outer_w = width.max(1);
        let cw = self.inner_width(outer_w);

        if self.is_multi_questionnaire() {
            let mut tabs = String::new();
            for (i, q) in self.questions.iter().enumerate() {
                let answered = self.answers.contains_key(&q.id);
                let mode_tag = match q.mode {
                    ChoiceMode::Single => "",
                    ChoiceMode::Multi => "+",
                };
                let check = if answered { " ✓" } else { "" };
                let label = format!("{}{}{}", q.label, mode_tag, check);
                let painted = if i == self.tab {
                    (self.theme.tab_active)(&label)
                } else if answered {
                    (self.theme.normal)(&label)
                } else {
                    (self.theme.tab_idle)(&label)
                };
                if i > 0 {
                    tabs.push_str(" · ");
                }
                tabs.push_str(&painted);
            }
            let review = "Review";
            tabs.push_str(" · ");
            tabs.push_str(&if self.on_submit_tab() {
                (self.theme.tab_active)(review)
            } else {
                (self.theme.tab_idle)(review)
            });
            lines.push(Self::fit(&tabs, cw));
            lines.push(String::new());
        }

        if self.on_submit_tab() {
            lines.push(Self::fit(&(self.theme.title)(" 核对后提交"), cw));
            for q in &self.questions {
                let mode_tag = match q.mode {
                    ChoiceMode::Single => "单选",
                    ChoiceMode::Multi => "多选",
                };
                let summary = match self.answers.get(&q.id) {
                    Some(a) => a.labels.join(", "),
                    None => "(未答)".into(),
                };
                let row = format!("  {} ({mode_tag}): {summary}", q.label);
                lines.push(Self::fit(&(self.theme.normal)(&row), cw));
            }
            if self.skip_confirm_pending {
                let labels = self.unanswered_labels().join(" · ");
                let warn = if labels.is_empty() {
                    " 有 tab 未填写".to_string()
                } else {
                    format!(" 有 tab 未填写（{labels}）")
                };
                lines.push(String::new());
                lines.push(Self::fit(&(self.theme.selected)(&warn), cw));
            }
            lines.push(Self::fit(
                &(self.theme.hint)(&self.chrome_hint(true, false, false)),
                cw,
            ));
            return self.with_rail(lines, outer_w);
        }

        let Some(q) = self.current_question() else {
            return self.with_rail(lines, outer_w);
        };
        let mode = q.mode;
        let allow_other = q.allow_other;
        let options = q.options.clone();
        let prompt = q.prompt.clone();
        let mode_hint = match mode {
            ChoiceMode::Single => "单选",
            ChoiceMode::Multi => "多选",
        };
        let progress = if self.is_multi_questionnaire() {
            format!("{} / {}  · ", self.tab + 1, self.questions.len())
        } else {
            String::new()
        };
        let muted_hint = (self.theme.muted)(&format!("· {mode_hint}"));
        let prompt_rows: Vec<&str> = prompt.lines().collect();
        if prompt_rows.is_empty() {
            lines.push(Self::fit(
                &(self.theme.muted)(&format!("{progress}{muted_hint}")),
                cw,
            ));
        } else {
            for (i, row) in prompt_rows.iter().enumerate() {
                if row.is_empty() {
                    lines.push(String::new());
                    continue;
                }
                let painted = if i == 0 {
                    let head = format!("{progress}{row}  {muted_hint}");
                    (self.theme.prompt)(&head)
                } else {
                    (self.theme.prompt)(row)
                };
                lines.push(Self::fit(&painted, cw));
            }
        }
        // Breathing room between prompt and option list (product Ask density).
        lines.push(String::new());

        let any_desc = options
            .iter()
            .any(|o| o.description.as_ref().is_some_and(|d| !d.is_empty()));
        let side = cw >= SIDE_DESC_MIN_WIDTH && any_desc;
        let left_w = if side {
            (cw * 55 / 100).clamp(28, cw.saturating_sub(22))
        } else {
            cw
        };
        let right_w = if side {
            cw.saturating_sub(left_w + 1).max(12)
        } else {
            0
        };

        let mut left_lines: Vec<String> = Vec::new();
        let mut focus_desc: Option<String> = None;
        for (i, opt) in options.iter().enumerate() {
            let selected = self.cursor == i;
            let checked = self.states[self.tab]
                .checked
                .get(i)
                .copied()
                .unwrap_or(false);
            left_lines.push(Self::fit(
                &self.format_option_row(mode, selected, checked, &opt.label, opt.recommended),
                left_w,
            ));
            if selected {
                focus_desc = opt.description.clone();
            }
            if !side
                && let Some(ref desc) = opt.description
                && selected
            {
                // Narrow: show description only under the focused option (Claude density).
                left_lines.push(Self::fit(
                    &(self.theme.muted)(&format!("    {desc}")),
                    left_w,
                ));
            }
        }

        if allow_other {
            let other_i = options.len();
            let selected = self.cursor == other_i;
            let has = !self.states[self.tab].other_text.trim().is_empty()
                || (self.other_focused && !self.other_input.value().trim().is_empty());
            let body = match mode {
                ChoiceMode::Single => {
                    let prefix = if selected { "→ " } else { "  " };
                    let pencil = if self.other_focused { " ✎" } else { "" };
                    format!("{prefix}Other…{pencil}")
                }
                ChoiceMode::Multi => {
                    let mark = if has { "[x]" } else { "[ ]" };
                    let prefix = if selected { "→ " } else { "  " };
                    let pencil = if self.other_focused { " ✎" } else { "" };
                    format!("{prefix}{mark} Other…{pencil}")
                }
            };
            let painted = if selected || self.other_focused {
                (self.theme.selected)(&body)
            } else {
                (self.theme.normal)(&body)
            };
            left_lines.push(Self::fit(&painted, left_w));
            if selected {
                focus_desc = Some("自由补充你的答案".into());
            }
            if self.other_focused {
                for line in self.other_input.render(left_w.saturating_sub(4).max(1)) {
                    left_lines.push(Self::fit(&format!("    {line}"), left_w));
                }
            } else if !self.states[self.tab].other_text.is_empty() {
                left_lines.push(Self::fit(
                    &(self.theme.muted)(&format!("    > {}", self.states[self.tab].other_text)),
                    left_w,
                ));
            } else if selected && !side {
                left_lines.push(Self::fit(
                    &(self.theme.muted)("    Tab · type free text"),
                    left_w,
                ));
            }
        }

        if side {
            let mut right_lines: Vec<String> = Vec::new();
            right_lines.push(Self::fit(&(self.theme.muted)("说明"), right_w));
            if let Some(desc) = focus_desc.filter(|d| !d.is_empty()) {
                for chunk in wrap_plain(&desc, right_w) {
                    right_lines.push(Self::fit(&(self.theme.muted)(&chunk), right_w));
                }
            } else {
                right_lines.push(Self::fit(
                    &(self.theme.muted)("（聚焦选项查看易懂说明）"),
                    right_w,
                ));
            }
            let rows = left_lines.len().max(right_lines.len());
            for i in 0..rows {
                let l = left_lines.get(i).cloned().unwrap_or_default();
                let r = right_lines.get(i).cloned().unwrap_or_default();
                let lpad = if visible_width(&l) < left_w {
                    format!("{l}{}", " ".repeat(left_w - visible_width(&l)))
                } else {
                    l
                };
                lines.push(format!("{lpad} {r}"));
            }
        } else {
            lines.extend(left_lines);
        }

        let chrome = format!(
            "{}{}",
            (self.theme.tab_active)("Skip"),
            (self.theme.hint)(&format!(
                " · {}",
                self.chrome_hint(false, matches!(mode, ChoiceMode::Multi), allow_other)
            ))
        );
        lines.push(Self::fit(&chrome, cw));
        self.with_rail(lines, outer_w)
    }

    fn handle_input(&mut self, event: InputEvent) {
        if self.finished {
            return;
        }
        let InputEvent::Key(ref key) = event else {
            return;
        };

        let up = with_keybindings(|kb| kb.matches_event(key, "tui.select.up"))
            || matches!(key.code, KeyCode::Up);
        let down = with_keybindings(|kb| kb.matches_event(key, "tui.select.down"))
            || matches!(key.code, KeyCode::Down);
        let confirm = with_keybindings(|kb| kb.matches_event(key, "tui.select.confirm"))
            || matches!(key.code, KeyCode::Enter);
        let cancel = with_keybindings(|kb| kb.matches_event(key, "tui.select.cancel"))
            || matches!(key.code, KeyCode::Esc)
            || matches_key_event(key, "ctrl+c");
        let is_tab = matches!(key.code, KeyCode::Tab);
        let is_space = matches!(key.code, KeyCode::Char(' '));
        let left = matches!(key.code, KeyCode::Left);
        let right = matches!(key.code, KeyCode::Right);

        if cancel {
            self.finish(true);
            return;
        }

        if self.other_focused {
            if is_tab {
                self.blur_other();
                return;
            }
            if confirm {
                // Save other text into single selection / multi other field.
                if let Some(q) = self.questions.get(self.tab) {
                    let text = self.other_input.value().to_string();
                    if let Some(st) = self.states.get_mut(self.tab) {
                        st.other_text = text;
                        if q.mode == ChoiceMode::Single {
                            st.single_idx = q.options.len();
                        }
                    }
                }
                self.blur_other();
                if self.save_current_answer() {
                    self.advance_after_answer();
                }
                return;
            }
            self.other_input.handle_input(InputEvent::Key(*key));
            return;
        }

        if self.on_submit_tab() {
            if confirm {
                let unanswered = self.unanswered_labels();
                if unanswered.is_empty() {
                    self.skip_confirm_pending = false;
                    self.finish(false);
                } else if self.skip_confirm_pending {
                    self.finish(true);
                } else {
                    // First Enter: arm skip confirm; second Enter skips.
                    self.skip_confirm_pending = true;
                }
            } else if left || up {
                self.skip_confirm_pending = false;
                self.tab = self.questions.len().saturating_sub(1);
                self.cursor = 0;
            }
            return;
        }

        // Tab bar navigation (multi-q): Left/Right switch tabs when not editing.
        if self.is_multi_questionnaire() && (left || right) {
            self.skip_confirm_pending = false;
            let max = self.questions.len(); // include Submit
            if right {
                self.tab = (self.tab + 1).min(max);
            } else {
                self.tab = self.tab.saturating_sub(1);
            }
            self.cursor = 0;
            self.clamp_cursor();
            return;
        }

        if is_tab {
            if let Some(q) = self.current_question()
                && q.allow_other
            {
                self.focus_other();
            }
            return;
        }

        if up {
            if self.cursor > 0 {
                self.cursor -= 1;
            }
            return;
        }
        if down {
            let n = self
                .current_question()
                .map(|q| self.row_count(q))
                .unwrap_or(0);
            if n > 0 && self.cursor + 1 < n {
                self.cursor += 1;
            }
            return;
        }

        if is_space {
            let Some(q) = self.questions.get(self.tab).cloned() else {
                return;
            };
            if q.mode != ChoiceMode::Multi {
                return;
            }
            if self.cursor < q.options.len() {
                if let Some(st) = self.states.get_mut(self.tab)
                    && let Some(slot) = st.checked.get_mut(self.cursor)
                {
                    *slot = !*slot;
                }
            } else if q.allow_other {
                self.focus_other();
            }
            return;
        }

        if confirm {
            let Some(q) = self.questions.get(self.tab).cloned() else {
                return;
            };
            match q.mode {
                ChoiceMode::Single => {
                    if Some(self.cursor) == self.other_row(&q) {
                        self.focus_other();
                        return;
                    }
                    if let Some(st) = self.states.get_mut(self.tab) {
                        st.single_idx = self.cursor;
                    }
                    if self.save_current_answer() {
                        self.advance_after_answer();
                    }
                }
                ChoiceMode::Multi => {
                    if Some(self.cursor) == self.other_row(&q)
                        && self.states[self.tab].other_text.trim().is_empty()
                    {
                        self.focus_other();
                        return;
                    }
                    if self.save_current_answer() {
                        self.advance_after_answer();
                    }
                }
            }
            return;
        }

        // Ignore stray printable when not in Other focus.
        let _ = printable_from_key_event(key);
    }

    fn invalidate(&mut self) {}
}

// Prefer Input::with_value if present; otherwise fall back.
trait InputWithValue {
    fn with_value(value: String) -> Self;
}

impl InputWithValue for Input {
    fn with_value(value: String) -> Self {
        let mut input = Input::new();
        // Prefer dedicated setter when available.
        #[allow(clippy::needless_late_init)]
        {
            input.set_value(value);
        }
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use std::cell::RefCell;
    use std::rc::Rc;

    fn key(code: KeyCode) -> InputEvent {
        InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn sample_single() -> ChoiceQuestion {
        ChoiceQuestion {
            id: "q1".into(),
            label: "Scope".into(),
            prompt: "What first?".into(),
            mode: ChoiceMode::Single,
            options: vec![
                ChoiceOption::new("bug", "Fix bug"),
                ChoiceOption::new("test", "Add tests"),
            ],
            allow_other: true,
        }
    }

    fn sample_multi() -> ChoiceQuestion {
        ChoiceQuestion {
            id: "q2".into(),
            label: "Checks".into(),
            prompt: "Which checks?".into(),
            mode: ChoiceMode::Multi,
            options: vec![
                ChoiceOption::new("unit", "Unit"),
                ChoiceOption::new("harness", "Harness"),
            ],
            allow_other: false,
        }
    }

    #[test]
    fn single_enter_submits_value() {
        let result = Rc::new(RefCell::new(None));
        let slot = result.clone();
        let mut p = ChoicePrompt::new(
            vec![sample_single()],
            ChoicePromptTheme::default(),
            move |r| {
                *slot.borrow_mut() = Some(r);
            },
        );
        p.handle_input(key(KeyCode::Down));
        p.handle_input(key(KeyCode::Enter));
        let r = result.borrow().clone().expect("done");
        assert!(!r.cancelled);
        assert_eq!(r.status, ChoiceStatus::Answered);
        assert_eq!(r.answers[0].values, vec!["test".to_string()]);
    }

    #[test]
    fn multi_space_then_enter() {
        let result = Rc::new(RefCell::new(None));
        let slot = result.clone();
        let mut p = ChoicePrompt::new(
            vec![sample_multi()],
            ChoicePromptTheme::default(),
            move |r| {
                *slot.borrow_mut() = Some(r);
            },
        );
        p.handle_input(key(KeyCode::Char(' ')));
        p.handle_input(key(KeyCode::Down));
        p.handle_input(key(KeyCode::Char(' ')));
        p.handle_input(key(KeyCode::Enter));
        let r = result.borrow().clone().expect("done");
        assert_eq!(r.answers[0].values.len(), 2);
    }

    #[test]
    fn esc_cancels() {
        let result = Rc::new(RefCell::new(None));
        let slot = result.clone();
        let mut p = ChoicePrompt::new(
            vec![sample_single()],
            ChoicePromptTheme::default(),
            move |r| {
                *slot.borrow_mut() = Some(r);
            },
        );
        p.handle_input(key(KeyCode::Esc));
        let r = result.borrow().clone().expect("done");
        assert!(r.cancelled);
        assert_eq!(r.status, ChoiceStatus::Skipped);
        assert!(r.is_skipped());
        assert!(r.answers.is_empty());
        assert_eq!(
            r.to_ask_payload_json(),
            r#"{"status":"skipped","answers":[]}"#
        );
        assert_eq!(r.human_summary_line(), "Ask · 已跳过 · 按已有信息继续");
    }

    #[test]
    fn multiline_prompt_renders_as_separate_rows() {
        let mut p = ChoicePrompt::new(
            vec![ChoiceQuestion {
                id: "trust".into(),
                label: "Trust".into(),
                prompt: "Trust project folder?\n/home/user/proj\n\nThis allows load.".into(),
                mode: ChoiceMode::Single,
                options: vec![ChoiceOption::new("t", "Trust")],
                allow_other: false,
            }],
            ChoicePromptTheme::default(),
            |_| {},
        );
        let lines = p.render(80);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Trust project folder?"),
            "first prompt row missing: {joined}"
        );
        assert!(
            joined.contains("/home/user/proj"),
            "path row missing: {joined}"
        );
        assert!(
            !joined.contains("Trust project folder?\n/home/user/proj  ·"),
            "must not glue path onto mode-hint line: {joined}"
        );
        // Path should appear as its own rendered row (not only inside a wrap).
        assert!(
            lines.iter().any(|l| l.contains("/home/user/proj")),
            "path must be its own line: {lines:?}"
        );
    }

    #[test]
    fn multi_question_shows_tabs_single_does_not() {
        let mut single =
            ChoicePrompt::new(vec![sample_single()], ChoicePromptTheme::default(), |_| {});
        let lines = single.render(80);
        let text = lines.join("\n");
        assert!(
            !text.contains("Review"),
            "single-q must hide Review tab; got:\n{text}"
        );
        assert!(text.contains("Skip"), "chrome Skip required; got:\n{text}");

        let mut multi = ChoicePrompt::new(
            vec![sample_single(), sample_multi()],
            ChoicePromptTheme::default(),
            |_| {},
        );
        let lines = multi.render(80);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Review") || l.contains("Scope")),
            "multi-q should show Review/Scope tabs"
        );
    }

    #[test]
    fn other_tab_focuses_input() {
        let result = Rc::new(RefCell::new(None));
        let slot = result.clone();
        let mut p = ChoicePrompt::new(
            vec![sample_single()],
            ChoicePromptTheme::default(),
            move |r| {
                *slot.borrow_mut() = Some(r);
            },
        );
        p.handle_input(key(KeyCode::Down));
        p.handle_input(key(KeyCode::Down)); // Other
        p.handle_input(key(KeyCode::Tab));
        assert!(p.other_focused);
        p.handle_input(key(KeyCode::Char('h')));
        p.handle_input(key(KeyCode::Char('i')));
        p.handle_input(key(KeyCode::Enter));
        let r = result.borrow().clone().expect("done");
        assert!(r.answers[0].was_custom);
        assert_eq!(r.answers[0].values, vec!["hi".to_string()]);
    }

    #[test]
    fn single_render_has_no_radio_glyphs() {
        let mut p = ChoicePrompt::new(vec![sample_single()], ChoicePromptTheme::default(), |_| {});
        let text = p.render(80).join("\n");
        assert!(
            !text.contains('●') && !text.contains('○'),
            "Single mode must not use radio glyphs; got:\n{text}"
        );
        assert!(text.contains("→ ") || text.contains("Fix bug"));
    }

    #[test]
    fn mixed_tabs_show_multi_marker() {
        let mut p = ChoicePrompt::new(
            vec![sample_single(), sample_multi()],
            ChoicePromptTheme::default(),
            |_| {},
        );
        let text = p.render(80).join("\n");
        assert!(
            text.contains("Checks+") || text.contains('+'),
            "multi question tab should mark +: {text}"
        );
    }

    #[test]
    fn wide_width_shows_side_description_panel() {
        let mut p = ChoicePrompt::new(
            vec![ChoiceQuestion {
                id: "q".into(),
                label: "Q".into(),
                prompt: "Pick?".into(),
                mode: ChoiceMode::Single,
                options: vec![
                    ChoiceOption::new("a", "Alpha")
                        .with_description("Easy example for Alpha choice"),
                ],
                allow_other: false,
            }],
            ChoicePromptTheme {
                rail: Some(RgbColor {
                    r: 137,
                    g: 180,
                    b: 250,
                }),
                ..ChoicePromptTheme::default()
            },
            |_| {},
        );
        let text = p.render(100).join("\n");
        assert!(
            text.contains("说明") && text.contains("Easy example"),
            "wide Ask should show side 说明 panel; got:\n{text}"
        );
    }

    #[test]
    fn prompt_and_options_have_blank_separator() {
        let mut p = ChoicePrompt::new(vec![sample_single()], ChoicePromptTheme::default(), |_| {});
        let lines = p.render(80);
        let prompt_i = lines
            .iter()
            .position(|l| l.contains("What first"))
            .expect("prompt");
        let opt_i = lines
            .iter()
            .position(|l| l.contains("Fix bug"))
            .expect("option");
        assert!(
            opt_i > prompt_i + 1 && lines[prompt_i + 1].trim().is_empty(),
            "need blank line between prompt and options; lines={lines:?}"
        );
    }

    #[test]
    fn without_descriptions_wide_width_skips_side_panel() {
        let mut p = ChoicePrompt::new(vec![sample_single()], ChoicePromptTheme::default(), |_| {});
        let text = p.render(100).join("\n");
        assert!(
            !text.contains("说明"),
            "label-only options must not open empty 说明 panel; got:\n{text}"
        );
    }

    #[test]
    fn review_unanswered_enter_twice_confirms_skip() {
        let result = Rc::new(RefCell::new(None));
        let slot = result.clone();
        let mut p = ChoicePrompt::new(
            vec![sample_single(), sample_multi()],
            ChoicePromptTheme::default(),
            move |r| {
                *slot.borrow_mut() = Some(r);
            },
        );
        // Jump to Review without answering (→ →).
        p.handle_input(key(KeyCode::Right));
        p.handle_input(key(KeyCode::Right));
        p.handle_input(key(KeyCode::Enter));
        assert!(result.borrow().is_none(), "first Enter must not finish");
        let warn = p.render(80).join("\n");
        let status = warn
            .lines()
            .find(|l| l.contains("有 tab 未填写"))
            .expect("status line");
        assert!(
            !status.contains("再按 Enter"),
            "status must not repeat Enter action; got: {status}"
        );
        assert!(
            warn.contains("再按 Enter 确认 Skip"),
            "chrome hint carries Enter action; got:\n{warn}"
        );
        p.handle_input(key(KeyCode::Enter));
        let r = result.borrow().clone().expect("second Enter skips");
        assert!(r.is_skipped());
        assert_eq!(r.status, ChoiceStatus::Skipped);
    }

    #[test]
    fn review_unanswered_enter_then_back_clears_confirm() {
        let mut p = ChoicePrompt::new(
            vec![sample_single(), sample_multi()],
            ChoicePromptTheme::default(),
            |_| {},
        );
        p.handle_input(key(KeyCode::Right));
        p.handle_input(key(KeyCode::Right));
        p.handle_input(key(KeyCode::Enter));
        let armed = p.render(80).join("\n");
        assert!(
            armed.contains("有 tab 未填写"),
            "first Enter should arm; got:\n{armed}"
        );
        p.handle_input(key(KeyCode::Left));
        let cleared = p.render(80).join("\n");
        assert!(
            !cleared.contains("有 tab 未填写"),
            "← should clear confirm; got:\n{cleared}"
        );
    }
}
