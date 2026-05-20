//! Chat component — scrollable message history with markdown rendering.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::agent::r#loop::{AgentError, AgentEvent};

use super::chat_style;
use super::component::{Component, EventResult};
use super::event::TuiEvent;
use super::markdown::MarkdownRenderer;
use std::collections::HashMap;

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
    thinking: Option<String>,
    streaming: bool,
    cached_width: u16,
    cached_lines: Vec<Line<'static>>,
    dirty: bool,
}

impl Message {
    pub(crate) fn role(&self) -> &'static str {
        match self.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Error => "error",
        }
    }

    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn streaming(&self) -> bool {
        self.streaming
    }
}

impl Message {
    fn new(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            thinking: None,
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

#[derive(Debug, Clone)]
enum ChatItem {
    Message(Message),
}

pub(crate) struct ChatComponent {
    markdown: MarkdownRenderer,
    items: Vec<ChatItem>,
    raw_output: bool,
    pending_thinking: String,
    last_thinking: String,
    tool_names: HashMap<String, String>,
    next_insert_index: usize,
    show_thinking: bool,
    dirty: bool,
}

impl ChatComponent {
    pub(crate) fn new(markdown: MarkdownRenderer) -> Self {
        Self {
            markdown,
            items: Vec::new(),
            raw_output: false,
            pending_thinking: String::new(),
            last_thinking: String::new(),
            tool_names: HashMap::new(),
            next_insert_index: 0,
            show_thinking: false,
            dirty: true,
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        let _ = focused;
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
            }
        }
        self.dirty = true;
        true
    }

    pub(crate) fn set_raw_output(&mut self, raw: bool) {
        if self.raw_output != raw {
            self.raw_output = raw;
            // Invalidate markdown caches so the next draw uses the selected renderer.
            for item in &mut self.items {
                match item {
                    ChatItem::Message(msg) => {
                        msg.cached_width = 0;
                        msg.dirty = true;
                    }
                }
            }
            self.dirty = true;
        }
    }

    pub(crate) fn add_user_message(&mut self, text: &str) {
        self.items.push(ChatItem::Message(Message::new(
            Role::User,
            text.to_string(),
        )));
        self.dirty = true;
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
        self.pending_thinking.clear();
        self.last_thinking.clear();
        self.tool_names.clear();
        self.next_insert_index = 0;
        self.show_thinking = false;
        self.dirty = true;
    }

    fn thinking_text(&self) -> &str {
        if !self.pending_thinking.trim().is_empty() {
            &self.pending_thinking
        } else {
            &self.last_thinking
        }
    }

    pub(crate) fn has_thinking_panel(&self) -> bool {
        !self.thinking_text().trim().is_empty()
    }

    pub(crate) fn thinking_panel_height(&self, max_height: u16) -> u16 {
        if !self.has_thinking_panel() {
            return 0;
        }
        // 1 header line + content lines, capped.
        let content_lines = self.thinking_text().lines().count() as u16;
        content_lines.saturating_add(1).min(max_height.max(1))
    }

    pub(crate) fn render_thinking_panel(&mut self, frame: &mut Frame, area: Rect) {
        if area.is_empty() || !self.has_thinking_panel() {
            return;
        }

        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        let mut lines = Vec::<Line<'static>>::new();
        lines.push(Line::from(Span::styled("Thinking", style)));

        let content_h = area.height.saturating_sub(1) as usize;
        if content_h > 0 {
            let all = self
                .thinking_text()
                .lines()
                .map(|l| Line::from(Span::styled(l.to_string(), style)))
                .collect::<Vec<_>>();
            let start = all.len().saturating_sub(content_h);
            lines.extend(all.into_iter().skip(start));
        }

        // No borders/cards: keep it lightweight and let the transcript use the remaining space.
        let para = Paragraph::new(lines)
            .block(Block::default())
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
    }

    pub(crate) fn scroll_wheel_up(&mut self, n: u16) {
        let _ = n;
        self.dirty = true;
    }

    pub(crate) fn scroll_wheel_down(&mut self, n: u16) {
        let _ = n;
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

        self.dirty = true;
    }

    fn append_thinking_delta(&mut self, delta: &str) {
        self.pending_thinking.push_str(delta);
        self.show_thinking = true;
        self.dirty = true;
    }

    fn finish_streaming(&mut self) {
        let last_assistant_index = self.items.iter().rposition(
            |item| matches!(item, ChatItem::Message(msg) if msg.role == Role::Assistant),
        );

        let mut thinking_parts = Vec::<String>::new();

        if let Some(idx) = last_assistant_index
            && let Some(ChatItem::Message(msg)) = self.items.get_mut(idx)
        {
            msg.streaming = false;
            msg.dirty = true;

            let thinking_blocks = extract_thinking_blocks(&mut msg.content);
            if !thinking_blocks.is_empty() {
                thinking_parts.extend(thinking_blocks);
            }
        }

        if !self.pending_thinking.trim().is_empty() {
            let block = std::mem::take(&mut self.pending_thinking);
            thinking_parts.push(block);
        } else {
            self.pending_thinking.clear();
        }

        let combined_thinking = thinking_parts.join("\n\n").trim().to_string();
        if let Some(idx) = last_assistant_index
            && let Some(ChatItem::Message(msg)) = self.items.get_mut(idx)
        {
            if combined_thinking.is_empty() {
                msg.thinking = None;
            } else {
                msg.thinking = Some(combined_thinking.clone());
            }
        }

        if combined_thinking.is_empty() {
            self.last_thinking.clear();
        } else {
            self.last_thinking = combined_thinking;
        }
        // Collapse thinking once the assistant is done; users can re-open it via a shortcut.
        self.show_thinking = false;
        self.dirty = true;
    }

    fn show_tool_message(&mut self, line: String) {
        self.items
            .push(ChatItem::Message(Message::new(Role::System, line)));
        self.dirty = true;
    }

    fn show_error(&mut self, err: AgentError) {
        self.items.push(ChatItem::Message(Message::new(
            Role::Error,
            format!("{err}"),
        )));
        self.dirty = true;
    }

    fn render_thinking_inline_lines(&self, max_height: usize) -> Vec<Line<'static>> {
        if !self.has_thinking_panel() || max_height == 0 {
            return Vec::new();
        }

        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        let mut lines = Vec::<Line<'static>>::new();
        lines.push(Line::from(Span::styled("Thinking", style)));

        let max_content = max_height.saturating_sub(1);
        let mut content = self
            .thinking_text()
            .lines()
            .map(|l| Line::from(Span::styled(format!("  {l}"), style)))
            .collect::<Vec<_>>();
        if content.len() > max_content {
            let start = content.len().saturating_sub(max_content);
            content = content.split_off(start);
        }
        lines.extend(content);
        lines
    }

