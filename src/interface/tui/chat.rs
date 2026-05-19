//! Chat component — scrollable message history with markdown rendering.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::agent::r#loop::{AgentError, AgentEvent};

use super::component::{Component, EventResult};
use super::event::TuiEvent;
use super::markdown::MarkdownRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct Message {
    role: Role,
    content: String,
    streaming: bool,
    cached_width: u16,
    cached_lines: Vec<Line<'static>>,
    dirty: bool,
}

impl Message {
    fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            streaming: false,
            cached_width: 0,
            cached_lines: Vec::new(),
            dirty: true,
        }
    }

    fn role_label(&self) -> (&'static str, Style) {
        match self.role {
            Role::User => (
                "user",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Assistant => (
                "assistant",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::System => (
                "system",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Role::Error => (
                "error",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolCardStatus {
    Running,
    Success,
    Failed,
}

#[derive(Debug, Clone)]
struct ToolCallCard {
    id: String,
    name: String,
    args: serde_json::Value,
    status: ToolCardStatus,
    result: Option<serde_json::Value>,
    started_at: Instant,
    ended_at: Option<Instant>,
    expanded: bool,
    cached_width: u16,
    cached_lines: Vec<Line<'static>>,
    dirty: bool,
}

impl ToolCallCard {
    fn new_running(id: String, name: String, args: serde_json::Value) -> Self {
        Self {
            id,
            name,
            args,
            status: ToolCardStatus::Running,
            result: None,
            started_at: Instant::now(),
            ended_at: None,
            expanded: true,
            cached_width: 0,
            cached_lines: Vec::new(),
            dirty: true,
        }
    }

    fn complete(&mut self, result: serde_json::Value) {
        self.ended_at = Some(Instant::now());
        self.status = if result.get("blocked").and_then(|v| v.as_bool()) == Some(true)
            || result.get("error").is_some()
        {
            ToolCardStatus::Failed
        } else {
            ToolCardStatus::Success
        };
        self.result = Some(result);
        // Default to collapsed on completion.
        self.expanded = false;
        self.dirty = true;
    }

    fn toggle(&mut self) {
        self.expanded = !self.expanded;
        self.dirty = true;
    }

    fn render_lines(&mut self, width: u16) -> Vec<Line<'static>> {
        if !self.dirty && self.cached_width == width {
            return self.cached_lines.clone();
        }

        let inner_w = width.max(20);
        let mut out = Vec::<Line<'static>>::new();

        let (icon, status_label, status_style) = match self.status {
            ToolCardStatus::Running => (
                "🔧",
                "running…",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            ToolCardStatus::Success => (
                "✅",
                "ok",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            ToolCardStatus::Failed => (
                "❌",
                "failed",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        };

        let duration = self
            .ended_at
            .and_then(|end| end.checked_duration_since(self.started_at))
            .map(|d| format!("{:.1}s", d.as_secs_f32()));

        let mut header_right = status_label.to_string();
        if let Some(d) = duration {
            header_right = format!("{header_right}  {d}");
        }

        // ┌─ <icon> <name> ───────── <status> ┐
        out.push(render_box_top(
            inner_w,
            &format!("{icon} {}", self.name),
            &header_right,
            status_style,
        ));

        if self.expanded {
            // Arguments (pretty, limited).
            let args_text =
                serde_json::to_string_pretty(&self.args).unwrap_or_else(|_| self.args.to_string());
            out.extend(render_box_body(
                inner_w,
                "Arguments",
                &args_text,
                Style::default().fg(Color::DarkGray),
            ));

            if let Some(ref result) = self.result {
                let result_text =
                    serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string());
                out.extend(render_box_body(
                    inner_w,
                    "Result",
                    &result_text,
                    Style::default().fg(Color::DarkGray),
                ));
            }

            out.push(render_box_hint(
                inner_w,
                "Enter to collapse",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            out.push(render_box_hint(
                inner_w,
                "Enter to expand",
                Style::default().fg(Color::DarkGray),
            ));
        }

        out.push(Line::from(render_box_bottom(inner_w)));

        self.cached_width = inner_w;
        self.cached_lines = out.clone();
        self.dirty = false;

        out
    }
}

#[derive(Debug, Clone)]
struct ThinkingBlock {
    content: String,
    expanded: bool,
    cached_width: u16,
    cached_lines: Vec<Line<'static>>,
    dirty: bool,
}

impl ThinkingBlock {
    fn new_collapsed(content: String) -> Self {
        Self {
            content,
            expanded: false,
            cached_width: 0,
            cached_lines: Vec::new(),
            dirty: true,
        }
    }

    fn toggle(&mut self) {
        self.expanded = !self.expanded;
        self.dirty = true;
    }

    fn render_lines(&mut self, width: u16) -> Vec<Line<'static>> {
        if !self.dirty && self.cached_width == width {
            return self.cached_lines.clone();
        }

        let inner_w = width.max(20);
        let mut out = Vec::<Line<'static>>::new();

        out.push(render_box_top(
            inner_w,
            "💭 Thinking",
            if self.expanded {
                "expanded"
            } else {
                "collapsed"
            },
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));

        if self.expanded {
            out.extend(render_box_body(
                inner_w,
                "Content",
                &self.content,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
            out.push(render_box_hint(
                inner_w,
                "Enter to collapse",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            out.push(render_box_hint(
                inner_w,
                "Enter to expand",
                Style::default().fg(Color::DarkGray),
            ));
        }

        out.push(Line::from(render_box_bottom(inner_w)));

        self.cached_width = inner_w;
        self.cached_lines = out.clone();
        self.dirty = false;
        out
    }
}

#[derive(Debug, Clone)]
enum ChatItem {
    Message(Message),
    ToolCard(ToolCallCard),
    Thinking(ThinkingBlock),
}

pub(crate) struct ChatComponent {
    markdown: MarkdownRenderer,
    items: Vec<ChatItem>,
    /// Scroll offset measured from the bottom (0 = show newest).
    scroll_offset: u16,
    follow_tail: bool,
    focused: bool,
    dirty: bool,
    last_toggle_idx: Option<usize>,
}

impl ChatComponent {
    pub(crate) fn new(markdown: MarkdownRenderer) -> Self {
        Self {
            markdown,
            items: Vec::new(),
            scroll_offset: 0,
            follow_tail: true,
            focused: false,
            dirty: true,
            last_toggle_idx: None,
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    pub(crate) fn set_theme(&mut self, theme: &str) -> bool {
        if !self.markdown.set_theme(theme) {
            return false;
        }

        // Invalidate markdown caches.
        for item in &mut self.items {
            match item {
                ChatItem::Message(msg) => {
                    msg.cached_width = 0;
                    msg.dirty = true;
                }
                ChatItem::ToolCard(card) => {
                    card.cached_width = 0;
                    card.dirty = true;
                }
                ChatItem::Thinking(block) => {
                    block.cached_width = 0;
                    block.dirty = true;
                }
            }
        }
        self.dirty = true;
        true
    }

    pub(crate) fn add_user_message(&mut self, text: &str) {
        self.items.push(ChatItem::Message(Message::new(
            Role::User,
            text.to_string(),
        )));
        self.scroll_to_bottom();
        self.dirty = true;
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
        self.scroll_offset = 0;
        self.follow_tail = true;
        self.last_toggle_idx = None;
        self.dirty = true;
    }

    pub(crate) fn scroll_wheel_up(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    pub(crate) fn scroll_wheel_down(&mut self, n: u16) {
        self.scroll_down(n);
    }

    fn append_assistant_delta(&mut self, delta: &str) {
        let last_streaming = self.items.iter_mut().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant && msg.streaming => Some(msg),
            _ => None,
        });

        if let Some(msg) = last_streaming {
            msg.content.push_str(delta);
            msg.dirty = true;
        } else {
            let mut m = Message::new(Role::Assistant, delta.to_string());
            m.streaming = true;
            self.items.push(ChatItem::Message(m));
        }

        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn finish_streaming(&mut self) {
        let last_assistant = self.items.iter_mut().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant => Some(msg),
            _ => None,
        });

        if let Some(msg) = last_assistant {
            msg.streaming = false;
            msg.dirty = true;

            let thinking_blocks = extract_thinking_blocks(&mut msg.content);
            if !thinking_blocks.is_empty() {
                for block in thinking_blocks {
                    self.items
                        .push(ChatItem::Thinking(ThinkingBlock::new_collapsed(block)));
                    self.last_toggle_idx = Some(self.items.len().saturating_sub(1));
                }
            }
        }
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn show_tool_message(&mut self, line: String) {
        self.items
            .push(ChatItem::Message(Message::new(Role::System, line)));
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn show_error(&mut self, err: AgentError) {
        self.items.push(ChatItem::Message(Message::new(
            Role::Error,
            format!("{err}"),
        )));
        if self.follow_tail {
            self.scroll_offset = 0;
        }
        self.dirty = true;
    }

    fn scroll_up(&mut self, n: u16, max_scroll: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n).min(max_scroll);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.follow_tail = self.scroll_offset == 0;
        self.dirty = true;
    }

    fn scroll_to_top(&mut self, max_scroll: u16) {
        self.scroll_offset = max_scroll;
        self.follow_tail = false;
        self.dirty = true;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.follow_tail = true;
        self.dirty = true;
    }

    fn render_lines(&mut self, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        for item in &mut self.items {
            match item {
                ChatItem::Message(msg) => {
                    let (label, style) = msg.role_label();
                    let suffix = if msg.streaming { " ▍" } else { "" };
                    lines.push(Line::from(Span::styled(format!("{label}{suffix}"), style)));

                    if msg.dirty || msg.cached_width != width {
                        msg.cached_width = width;
                        msg.cached_lines = self.markdown.render(&msg.content, width);
                        msg.dirty = false;
                    }
                    lines.extend(msg.cached_lines.clone());
                    lines.push(Line::from(""));
                }
                ChatItem::ToolCard(card) => {
                    lines.extend(card.render_lines(width));
                    lines.push(Line::from(""));
                }
                ChatItem::Thinking(block) => {
                    lines.extend(block.render_lines(width));
                    lines.push(Line::from(""));
                }
            }
        }

        lines
    }
}

impl Component for ChatComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let inner_width = area.width.saturating_sub(2);
        let mut lines = self.render_lines(inner_width);

        let view_height = area.height.saturating_sub(2);
        let total_lines = lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(view_height);
        let scroll = self.scroll_offset.min(max_scroll);
        let start = max_scroll.saturating_sub(scroll) as usize;
        let end = (start + view_height as usize).min(lines.len());

        let visible = if start < end {
            lines.drain(start..end).collect::<Vec<Line<'static>>>()
        } else {
            Vec::new()
        };

        let title = if max_scroll == 0 {
            " Chat ".to_string()
        } else {
            format!(
                " Chat  ({} / {}) ",
                max_scroll.saturating_sub(scroll),
                max_scroll
            )
        };

        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let para = Paragraph::new(visible)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style)
                    .title_style(border_style),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(para, area);
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
            TuiEvent::Agent(agent_event) => {
                match agent_event {
                    AgentEvent::TextDelta(delta) => self.append_assistant_delta(delta),
                    AgentEvent::ToolCallStart { id, name, args } => {
                        self.items
                            .push(ChatItem::ToolCard(ToolCallCard::new_running(
                                id.clone(),
                                name.clone(),
                                args.clone(),
                            )));
                        self.last_toggle_idx = Some(self.items.len().saturating_sub(1));
                    }
                    AgentEvent::ToolCallEnd { id, result } => {
                        if let Some(idx) = self.items.iter().position(|item| match item {
                            ChatItem::ToolCard(card) => card.id == *id,
                            _ => false,
                        }) {
                            if let Some(ChatItem::ToolCard(card)) = self.items.get_mut(idx) {
                                card.complete(result.clone());
                            }
                            self.last_toggle_idx = Some(idx);
                        } else {
                            self.show_tool_message(format!("Tool result ({id}): {result}"));
                        }
                    }
                    AgentEvent::StepComplete { .. } => self.finish_streaming(),
                    AgentEvent::RepeatDetected { .. } => {
                        self.show_tool_message("Repeat detected — interrupted.".to_string());
                        self.finish_streaming();
                    }
                    AgentEvent::Error(err) => self.show_error(err.clone()),
                }
                return EventResult::consumed();
            }
            TuiEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};
                if !key.modifiers.is_empty() {
                    return EventResult::default();
                }

                if key.code == KeyCode::Enter
                    && let Some(idx) = self.last_toggle_idx
                    && let Some(item) = self.items.get_mut(idx)
                {
                    match item {
                        ChatItem::ToolCard(card) => card.toggle(),
                        ChatItem::Thinking(block) => block.toggle(),
                        ChatItem::Message(_) => {}
                    }
                    self.dirty = true;
                    return EventResult::consumed();
                }

                // Scroll shortcuts.
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        // Max scroll is computed lazily in render; approximate here as "some".
                        // The exact clamp will happen in render.
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                        self.follow_tail = self.scroll_offset == 0;
                        self.dirty = true;
                        return EventResult::consumed();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.scroll_down(1);
                        return EventResult::consumed();
                    }
                    KeyCode::Char('g') => {
                        // We'll clamp in render once we know max_scroll.
                        self.scroll_offset = u16::MAX;
                        self.follow_tail = false;
                        self.dirty = true;
                        return EventResult::consumed();
                    }
                    KeyCode::Char('G') => {
                        self.scroll_to_bottom();
                        return EventResult::consumed();
                    }
                    _ => {}
                }

                // Ctrl+L: clear screen (handled at app level) should not scroll chat.
                if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return EventResult::default();
                }
            }
            TuiEvent::Mouse(_) | TuiEvent::Tick | TuiEvent::Shutdown => {}
        }

        EventResult::default()
    }
}

fn extract_thinking_blocks(text: &mut String) -> Vec<String> {
    // Supported markers (minimal v1):
    // - <thinking>...</thinking>
    // - <analysis>...</analysis>
    let mut blocks = Vec::new();

    let candidates = [
        ("thinking", "<thinking>", "</thinking>"),
        ("analysis", "<analysis>", "</analysis>"),
    ];

    for (_label, open, close) in candidates {
        while let Some(start) = text.find(open) {
            let Some(end) = text[start + open.len()..]
                .find(close)
                .map(|i| i + start + open.len())
            else {
                break;
            };
            let inner = text[start + open.len()..end].to_string();
            blocks.push(inner.trim().to_string());
            text.replace_range(start..end + close.len(), "");
        }
    }

    blocks.retain(|b| !b.is_empty());
    blocks
}

fn render_box_top(width: u16, left: &str, right: &str, right_style: Style) -> Line<'static> {
    let inner = width.saturating_sub(2).max(2) as usize;
    let mut left_text = format!(" {left} ");
    let mut right_text = format!(" {right} ");

    if left_text.chars().count() > inner {
        left_text = left_text.chars().take(inner).collect();
    }
    if right_text.chars().count() > inner {
        right_text = right_text.chars().take(inner).collect();
    }

    // Place right-aligned marker at the end.
    let left_len = left_text.chars().count();
    let right_len = right_text.chars().count();
    let gap = inner.saturating_sub(left_len + right_len);

    Line::from(vec![
        Span::raw("┌"),
        Span::raw(left_text),
        Span::raw("─".repeat(gap)),
        Span::styled(right_text, right_style),
        Span::raw("┐"),
    ])
}

fn render_box_body(width: u16, label: &str, body: &str, style: Style) -> Vec<Line<'static>> {
    let inner = width.saturating_sub(2).max(2) as usize;
    let mut out = Vec::new();

    out.push(Line::from(vec![
        Span::raw("│ "),
        Span::styled(format!("{label}:"), style.add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(inner.saturating_sub(label.len() + 2))),
        Span::raw("│"),
    ]));

    for line in body.lines().take(10) {
        let mut text = line.to_string();
        if text.chars().count() > inner.saturating_sub(3) {
            text = text.chars().take(inner.saturating_sub(3)).collect();
        }
        let padding = inner.saturating_sub(2).saturating_sub(text.chars().count());
        out.push(Line::from(vec![
            Span::raw("│ "),
            Span::styled(text, style),
            Span::raw(" ".repeat(padding)),
            Span::raw("│"),
        ]));
    }
    if body.lines().count() > 10 {
        out.push(Line::from(vec![
            Span::raw("│ "),
            Span::styled("…".to_string(), style),
            Span::raw(" ".repeat(inner.saturating_sub(3))),
            Span::raw("│"),
        ]));
    }

    out
}

fn render_box_hint(width: u16, hint: &str, style: Style) -> Line<'static> {
    let inner = width.saturating_sub(2).max(2) as usize;
    let mut text = hint.to_string();
    if text.chars().count() > inner.saturating_sub(2) {
        text = text.chars().take(inner.saturating_sub(2)).collect();
    }
    let padding = inner.saturating_sub(2).saturating_sub(text.chars().count());

    Line::from(vec![
        Span::raw("│ "),
        Span::styled(text, style),
        Span::raw(" ".repeat(padding)),
        Span::raw("│"),
    ])
}

fn render_box_bottom(width: u16) -> String {
    let inner = width.saturating_sub(2).max(2) as usize;
    format!("└{}┘", "─".repeat(inner))
}
