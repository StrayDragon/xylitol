//! Input component — multi-line prompt editor for the TUI.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthStr;

use super::component::{Component, EventResult};
use super::event::{AppAction, TuiEvent};
use super::slash::Completer;

const MIN_HEIGHT: u16 = 3;
const MAX_HEIGHT: u16 = MIN_HEIGHT + 5;

pub(crate) struct InputComponent {
    textarea: TextArea<'static>,
    dirty: bool,
    completer: Completer,
    history: Vec<String>,
    history_pos: Option<usize>,
    focused: bool,
    raw_output: bool,
    use_shift_enter_hint: bool,

    // Slash popup (Codex-style, minimal v1).
    slash_popup: SlashPopupState,
}

#[derive(Debug, Clone)]
struct SlashPopupState {
    active: bool,
    query: String,
    matches: Vec<String>,
    selected: usize,
}

impl SlashPopupState {
    fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        }
    }
}

#[cfg(test)]
impl InputComponent {
    pub(crate) fn slash_popup_active(&self) -> bool {
        self.slash_popup.active
    }
}

impl InputComponent {
    fn configure_textarea(textarea: &mut TextArea<'static>) {
        // tui-textarea defaults to styling the cursor/line (often blue + underline). Keep the
        // composer visually minimal so it doesn't fight with terminal themes.
        textarea.set_cursor_style(Style::default());
        textarea.set_cursor_line_style(Style::default());
    }

    pub(crate) fn new(completer: Completer) -> Self {
        let mut textarea = TextArea::default();
        Self::configure_textarea(&mut textarea);
        textarea.set_placeholder_text("Type a message… (Enter to submit, Ctrl+J newline)");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));

        Self {
            textarea,
            dirty: true,
            completer,
            history: Vec::new(),
            history_pos: None,
            focused: false,
            raw_output: false,
            use_shift_enter_hint: false,

            slash_popup: SlashPopupState::new(),
        }
    }

    pub(crate) fn set_use_shift_enter_hint(&mut self, enabled: bool) {
        if self.use_shift_enter_hint != enabled {
            self.use_shift_enter_hint = enabled;
            self.dirty = true;
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    pub(crate) fn desired_height(&self) -> u16 {
        let n = self.textarea.lines().len() as u16;
        n.clamp(MIN_HEIGHT, MAX_HEIGHT)
    }

    pub(crate) fn set_raw_output(&mut self, raw: bool) {
        if self.raw_output != raw {
            self.raw_output = raw;
            self.dirty = true;
        }
    }

    pub(crate) fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
        self.history_pos = None;
    }

    pub(crate) fn clear(&mut self) {
        self.textarea = TextArea::default();
        Self::configure_textarea(&mut self.textarea);
        self.dirty = true;
        self.history_pos = None;
        self.slash_popup = SlashPopupState::new();
    }

    pub(crate) fn load_text(&mut self, text: &str) {
        self.set_text(text);
        self.dirty = true;
    }

    fn set_text(&mut self, text: &str) {
        self.textarea = TextArea::from(text.lines());
        Self::configure_textarea(&mut self.textarea);
        self.dirty = true;
        // Move cursor to end.
        self.textarea.move_cursor(tui_textarea::CursorMove::End);
        self.sync_slash_popup();
    }

    pub(crate) fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub(crate) fn cursor(&self) -> (usize, usize) {
        self.textarea.cursor()
    }

    pub(crate) fn cursor_display_col(&self) -> (usize, usize) {
        let (row, col_chars) = self.textarea.cursor();
        let line = self
            .textarea
            .lines()
            .get(row)
            .map(String::as_str)
            .unwrap_or("");
        let byte_idx = line
            .char_indices()
            .nth(col_chars)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| line.len());
        let prefix = &line[..byte_idx];
        let col_cells = UnicodeWidthStr::width(prefix);
        (row, col_cells)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text().trim().is_empty()
    }

    pub(crate) fn cancel_draft(&mut self) -> bool {
        let mut consumed = false;
        if self.slash_popup.active {
            self.slash_popup = SlashPopupState::new();
            self.dirty = true;
            consumed = true;
        }
        if !self.is_empty() {
            self.clear();
            consumed = true;
        }
        consumed
    }

    pub(crate) fn is_bang_shell_draft(&self) -> bool {
        self.text().trim_start().starts_with('!')
    }

    fn sync_slash_popup(&mut self) {
        let text = self.text();
        let first = text.lines().next().unwrap_or("");
        let trimmed = first.trim_start();
        let should_open = trimmed.starts_with('/');
        if !should_open {
            self.slash_popup = SlashPopupState::new();
            return;
        }

        self.slash_popup.active = true;
        self.slash_popup.query = trimmed.to_string();
        self.slash_popup.matches = self.completer.matches_for_prefix(trimmed);
        if self.slash_popup.matches.is_empty() {
            self.slash_popup.selected = 0;
        } else {
            self.slash_popup.selected = self
                .slash_popup
                .selected
                .min(self.slash_popup.matches.len() - 1);
        }
    }

    fn slash_popup_move(&mut self, delta: i32) {
        if !self.slash_popup.active || self.slash_popup.matches.is_empty() {
            return;
        }
        let len = self.slash_popup.matches.len() as i32;
        let next = (self.slash_popup.selected as i32 + delta).rem_euclid(len) as usize;
        self.slash_popup.selected = next;
        self.dirty = true;
    }

    fn slash_popup_complete_selected(&mut self) -> bool {
        if !self.slash_popup.active {
            return false;
        }
        let Some(selected) = self
            .slash_popup
            .matches
            .get(self.slash_popup.selected)
            .cloned()
        else {
            return false;
        };

        let next = if selected.ends_with(' ') {
            selected
        } else {
            format!("{selected} ")
        };

        self.set_text(&next);
        true
    }

    fn slash_popup_execute_selected(&mut self) -> Option<String> {
        if !self.slash_popup.active {
            return None;
        }
        let selected = self
            .slash_popup
            .matches
            .get(self.slash_popup.selected)
            .cloned()?;

        let text = selected.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.clear();
        Some(text)
    }

    fn handle_history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        match self.history_pos {
            None => self.history_pos = Some(self.history.len().saturating_sub(1)),
            Some(0) => return false,
            Some(pos) => self.history_pos = Some(pos.saturating_sub(1)),
        }
        let pos = self.history_pos.unwrap_or(0);
        let entry = self.history[pos].clone();
        self.set_text(&entry);
        true
    }

    fn handle_history_down(&mut self) -> bool {
        match self.history_pos {
            None => false,
            Some(pos) if pos + 1 < self.history.len() => {
                self.history_pos = Some(pos + 1);
                let entry = self.history[pos + 1].clone();
                self.set_text(&entry);
                true
            }
            Some(_) => {
                self.history_pos = None;
                self.set_text("");
                true
            }
        }
    }

    fn submit(&mut self) -> Option<String> {
        let text = self.text().trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.history.push(text.clone());
        self.history_pos = None;
        self.clear();
        Some(text)
    }

    fn is_slash_mode(&self) -> bool {
        let s = self.text();
        s.trim_start().starts_with('/')
    }

    fn insert_newline(&mut self) {
        self.textarea.insert_newline();
        self.dirty = true;
        self.sync_slash_popup();
    }
}