    pub(crate) fn toggle_show_thinking(&mut self) {
        self.show_thinking = !self.show_thinking;
        self.dirty = true;
    }

    pub(crate) fn take_pending_insert_lines(&mut self, width: u16) -> Vec<Line<'static>> {
        let mut out = Vec::<Line<'static>>::new();
        while let Some(item) = self.items.get_mut(self.next_insert_index) {
            let ChatItem::Message(msg) = item;
            if msg.streaming {
                break;
            }

            if msg.dirty || msg.cached_width != width {
                msg.cached_width = width;
                msg.cached_lines = if self.raw_output {
                    msg.content
                        .trim_end_matches(['\r', '\n'])
                        .lines()
                        .map(|l| Line::from(l.to_string()))
                        .collect::<Vec<_>>()
                } else {
                    self.markdown.render(&msg.content, width)
                };
                msg.dirty = false;
            }

            match msg.role {
                Role::User => {
                    out.extend(chat_style::prefix_user_lines(msg.cached_lines.clone()));
                    out.push(Line::from(""));
                }
                Role::Assistant => {
                    out.extend(chat_style::prefix_assistant_lines(
                        msg.cached_lines.clone(),
                        /*streaming*/ false,
                    ));
                    out.push(Line::from(""));
                }
                Role::System => {
                    let mut system_lines = msg.cached_lines.clone();
                    for line in &mut system_lines {
                        line.style = Style::default().fg(Color::DarkGray);
                    }
                    out.extend(system_lines);
                    out.push(Line::from(""));
                }
                Role::Error => {
                    let mut error_lines = msg.cached_lines.clone();
                    for line in &mut error_lines {
                        line.style = Style::default().fg(Color::Red);
                    }
                    out.extend(error_lines);
                    out.push(Line::from(""));
                }
            }

            self.next_insert_index += 1;
        }

