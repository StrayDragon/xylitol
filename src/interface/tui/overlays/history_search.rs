//! Interactive history search overlay (Ctrl+R).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::interface::tui::component::{Component, EventResult, OverlayAction};
use crate::interface::tui::event::{AppAction, TuiEvent};

pub(crate) struct HistorySearchOverlay {
    entries: Vec<String>,
    query: String,
    filtered: Vec<usize>,
    selected: usize,
    dirty: bool,
}

impl HistorySearchOverlay {
    pub(crate) fn new(entries: Vec<String>) -> Self {
        let mut overlay = Self {
            entries,
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            dirty: true,
        };
        overlay.recompute_filtered();
        overlay
    }

    fn centered(area: Rect, width: u16, height: u16) -> Rect {
        let w = width.min(area.width);
        let h = height.min(area.height);
        let x = area.x + (area.width.saturating_sub(w) / 2);
        let y = area.y + (area.height.saturating_sub(h) / 2);
        Rect::new(x, y, w, h)
    }

    fn recompute_filtered(&mut self) {
        let query = self.query.trim();
        let mut out: Vec<(usize, i64)> = Vec::new();
        for (idx, item) in self.entries.iter().enumerate() {
            if let Some(score) = fuzzy_score(item, query) {
                out.push((idx, score));
            }
        }
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.filtered = out.into_iter().map(|(idx, _)| idx).collect();
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
    }

    fn selected_value(&self) -> String {
        let idx = self.filtered.get(self.selected).copied().unwrap_or(0);
        self.entries.get(idx).cloned().unwrap_or_default()
    }
}

impl Component for HistorySearchOverlay {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let modal = Self::centered(area, 80, 20);
        frame.render_widget(Clear, modal);

        let items = self
            .filtered
            .iter()
            .filter_map(|idx| self.entries.get(*idx))
            .map(|s| ListItem::new(s.as_str()))
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" History  {} ", self.query))
                    .border_style(Style::default().fg(Color::LightCyan)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        if !self.filtered.is_empty() {
            state.select(Some(
                self.selected.min(self.filtered.len().saturating_sub(1)),
            ));
        }

        frame.render_stateful_widget(list, modal, &mut state);
        self.dirty = false;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn handle_event(&mut self, event: &TuiEvent) -> EventResult {
        let TuiEvent::Key(key) = event else {
            return EventResult::default();
        };
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => EventResult {
                consumed: true,
                overlay: Some(OverlayAction::Dismiss),
                ..EventResult::default()
            },
            KeyCode::Backspace => {
                self.query.pop();
                self.recompute_filtered();
                self.dirty = true;
                EventResult::consumed()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.dirty = true;
                }
                EventResult::consumed()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                    self.dirty = true;
                }
                EventResult::consumed()
            }
            KeyCode::Enter => {
                let selected = self.selected_value();
                EventResult {
                    consumed: true,
                    action: Some(AppAction::LoadInput(selected)),
                    overlay: Some(OverlayAction::Dismiss),
                }
            }
            KeyCode::Char(ch)
                if key.modifiers.is_empty()
                    || key.modifiers == KeyModifiers::SHIFT
                    || key.modifiers == KeyModifiers::NONE =>
            {
                self.query.push(ch);
                self.recompute_filtered();
                self.dirty = true;
                EventResult::consumed()
            }
            _ => EventResult::consumed(),
        }
    }
}

fn fuzzy_score(candidate: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let cand_lower = candidate.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();

    let mut score: i64 = 0;
    let mut last_match: Option<usize> = None;
    let mut pos = 0usize;

    for ch in query_lower.chars() {
        let mut found: Option<usize> = None;
        for (i, cand_ch) in cand_lower[pos..].chars().enumerate() {
            if cand_ch == ch {
                found = Some(pos + i);
                break;
            }
        }
        let idx = found?;

        score -= idx as i64;
        if let Some(prev) = last_match
            && idx == prev + 1
        {
            score += 10;
        }

        last_match = Some(idx);
        pos = idx + 1;
    }

    Some(score)
}
