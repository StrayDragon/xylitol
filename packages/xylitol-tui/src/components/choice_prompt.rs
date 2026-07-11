//! ChoicePrompt — AskQuestion-style single/multi choice with optional Other + multi-question tabs.
//!
//! Inline Component (editor-slot friendly). Not an overlay dashboard.

use std::collections::HashMap;

use crate::components::input::Input;
use crate::keybindings::with_keybindings;
use crate::keys::printable_from_key_event;
use crate::tui::{Component, InputEvent};
use crate::utils::{truncate_to_width, visible_width};
use crossterm::event::KeyCode;

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
    pub description: Option<String>,
}

impl ChoiceOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
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

#[derive(Clone, Debug)]
pub struct ChoiceResult {
    pub answers: Vec<ChoiceAnswer>,
    pub cancelled: bool,
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
}

impl Default for ChoicePromptTheme {
    fn default() -> Self {
        Self {
            title: Box::new(|s| format!("\x1b[1m{s}\x1b[22m")),
            prompt: Box::new(|s| s.to_string()),
            selected: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
            normal: Box::new(|s| s.to_string()),
            muted: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            tab_active: Box::new(|s| format!("\x1b[36m{s}\x1b[39m")),
            tab_idle: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            hint: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
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
        }
    }

    pub fn finished(&self) -> bool {
        self.finished
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

    fn finish(&mut self, cancelled: bool) {
        if self.finished {
            return;
        }
        self.finished = true;
        let mut answers: Vec<ChoiceAnswer> = self
            .questions
            .iter()
            .filter_map(|q| self.answers.get(&q.id).cloned())
            .collect();
        if cancelled {
            answers.clear();
        }
        (self.on_done)(ChoiceResult { answers, cancelled });
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

    fn save_current_answer(&mut self) {
        let Some(q) = self.questions.get(self.tab).cloned() else {
            return;
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
            return;
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
}

impl Component for ChoicePrompt {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.finished {
            return lines;
        }

        if self.is_multi_questionnaire() {
            let mut tabs = String::new();
            for (i, q) in self.questions.iter().enumerate() {
                let answered = self.answers.contains_key(&q.id);
                let mode_tag = match q.mode {
                    ChoiceMode::Single => "",
                    ChoiceMode::Multi => "+",
                };
                let check = if answered { "✓" } else { "" };
                let label = format!("[{}{}{}]", q.label, mode_tag, check);
                let painted = if i == self.tab {
                    (self.theme.tab_active)(&label)
                } else {
                    (self.theme.tab_idle)(&label)
                };
                if i > 0 {
                    tabs.push_str(" · ");
                }
                tabs.push_str(&painted);
            }
            let submit = if self.on_submit_tab() {
                "[Submit]"
            } else {
                "Submit"
            };
            tabs.push_str(" · ");
            tabs.push_str(&if self.on_submit_tab() {
                (self.theme.tab_active)(submit)
            } else {
                (self.theme.tab_idle)(submit)
            });
            lines.push(Self::fit(&tabs, width));
            lines.push(String::new());
        }

        if self.on_submit_tab() {
            lines.push(Self::fit(&(self.theme.title)(" Review answers"), width));
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
                lines.push(Self::fit(&(self.theme.normal)(&row), width));
            }
            lines.push(Self::fit(
                &(self.theme.hint)(" Enter 提交 · Esc 取消 · ← 回上一题"),
                width,
            ));
            return lines;
        }

        let Some(q) = self.current_question() else {
            return lines;
        };
        let mode = q.mode;
        let allow_other = q.allow_other;
        let options = q.options.clone();
        let prompt = q.prompt.clone();
        let mode_hint = match mode {
            ChoiceMode::Single => "单选",
            ChoiceMode::Multi => "多选",
        };
        let muted_hint = (self.theme.muted)(&format!("· {mode_hint}"));
        let prompt_line = format!("{prompt}  {muted_hint}");
        lines.push(Self::fit(&(self.theme.prompt)(&prompt_line), width));

        for (i, opt) in options.iter().enumerate() {
            let selected = self.cursor == i;
            let body = match mode {
                // SelectList-style: arrow + label only — no ●/○ radios.
                ChoiceMode::Single => {
                    let prefix = if selected { "→ " } else { "  " };
                    format!("{prefix}{}", opt.label)
                }
                ChoiceMode::Multi => {
                    let mark = if self.states[self.tab]
                        .checked
                        .get(i)
                        .copied()
                        .unwrap_or(false)
                    {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let prefix = if selected { "→ " } else { "  " };
                    format!("{prefix}{mark} {}", opt.label)
                }
            };
            let painted = if selected {
                (self.theme.selected)(&body)
            } else {
                (self.theme.normal)(&body)
            };
            lines.push(Self::fit(&painted, width));
            if let Some(ref desc) = opt.description {
                let indent = match mode {
                    ChoiceMode::Single => "    ",
                    ChoiceMode::Multi => "       ",
                };
                lines.push(Self::fit(
                    &(self.theme.muted)(&format!("{indent}{desc}")),
                    width,
                ));
            }
        }

        if allow_other {
            let other_i = options.len();
            let selected = self.cursor == other_i;
            let body = match mode {
                ChoiceMode::Single => {
                    let prefix = if selected { "→ " } else { "  " };
                    let pencil = if self.other_focused { " ✎" } else { "" };
                    format!("{prefix}Other…{pencil}")
                }
                ChoiceMode::Multi => {
                    let has = !self.states[self.tab].other_text.trim().is_empty()
                        || (self.other_focused && !self.other_input.value().trim().is_empty());
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
            lines.push(Self::fit(&painted, width));

            if self.other_focused {
                for line in self.other_input.render(width.saturating_sub(4).max(1)) {
                    lines.push(Self::fit(&format!("    {line}"), width));
                }
            } else if !self.states[self.tab].other_text.is_empty() {
                lines.push(Self::fit(
                    &(self.theme.muted)(&format!("    > {}", self.states[self.tab].other_text)),
                    width,
                ));
            } else if selected {
                lines.push(Self::fit(
                    &(self.theme.muted)("    Tab · type free text"),
                    width,
                ));
            }
        }

        let hint = match mode {
            ChoiceMode::Single => " ↑↓ · Enter · Tab→Other · Esc",
            ChoiceMode::Multi => " ↑↓ · Space · Enter · Tab→Other · Esc",
        };
        lines.push(Self::fit(&(self.theme.hint)(hint), width));
        lines
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
            || matches!(key.code, KeyCode::Esc);
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
                self.save_current_answer();
                self.advance_after_answer();
                return;
            }
            self.other_input.handle_input(InputEvent::Key(*key));
            return;
        }

        if self.on_submit_tab() {
            if confirm {
                self.finish(false);
            } else if left || up {
                self.tab = self.questions.len().saturating_sub(1);
                self.cursor = 0;
            }
            return;
        }

        // Tab bar navigation (multi-q): Left/Right switch tabs when not editing.
        if self.is_multi_questionnaire() && (left || right) {
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
                    self.save_current_answer();
                    self.advance_after_answer();
                }
                ChoiceMode::Multi => {
                    if Some(self.cursor) == self.other_row(&q)
                        && self.states[self.tab].other_text.trim().is_empty()
                    {
                        self.focus_other();
                        return;
                    }
                    self.save_current_answer();
                    self.advance_after_answer();
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
        assert!(r.answers.is_empty());
    }

    #[test]
    fn multi_question_shows_tabs_single_does_not() {
        let mut single =
            ChoicePrompt::new(vec![sample_single()], ChoicePromptTheme::default(), |_| {});
        let lines = single.render(80);
        assert!(!lines.iter().any(|l| l.contains("Submit")));

        let mut multi = ChoicePrompt::new(
            vec![sample_single(), sample_multi()],
            ChoicePromptTheme::default(),
            |_| {},
        );
        let lines = multi.render(80);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Submit") || l.contains("Scope"))
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
}
