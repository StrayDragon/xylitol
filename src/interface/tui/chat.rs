//! Chat component — scrollable message history with markdown rendering.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use leaf_core::streaming::StreamingRenderer;

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
    streaming_renderer: Option<StreamingRenderer>,
    items: Vec<ChatItem>,
    raw_output: bool,
    pending_thinking: String,
    last_thinking: String,
    tool_names: HashMap<String, String>,
    scroll_from_bottom: usize,
    last_rendered_width: u16,
    last_rendered_line_count: usize,
    dirty: bool,
}

impl ChatComponent {
    pub(crate) fn new(markdown: MarkdownRenderer) -> Self {
        Self {
            markdown,
            streaming_renderer: None,
            items: Vec::new(),
            raw_output: false,
            pending_thinking: String::new(),
            last_thinking: String::new(),
            tool_names: HashMap::new(),
            scroll_from_bottom: 0,
            last_rendered_width: 0,
            last_rendered_line_count: 0,
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
        self.scroll_from_bottom = 0;
        self.last_rendered_width = 0;
        self.last_rendered_line_count = 0;
        self.dirty = true;
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.scroll_from_bottom = self.scroll_from_bottom.saturating_add(n.max(1));
        self.dirty = true;
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.scroll_from_bottom = self.scroll_from_bottom.saturating_sub(n.max(1));
        self.dirty = true;
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_from_bottom = 0;
        self.dirty = true;
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

        if !self.raw_output {
            if let Some(ref mut sr) = self.streaming_renderer {
                sr.push(delta);
            } else {
                let renderer = leaf_core::MarkdownRenderer::new();
                let width = self.last_rendered_width.max(80) as usize;
                let mut sr = StreamingRenderer::new(renderer, width).with_dual_phase(true);
                sr.push(delta);
                self.streaming_renderer = Some(sr);
            }
        }

        self.dirty = true;
    }

    fn append_thinking_delta(&mut self, delta: &str) {
        self.pending_thinking.push_str(delta);
        self.dirty = true;
    }

    fn finish_streaming(&mut self) {
        if let Some(ref mut sr) = self.streaming_renderer {
            let final_update = sr.finish();
            let last_streaming_msg = self.items.iter_mut().rev().find_map(|item| match item {
                ChatItem::Message(msg) if msg.role == Role::Assistant && msg.streaming => Some(msg),
                _ => None,
            });
            if let Some(msg) = last_streaming_msg {
                msg.cached_lines = final_update.lines.to_vec();
                msg.cached_width = self.last_rendered_width;
            }
        }
        self.streaming_renderer = None;

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
        self.dirty = true;
    }

    fn show_tool_message(&mut self, line: String) {
        self.items
            .push(ChatItem::Message(Message::new(Role::System, line)));
        if self.scroll_from_bottom == 0 {
            // keep pinned
        }
        self.dirty = true;
    }

    fn show_error(&mut self, err: AgentError) {
        self.items.push(ChatItem::Message(Message::new(
            Role::Error,
            format!("{err}"),
        )));
        if self.scroll_from_bottom == 0 {
            // keep pinned
        }
        self.dirty = true;
    }

    pub(crate) fn thinking_snapshot(&self) -> Option<(String, bool)> {
        if !self.pending_thinking.trim().is_empty() {
            return Some((self.pending_thinking.clone(), true));
        }
        if !self.last_thinking.trim().is_empty() {
            return Some((self.last_thinking.clone(), false));
        }
        None
    }

    fn thinking_collapsed_line(&self) -> Line<'static> {
        let has_pending_thinking = !self.pending_thinking.trim().is_empty();
        let label = if has_pending_thinking {
            "Thinking…"
        } else {
            "Thinking"
        };

        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        Line::from(Span::styled(label, style))
    }

