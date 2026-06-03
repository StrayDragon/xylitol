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
use super::event::{AppAction, TuiEvent};
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
    thinking_visible: bool,
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
            thinking_visible: false,
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
    tool_names: HashMap<String, String>,
    /// Index of the message under the cursor (for thinking toggle and visual selection).
    cursor_index: usize,
    /// Visual selection state.
    select: SelectState,
    /// Whether the chat pane has keyboard focus.
    focused: bool,
    scroll_from_bottom: usize,
    last_rendered_width: u16,
    last_rendered_line_count: usize,
    /// Maps each rendered line index to its source message index.
    last_line_message_indices: Vec<usize>,
    dirty: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectState {
    pub(crate) active: bool,
    pub(crate) anchor: usize,
    pub(crate) cursor: usize,
}

impl ChatComponent {
    pub(crate) fn new(markdown: MarkdownRenderer) -> Self {
        Self {
            markdown,
            streaming_renderer: None,
            items: Vec::new(),
            raw_output: false,
            pending_thinking: String::new(),
            tool_names: HashMap::new(),
            cursor_index: 0,
            select: SelectState::default(),
            focused: false,
            scroll_from_bottom: 0,
            last_rendered_width: 0,
            last_rendered_line_count: 0,
            last_line_message_indices: Vec::new(),
            dirty: true,
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
        self.tool_names.clear();
        self.cursor_index = 0;
        self.select = SelectState::default();
        self.last_line_message_indices.clear();
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
        // Buffer content internally — streaming messages are hidden from view.
        // They become visible only on finish_streaming().
        let last_streaming = self.items.iter_mut().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant && msg.streaming => Some(msg),
            _ => None,
        });

        if let Some(msg) = last_streaming {
            msg.content.push_str(delta);
        } else {
            let mut m = Message::new(Role::Assistant, delta.to_string());
            m.streaming = true;
            self.items.push(ChatItem::Message(m));
        }