        out
    }

    pub(crate) fn render_live_preview(&mut self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let width = area.width.max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();

        let show_thinking = self.show_thinking && self.has_thinking_panel();
        if show_thinking {
            lines.extend(self.render_thinking_inline_lines(6));
            lines.push(Line::from(""));
        }

        let streaming = self.items.iter_mut().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant && msg.streaming => Some(msg),
            _ => None,
        });

        if let Some(msg) = streaming {
            if msg.dirty || msg.cached_width != width {
                msg.cached_width = width;
                msg.cached_lines = if self.raw_output {
                    msg.content
                        .trim_end_matches(['\r', '\n'])
                        .lines()
                        .map(|l| Line::from(l.to_string()))
                        .collect::<Vec<_>>()
                } else {
                    self.markdown.render(&msg.content, width)
                };
                msg.dirty = false;
            }

            let mut assistant_lines = chat_style::prefix_assistant_lines(
                msg.cached_lines.clone(),
                /*streaming*/ true,
            );
            // Clip to available height (keep newest).
            let remaining = area.height.saturating_sub(lines.len() as u16).max(1) as usize;
            if assistant_lines.len() > remaining {
                assistant_lines =
                    assistant_lines.split_off(assistant_lines.len().saturating_sub(remaining));
            }
            lines.extend(assistant_lines);
        }

        // Clip overall to the viewport height.
        if lines.len() > area.height as usize {
            lines = lines.split_off(lines.len() - area.height as usize);
        }

        let para = Paragraph::new(lines)
            .block(Block::default())
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
        self.dirty = false;
    }

    pub(crate) fn last_user_message(&self) -> Option<String> {
        self.items.iter().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::User => Some(msg.content.clone()),
            _ => None,
        })
    }

    pub(crate) fn last_assistant_message(&self) -> Option<String> {
        self.items.iter().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant => Some(msg.content.clone()),
            _ => None,
        })
    }

    pub(crate) fn transcript_as_markdown(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            match item {
                ChatItem::Message(msg) => {
                    let label = match msg.role {
                        Role::User => "User",
                        Role::Assistant => "Assistant",
                        Role::System => "System",
                        Role::Error => "Error",
                    };
                    out.push_str(&format!("## {label}\n\n"));
                    out.push_str(&msg.content);
                    out.push_str("\n\n");

                    if msg.role == Role::Assistant
                        && let Some(thinking) = msg.thinking.as_deref()
                        && !thinking.trim().is_empty()
                    {
                        out.push_str("### Thinking\n\n");
                        out.push_str(thinking);
                        out.push_str("\n\n");
                    }
                }
            }
        }
        out
    }
}

impl Component for ChatComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // In inline mode the transcript is appended above the viewport via `Terminal::insert_before`.
        // The chat widget's "render" path is now just the live preview.
        self.render_live_preview(frame, area);
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
                    AgentEvent::ThinkingDelta(delta) => self.append_thinking_delta(delta),
                    AgentEvent::ToolCallStart { id, name, args } => {
                        let args_summary = tool_args_summary(args);
                        let mut msg = format!("• {name}");
                        if !args_summary.is_empty() {
                            msg.push_str(&format!("\n  └ {args_summary}"));
                        }
                        self.tool_names.insert(id.clone(), name.clone());
                        self.show_tool_message(msg);
                    }
                    AgentEvent::ToolCallEnd { id, result } => {
                        let status = tool_result_status(result);
                        let tool_name = self.tool_names.remove(id).unwrap_or_else(|| "tool".into());
                        self.show_tool_message(format!("  └ {tool_name}: {status}"));
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
                let _ = (key, KeyCode::Enter, KeyModifiers::NONE);
            }
            TuiEvent::Paste(_) | TuiEvent::Mouse(_) | TuiEvent::Tick | TuiEvent::Shutdown => {}
        }

        EventResult::default()
    }
}

fn tool_args_summary(args: &serde_json::Value) -> String {
    use serde_json::Value;

    fn kv(key: &str, value: &Value) -> Option<String> {
        match value {
            Value::Null => None,
            Value::String(s) => Some(format!("{key}={}", s.trim())),
            Value::Bool(b) => Some(format!("{key}={b}")),
            Value::Number(n) => Some(format!("{key}={n}")),
            Value::Array(arr) => Some(format!("{key}=[{}]", arr.len())),
            Value::Object(map) => Some(format!("{key}={{{} keys}}", map.len())),
        }
    }

    let Value::Object(map) = args else {
        let s = args.to_string();
        return truncate_one_line(&s, 140);
    };

    let preferred_keys = [
        "file_path",
        "path",
        "query",
        "url",
        "command",
        "mode",
        "name",
    ];

    let mut parts = Vec::new();
    for key in preferred_keys {
        if let Some(value) = map.get(key)
            && let Some(rendered) = kv(key, value)
        {
            parts.push(rendered);
        }
    }

    if parts.is_empty() {
        truncate_one_line(&args.to_string(), 140)
    } else {
        truncate_one_line(&parts.join(" "), 140)
    }
}

fn tool_result_status(result: &serde_json::Value) -> &'static str {
    if result.get("blocked").and_then(|v| v.as_bool()) == Some(true) {
        return "blocked";
    }
    if result.get("error").is_some() {
        return "error";
    }
    "ok"
}

fn truncate_one_line(text: &str, max_chars: usize) -> String {
    let mut out = text.lines().next().unwrap_or("").trim().to_string();
    if out.chars().count() <= max_chars {
        return out;
    }
    out = out.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_deltas_accumulate_into_thinking_panel_on_step_complete() {
        let mut chat = ChatComponent::new(MarkdownRenderer::default());

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ThinkingDelta("a".to_string())));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ThinkingDelta("b".to_string())));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::StepComplete {
            step: 1,
            summary: String::new(),
        }));

        assert_eq!(chat.last_thinking, "ab");
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