    pub(crate) fn render_live_preview(&mut self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let width = area.width.max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Render full transcript in the viewport (keep newest lines).
        for item in &mut self.items {
            let ChatItem::Message(msg) = item;

            if msg.dirty || msg.cached_width != width {
                msg.cached_width = width;
                msg.cached_lines = if self.raw_output {
                    msg.content
                        .trim_end_matches(['\r', '\n'])
                        .lines()
                        .map(|l| Line::from(l.to_string()))
                        .collect::<Vec<_>>()
                } else if msg.streaming {
                    if let Some(ref mut sr) = self.streaming_renderer {
                        sr.set_width(width as usize);
                        if let Some(update) = sr.tick() {
                            update.lines.to_vec()
                        } else {
                            sr.force_tick().lines.to_vec()
                        }
                    } else {
                        self.markdown.render(&msg.content, width)
                    }
                } else {
                    self.markdown.render(&msg.content, width)
                };
                msg.dirty = false;
            }

            match msg.role {
                Role::User => {
                    lines.extend(chat_style::prefix_user_lines(msg.cached_lines.clone()));
                    lines.push(Line::from(""));
                }
                Role::Assistant => {
                    lines.extend(chat_style::prefix_assistant_lines(
                        msg.cached_lines.clone(),
                        /*streaming*/ msg.streaming,
                    ));
                    lines.push(Line::from(""));
                }
                Role::System => {
                    let mut system_lines = msg.cached_lines.clone();
                    for line in &mut system_lines {
                        line.style = Style::default().fg(Color::DarkGray);
                    }
                    lines.extend(system_lines);
                    lines.push(Line::from(""));
                }
                Role::Error => {
                    let mut error_lines = msg.cached_lines.clone();
                    for line in &mut error_lines {
                        line.style = Style::default().fg(Color::Red);
                    }
                    lines.extend(error_lines);
                    lines.push(Line::from(""));
                }
            }
        }

        // Thinking display (rendered at the bottom, close to the latest activity):
        // - While streaming: show a single collapsed line.
        // - After completion: keep a collapsed placeholder so users can expand it.
        let has_any_thinking =
            !self.pending_thinking.trim().is_empty() || !self.last_thinking.trim().is_empty();
        if has_any_thinking {
            if !lines.is_empty() && !lines.last().is_some_and(|l| l.spans.is_empty()) {
                lines.push(Line::from(""));
            }
            lines.push(self.thinking_collapsed_line());
        }

        // Keep scroll stable while new content streams in: if the user has scrolled up (i.e. not
        // pinned to bottom), maintain the same "end" index by increasing the distance from the
        // bottom as new lines are appended.
        let full_len = lines.len();
        if self.last_rendered_width == width
            && self.scroll_from_bottom > 0
            && full_len > self.last_rendered_line_count
        {
            let delta = full_len - self.last_rendered_line_count;
            self.scroll_from_bottom = self.scroll_from_bottom.saturating_add(delta);
        }
        self.last_rendered_width = width;
        self.last_rendered_line_count = full_len;

        // Window the transcript into the viewport with a scroll offset from the bottom.
        let height = area.height.max(1) as usize;
        if lines.len() > height {
            let max_scroll = lines.len().saturating_sub(height);
            self.scroll_from_bottom = self.scroll_from_bottom.min(max_scroll);

            let end = lines.len().saturating_sub(self.scroll_from_bottom);
            let start = end.saturating_sub(height);
            lines = lines[start..end].to_vec();
        } else {
            self.scroll_from_bottom = 0;
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
                use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    return EventResult::default();
                }

                if key.modifiers.is_empty() {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.scroll_up(1);
                            return EventResult::consumed();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.scroll_down(1);
                            return EventResult::consumed();
                        }
                        KeyCode::PageUp => {
                            self.scroll_up(10);
                            return EventResult::consumed();
                        }
                        KeyCode::PageDown => {
                            self.scroll_down(10);
                            return EventResult::consumed();
                        }
                        KeyCode::Char('g') => {
                            self.scroll_from_bottom = usize::MAX;
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        KeyCode::Char('G') => {
                            self.scroll_to_bottom();
                            return EventResult::consumed();
                        }
                        _ => {}
                    }
                }

                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('u') => {
                            self.scroll_up(10);
                            return EventResult::consumed();
                        }
                        KeyCode::Char('d') => {
                            self.scroll_down(10);
                            return EventResult::consumed();
                        }
                        _ => {}
                    }
                }
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

    #[test]
    fn streaming_renderer_activates_on_text_delta() {
        let mut chat = ChatComponent::new(MarkdownRenderer::default());
        assert!(chat.streaming_renderer.is_none());

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::TextDelta(
            "Hello **world**".to_string(),
        )));

        assert!(
            chat.streaming_renderer.is_some(),
            "StreamingRenderer should activate on first text delta"
        );
    }

    #[test]
    fn streaming_renderer_clears_on_step_complete() {
        let mut chat = ChatComponent::new(MarkdownRenderer::default());

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::TextDelta("# Hi".to_string())));
        assert!(chat.streaming_renderer.is_some());

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::StepComplete {
            step: 1,
            summary: String::new(),
        }));

        assert!(
            chat.streaming_renderer.is_none(),
            "StreamingRenderer should be cleared after finish"
        );
    }

    #[test]
    fn raw_output_skips_streaming_renderer() {
        let mut chat = ChatComponent::new(MarkdownRenderer::default());
        chat.set_raw_output(true);

        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::TextDelta(
            "plain text".to_string(),
        )));

        assert!(
            chat.streaming_renderer.is_none(),
            "StreamingRenderer should not activate in raw_output mode"
        );
    }
}

fn extract_thinking_blocks(text: &mut String) -> Vec<String> {
    let mut blocks = Vec::new();

    let candidates = [
        ("thinking", "<thinking>", "</thinking>"),
        ("think", "<think>", "</think>"),
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