        // Feed streaming renderer for later use, but don't mark dirty —
        // streaming messages are not rendered.
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
        // NOTE: deliberately NOT setting self.dirty — streaming messages are hidden.
    }

    fn append_thinking_delta(&mut self, delta: &str) {
        self.pending_thinking.push_str(delta);
        // NOTE: deliberately NOT setting self.dirty — streaming is hidden.
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

        // Store thinking in the message (per-message, not global).
        let combined_thinking = thinking_parts.join("\n\n").trim().to_string();
        if let Some(idx) = last_assistant_index
            && let Some(ChatItem::Message(msg)) = self.items.get_mut(idx)
        {
            if combined_thinking.is_empty() {
                msg.thinking = None;
            } else {
                msg.thinking = Some(combined_thinking);
            }
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

    /// Get thinking content from the message at cursor (for overlay display).
    pub(crate) fn thinking_snapshot(&self) -> Option<(String, bool)> {
        if !self.pending_thinking.trim().is_empty() {
            return Some((self.pending_thinking.clone(), true));
        }
        // Look at the message at cursor_index.
        if let Some(ChatItem::Message(msg)) = self.items.get(self.cursor_index)
            && let Some(ref thinking) = msg.thinking
            && !thinking.trim().is_empty()
        {
            return Some((thinking.clone(), false));
        }
        None
    }

    /// Toggle thinking visibility for the message at cursor.
    pub(crate) fn toggle_thinking_at_cursor(&mut self) {
        if let Some(ChatItem::Message(msg)) = self.items.get_mut(self.cursor_index)
            && msg.thinking.is_some()
        {
            msg.thinking_visible = !msg.thinking_visible;
            msg.dirty = true;
            self.dirty = true;
        }
    }

    /// Toggle thinking visibility for all messages.
    pub(crate) fn toggle_all_thinking(&mut self) {
        // Determine new state: if majority are visible, collapse all; else expand all.
        let visible_count = self
            .items
            .iter()
            .filter(|item| {
                let ChatItem::Message(msg) = item;
                msg.thinking_visible
            })
            .count();
        let has_thinking_count = self
            .items
            .iter()
            .filter(|item| {
                let ChatItem::Message(msg) = item;
                msg.thinking.is_some()
            })
            .count();
        let new_visible = if has_thinking_count == 0 {
            return;
        } else {
            visible_count < has_thinking_count
        };
        for item in &mut self.items {
            let ChatItem::Message(msg) = item;
            if msg.thinking.is_some() {
                msg.thinking_visible = new_visible;
                msg.dirty = true;
            }
        }
        self.dirty = true;
    }

    fn thinking_header_line(visible: bool, is_streaming: bool) -> Line<'static> {
        let indicator = if visible { "[-]" } else { "[+]" };
        let label = if is_streaming {
            format!(" {indicator} Thinking…")
        } else {
            format!(" {indicator} Thinking")
        };
        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);
        Line::from(Span::styled(label, style))
    }

    /// Separator line between thinking block and message content.
    fn thinking_separator() -> Line<'static> {
        Line::from(Span::styled(
            " ┄".to_string(),
            Style::default().fg(Color::DarkGray),
        ))
    }

    // ── Visual selection helpers ────────────────────────────────

    fn select_extend_down(&mut self) {
        let max = self.items.len().saturating_sub(1);
        self.select.cursor = (self.select.cursor + 1).min(max);
        self.dirty = true;
    }

    fn select_extend_up(&mut self) {
        self.select.cursor = self.select.cursor.saturating_sub(1);
        self.dirty = true;
    }

    fn collect_selected_text(&self, raw: bool) -> String {
        let lo = self.select.anchor.min(self.select.cursor);
        let hi = self.select.anchor.max(self.select.cursor);
        let mut out = String::new();
        for item in self.items.iter().take(hi + 1).skip(lo) {
            let ChatItem::Message(msg) = item;
            if raw {
                out.push_str(&msg.content);
            } else {
                let label = match msg.role {
                    Role::User => "> user: ",
                    Role::Assistant => "assistant: ",
                    Role::System => "system: ",
                    Role::Error => "error: ",
                };
                out.push_str(label);
                out.push_str(&msg.content);
            }
            out.push('\n');
        }
        out.trim().to_string()
    }

    fn message_at_cursor_text(&self, raw: bool) -> String {
        if let Some(ChatItem::Message(msg)) = self.items.get(self.cursor_index) {
            if raw {
                msg.content.clone()
            } else {
                let label = match msg.role {
                    Role::User => "> user: ",
                    Role::Assistant => "assistant: ",
                    Role::System => "system: ",
                    Role::Error => "error: ",
                };
                format!("{label}{}", msg.content)
            }
        } else {
            String::new()
        }
    }

    /// Reset visual selection state (called when focus leaves chat).
    pub(crate) fn reset_selection(&mut self) {
        if self.select.active {
            self.select.active = false;
            self.dirty = true;
        }
    }

    /// Get the message text at a viewport row (0-based from top of chat area).
    /// Used for click-to-copy behavior.
    pub(crate) fn message_at_viewport_row(&self, row: usize) -> Option<String> {
        let height = self.last_rendered_line_count;
        if height == 0 || self.last_line_message_indices.is_empty() {
            return None;
        }
        // Compute viewport window (same logic as render_live_preview).
        let full_len = self.last_line_message_indices.len();
        let max_scroll = full_len.saturating_sub(height);
        let scroll = self.scroll_from_bottom.min(max_scroll);
        let end = full_len.saturating_sub(scroll);
        let start = end.saturating_sub(height);
        let viewport_row = start + row;
        let &msg_idx = self.last_line_message_indices.get(viewport_row)?;
        if let Some(ChatItem::Message(msg)) = self.items.get(msg_idx) {
            let label = match msg.role {
                Role::User => "> ",
                Role::Assistant => "",
                Role::System => "[system] ",
                Role::Error => "[error] ",
            };
            Some(format!("{label}{}", msg.content))
        } else {
            None
        }
    }

    pub(crate) fn render_live_preview(&mut self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let width = area.width.max(1);
        let mut lines: Vec<Line<'static>> = Vec::new();
        // Track which message index each line belongs to (for cursor tracking).
        let mut line_message_indices: Vec<usize> = Vec::new();

        // Render full transcript in the viewport (keep newest lines).
        // Skip streaming messages — they are hidden until finish_streaming().
        for (msg_idx, item) in self.items.iter_mut().enumerate() {
            let ChatItem::Message(msg) = item;
            if msg.streaming {
                continue;
            }

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

            // Render thinking block inline before assistant messages.
            if msg.role == Role::Assistant {
                let has_thinking = msg.thinking.as_ref().is_some_and(|t| !t.trim().is_empty());
                let has_pending = msg.streaming && !self.pending_thinking.trim().is_empty();

                if has_thinking || has_pending {
                    let is_streaming = msg.streaming && has_pending;
                    lines.push(Self::thinking_header_line(
                        msg.thinking_visible,
                        is_streaming,
                    ));
                    line_message_indices.push(msg_idx);

                    if msg.thinking_visible {
                        // Render thinking content in dim italic style.
                        let thinking_text = if has_pending {
                            &self.pending_thinking
                        } else {
                            msg.thinking.as_deref().unwrap_or("")
                        };
                        for tline in thinking_text.lines() {
                            let styled = Line::from(Span::styled(
                                format!(" │ {tline}"),
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC),
                            ));
                            lines.push(styled);
                            line_message_indices.push(msg_idx);
                        }
                        // Separator between thinking and message content.
                        lines.push(Self::thinking_separator());
                        line_message_indices.push(msg_idx);
                    }
                }
            }

            match msg.role {
                Role::User => {
                    for line in chat_style::prefix_user_lines(msg.cached_lines.clone()) {
                        lines.push(line);
                        line_message_indices.push(msg_idx);
                    }
                    lines.push(Line::from(""));
                    line_message_indices.push(msg_idx);
                }
                Role::Assistant => {
                    for line in chat_style::prefix_assistant_lines(
                        msg.cached_lines.clone(),
                        /*streaming*/ msg.streaming,
                    ) {
                        lines.push(line);
                        line_message_indices.push(msg_idx);
                    }
                    lines.push(Line::from(""));
                    line_message_indices.push(msg_idx);
                }
                Role::System => {
                    let mut system_lines = msg.cached_lines.clone();
                    for line in &mut system_lines {
                        line.style = Style::default().fg(Color::DarkGray);
                    }
                    for line in system_lines {
                        line_message_indices.push(msg_idx);
                        lines.push(line);
                    }
                    lines.push(Line::from(""));
                    line_message_indices.push(msg_idx);
                }
                Role::Error => {
                    let mut error_lines = msg.cached_lines.clone();
                    for line in &mut error_lines {
                        line.style = Style::default().fg(Color::Red);
                    }
                    for line in error_lines {
                        line_message_indices.push(msg_idx);
                        lines.push(line);
                    }
                    lines.push(Line::from(""));
                    line_message_indices.push(msg_idx);
                }
            }
        }

        // Keep scroll stable while new content streams in.
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
        let (mut visible_lines, visible_indices) = if lines.len() > height {
            let max_scroll = lines.len().saturating_sub(height);
            self.scroll_from_bottom = self.scroll_from_bottom.min(max_scroll);

            let end = lines.len().saturating_sub(self.scroll_from_bottom);
            let start = end.saturating_sub(height);
            // Store full line-message mapping before slicing.
            self.last_line_message_indices = line_message_indices.clone();
            (
                lines[start..end].to_vec(),
                &line_message_indices[start..end],
            )
        } else {
            self.scroll_from_bottom = 0;
            self.last_line_message_indices = line_message_indices.clone();
            (lines, line_message_indices.as_slice())
        };

        // Update cursor_index to the last visible message.
        if let Some(&last_idx) = visible_indices.last() {
            self.cursor_index = last_idx;
        }

        // Apply visual selection highlight.
        if self.select.active {
            let lo = self.select.anchor.min(self.select.cursor);
            let hi = self.select.anchor.max(self.select.cursor);
            let highlight_bg = Color::DarkGray;
            for (i, line) in visible_lines.iter_mut().enumerate() {
                if let Some(&msg_idx) = visible_indices.get(i)
                    && msg_idx >= lo
                    && msg_idx <= hi
                {
                    line.style = Style::default().bg(highlight_bg);
                }
            }
        }

        // Show focus indicator: subtle bar on the left when chat is focused.
        if self.focused && !self.select.active {
            for line in &mut visible_lines {
                // Prepend a focus indicator as the first span.
                line.spans
                    .insert(0, Span::styled("▎", Style::default().fg(Color::DarkGray)));
            }
        }

        let para = Paragraph::new(visible_lines)
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

                    // Include thinking before assistant message content.
                    if msg.role == Role::Assistant
                        && let Some(thinking) = msg.thinking.as_deref()
                        && !thinking.trim().is_empty()
                    {
                        out.push_str("### Thinking\n\n");
                        out.push_str(thinking);
                        out.push_str("\n\n");
                    }

                    out.push_str(&format!("## {label}\n\n"));
                    out.push_str(&msg.content);
                    out.push_str("\n\n");
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

                // Visual selection mode keys.
                if self.select.active && key.modifiers.is_empty() {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            self.select_extend_down();
                            return EventResult::consumed();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.select_extend_up();
                            return EventResult::consumed();
                        }
                        KeyCode::Char('g') => {
                            self.select.cursor = 0;
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        KeyCode::Char('G') => {
                            self.select.cursor = self.items.len().saturating_sub(1);
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        KeyCode::Char('y') => {
                            // Copy selected messages with role prefixes.
                            let text = self.collect_selected_text(false);
                            self.select.active = false;
                            self.dirty = true;
                            if !text.is_empty() {
                                return EventResult::action(AppAction::CopyText(text));
                            }
                            return EventResult::consumed();
                        }
                        KeyCode::Char('Y') => {
                            // Copy selected messages as raw text.
                            let text = self.collect_selected_text(true);
                            self.select.active = false;
                            self.dirty = true;
                            if !text.is_empty() {
                                return EventResult::action(AppAction::CopyText(text));
                            }
                            return EventResult::consumed();
                        }
                        KeyCode::Esc => {
                            self.select.active = false;
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        _ => {}
                    }
                }

                if key.modifiers.is_empty() {
                    match key.code {
                        // Thinking toggle.
                        KeyCode::Char('t') => {
                            self.toggle_thinking_at_cursor();
                            return EventResult::consumed();
                        }
                        KeyCode::Char('T') => {
                            self.toggle_all_thinking();
                            return EventResult::consumed();
                        }
                        // Enter visual selection mode.
                        KeyCode::Char('v') => {
                            self.select.active = true;
                            self.select.anchor = self.cursor_index;
                            self.select.cursor = self.cursor_index;
                            self.dirty = true;
                            return EventResult::consumed();
                        }
                        // Copy message under cursor.
                        KeyCode::Char('y') => {
                            let text = self.message_at_cursor_text(false);
                            if !text.is_empty() {
                                return EventResult::action(AppAction::CopyText(text));
                            }
                            return EventResult::consumed();
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_deltas_accumulate_into_thinking_panel_on_step_complete() {
        let mut chat = ChatComponent::new(MarkdownRenderer::default());

        // Need a message to attach thinking to.
        chat.add_user_message("test");
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::TextDelta("hi".to_string())));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ThinkingDelta("a".to_string())));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::ThinkingDelta("b".to_string())));
        let _ = chat.handle_event(&TuiEvent::Agent(AgentEvent::StepComplete {
            step: 1,
            summary: String::new(),
        }));

        // Thinking should be stored in the assistant message.
        let last_msg = chat.items.iter().rev().find_map(|item| match item {
            ChatItem::Message(msg) if msg.role == Role::Assistant => Some(msg),
            _ => None,
        });
        assert!(last_msg.is_some());
        assert_eq!(last_msg.unwrap().thinking.as_deref(), Some("ab"));
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