impl Component for InputComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Codex-style: avoid a persistent boxed panel; rely on subtle style.
        let base_style = if self.focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Reset)
        };

        let placeholder = if self.raw_output {
            "Raw mode: type a message…".to_string()
        } else if self.use_shift_enter_hint {
            "Type a message… (Enter to submit, Shift+Enter newline)".to_string()
        } else {
            "Type a message… (Enter to submit, Ctrl+J newline)".to_string()
        };
        self.textarea.set_placeholder_text(placeholder);
        self.textarea
            .set_placeholder_style(Style::default().fg(Color::DarkGray));

        let block = Block::default().style(base_style);
        self.textarea.set_block(block);
        frame.render_widget(&self.textarea, area);

        if self.slash_popup.active && !self.slash_popup.matches.is_empty() && area.y > 0 {
            let popup_h = (self.slash_popup.matches.len() as u16)
                .min(6)
                .saturating_add(2);
            let popup_w = area.width.min(60);
            let popup_x = area.x;
            let popup_y = area.y.saturating_sub(popup_h);
            let popup = Rect::new(popup_x, popup_y, popup_w, popup_h);

            frame.render_widget(Clear, popup);

            let offset = self.slash_popup.selected.saturating_sub(5);
            let items = self
                .slash_popup
                .matches
                .iter()
                .skip(offset)
                .take(6)
                .enumerate()
                .map(|(idx, m)| {
                    let absolute_idx = offset + idx;
                    if absolute_idx == self.slash_popup.selected {
                        ListItem::new(format!("> {m}"))
                    } else {
                        ListItem::new(format!("  {m}"))
                    }
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Commands ")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().fg(Color::Black).bg(Color::LightCyan));
            let mut state = ListState::default();
            state.select(Some(self.slash_popup.selected.saturating_sub(offset)));
            frame.render_stateful_widget(list, popup, &mut state);
        }

        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult {
        match event {
            TuiEvent::Paste(text) => {
                if self.textarea.insert_str(text) {
                    self.dirty = true;
                    self.sync_slash_popup();
                }
                EventResult::consumed()
            }
            TuiEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};

                if !matches!(
                    key.kind,
                    crossterm::event::KeyEventKind::Press | crossterm::event::KeyEventKind::Repeat
                ) {
                    return EventResult::default();
                }

                // Slash popup navigation.
                if self.slash_popup.active {
                    match key.code {
                        KeyCode::Up => {
                            self.slash_popup_move(-1);
                            return EventResult::consumed();
                        }
                        KeyCode::Down => {
                            self.slash_popup_move(1);
                            return EventResult::consumed();
                        }
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.slash_popup_move(-1);
                            return EventResult::consumed();
                        }
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.slash_popup_move(1);
                            return EventResult::consumed();
                        }
                        KeyCode::Esc => {
                            self.slash_popup = SlashPopupState::new();
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        KeyCode::Tab
                            if key.modifiers.is_empty() && self.slash_popup_complete_selected() =>
                        {
                            return EventResult::consumed();
                        }
                        KeyCode::Enter if key.modifiers.is_empty() => {
                            if let Some(cmd) = self.slash_popup_execute_selected() {
                                return EventResult::action(AppAction::RunPrompt(cmd));
                            }
                        }
                        _ => {}
                    }
                }

                // Esc cancels the draft (Codex-style): dismiss popup if active, then clear.
                if key.code == KeyCode::Esc {
                    if self.cancel_draft() {
                        return EventResult::consumed();
                    }
                    // When the composer is empty, Esc is reserved for backtrack.
                    return EventResult::action(AppAction::BacktrackPrime);
                }

                // Newline insertion (Codex-style): support multiple chords because some terminals can't
                // reliably report Shift+Enter without keyboard enhancement.
                if matches!(key.code, KeyCode::Char('\n' | '\r')) {
                    self.insert_newline();
                    return EventResult::consumed();
                }
                if key.code == KeyCode::Char('j') && key.modifiers == KeyModifiers::CONTROL {
                    self.insert_newline();
                    return EventResult::consumed();
                }
                if key.code == KeyCode::Char('m') && key.modifiers == KeyModifiers::CONTROL {
                    self.insert_newline();
                    return EventResult::consumed();
                }
                if key.code == KeyCode::Enter
                    && (key.modifiers.contains(KeyModifiers::SHIFT)
                        || key.modifiers.contains(KeyModifiers::ALT))
                {
                    self.insert_newline();
                    return EventResult::consumed();
                }

                // Enter submits.
                if key.code == KeyCode::Enter {
                    if let Some(text) = self.submit() {
                        self.dirty = true;
                        return EventResult::action(AppAction::RunPrompt(text));
                    }
                    return EventResult::consumed();
                }

                // History navigation on bare Up/Down.
                if key.code == KeyCode::Up && key.modifiers.is_empty() && self.handle_history_up() {
                    return EventResult::consumed();
                }
                if key.code == KeyCode::Down
                    && key.modifiers.is_empty()
                    && self.handle_history_down()
                {
                    return EventResult::consumed();
                }

                // Codex-style Tab key:
                // - When slash popup active: complete selection (handled above)
                // - Otherwise: queue-or-submit semantics at the app layer
                //   (except for bang-shell drafts, which should not submit on Tab while idle).
                if key.code == KeyCode::Tab && key.modifiers.is_empty() {
                    if self.is_bang_shell_draft() {
                        // Preserve Tab for indentation/completion/no-op.
                        return EventResult::consumed();
                    }
                    if let Some(text) = self.submit() {
                        self.dirty = true;
                        return EventResult::action(AppAction::QueueOrSubmit(text));
                    }
                    return EventResult::consumed();
                }

                // Readline tweak: Ctrl+U kills to head-of-line (textarea default uses Ctrl+U for undo).
                if key.code == KeyCode::Char('u') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if self.textarea.delete_line_by_head() {
                        self.dirty = true;
                    }
                    return EventResult::consumed();
                }

                if key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return EventResult::action(AppAction::ShowHistorySearch);
                }

                if key.code == KeyCode::Char('g') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return EventResult::action(AppAction::OpenEditor(self.text()));
                }

                // Default handling via tui-textarea.
                let input: tui_textarea::Input = (*key).into();
                if self.textarea.input(input) {
                    self.dirty = true;
                    self.sync_slash_popup();
                    return EventResult::consumed();
                }

                EventResult::default()
            }
            _ => EventResult::default(),
        }
    }
}
