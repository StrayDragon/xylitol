//! CLI terminal-based diff review with ratatui.
//!
//! Provides a full-screen terminal interface for reviewing file diffs,
//! adding line-level comments, and accepting/rejecting changes.
//!
//! ## Key bindings
//!
//! | Key | Action |
//! |-----|--------|
//! | `j` / `Down` | Move cursor down |
//! | `k` / `Up` | Move cursor up |
//! | `g` | Go to top |
//! | `G` | Go to bottom |
//! | `c` | Add/edit comment on current line |
//! | `v` | View comment on current line |
//! | `n` / `N` | Next / previous commented line |
//! | `Ctrl+a` | Accept all changes |
//! | `Ctrl+r` | Reject with comments |
//! | `?` | Toggle help |

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use super::types::{
    CommentSeverity, DiffLine, DiffLineKind, ReviewComment, ReviewSession, ReviewVerdict,
};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Internal state for the TUI review application.
struct ReviewApp {
    session: ReviewSession,
    /// Currently selected hunk index.
    hunk_idx: usize,
    /// Currently selected line index within the current hunk.
    line_idx: usize,
    /// Whether we're in comment input mode.
    comment_mode: bool,
    /// Current comment text being typed.
    comment_buffer: String,
    /// Comment cursor position within the buffer.
    comment_cursor: usize,
    /// Whether to show the help overlay.
    show_help: bool,
    /// Terminal width for word-wrap calculations.
    term_width: u16,
    /// Scroll offset for the diff view.
    scroll_offset: usize,
    /// Total number of visible lines in the rendered diff.
    total_visible_lines: usize,
    /// Height of the visible content area in the body (set during render).
    body_height: usize,
    /// Whether to show a popup overlay.
    show_popup: bool,
    /// The popup message to display.
    popup_message: String,
    /// The final verdict, set when the user completes.
    verdict: Option<ReviewVerdict>,
}

impl ReviewApp {
    fn new(session: ReviewSession) -> Self {
        Self {
            session,
            hunk_idx: 0,
            line_idx: 0,
            comment_mode: false,
            comment_buffer: String::new(),
            comment_cursor: 0,
            show_help: false,
            term_width: 80,
            scroll_offset: 0,
            total_visible_lines: 0,
            body_height: 0,
            show_popup: false,
            popup_message: String::new(),
            verdict: None,
        }
    }

    /// Return a reference to the currently selected diff line, if any.
    fn current_line(&self) -> Option<&DiffLine> {
        self.session
            .hunks
            .get(self.hunk_idx)
            .and_then(|h| h.lines.get(self.line_idx))
    }

    /// Move selection up.
    fn move_up(&mut self) {
        if self.line_idx > 0 {
            self.line_idx -= 1;
        } else if self.hunk_idx > 0 {
            self.hunk_idx -= 1;
            self.line_idx = self.session.hunks[self.hunk_idx]
                .lines
                .len()
                .saturating_sub(1);
        }
        self.adjust_scroll();
    }

    /// Move selection down.
    fn move_down(&mut self) {
        if let Some(hunk) = self.session.hunks.get(self.hunk_idx) {
            if self.line_idx + 1 < hunk.lines.len() {
                self.line_idx += 1;
            } else if self.hunk_idx + 1 < self.session.hunks.len() {
                self.hunk_idx += 1;
                self.line_idx = 0;
            }
        }
        self.adjust_scroll();
    }

    /// Go to the top of the diff.
    fn go_top(&mut self) {
        self.hunk_idx = 0;
        self.line_idx = 0;
        self.scroll_offset = 0;
    }

    /// Go to the bottom of the diff.
    fn go_bottom(&mut self) {
        self.hunk_idx = self.session.hunks.len().saturating_sub(1);
        let last_len = self
            .session
            .hunks
            .last()
            .map(|h| h.lines.len())
            .unwrap_or(0);
        self.line_idx = last_len.saturating_sub(1);
        self.scroll_offset = self
            .total_visible_lines
            .saturating_sub(self.body_height.max(1));
    }

    /// Adjust scroll offset to keep selection visible.
    fn adjust_scroll(&mut self) {
        let cursor_line = self.absolute_cursor_line();
        let visible = self.body_height.max(1);
        if cursor_line < self.scroll_offset {
            self.scroll_offset = cursor_line;
        } else if cursor_line >= self.scroll_offset + visible {
            self.scroll_offset = cursor_line.saturating_sub(visible) + 1;
        }
    }

    /// Get the absolute line number of the current selection across all hunks.
    fn absolute_cursor_line(&self) -> usize {
        let mut line = 0;
        for (i, hunk) in self.session.hunks.iter().enumerate() {
            if i < self.hunk_idx {
                line += hunk.lines.len() + 2; // +1 header +1 trailing blank
            } else if i == self.hunk_idx {
                line += self.line_idx + 1; // +1 for current hunk's header
                break;
            }
        }
        line
    }

    /// Accept all changes (comments are still captured).
    fn accept(&mut self) {
        self.verdict = Some(ReviewVerdict::AcceptAll(self.session.comments.clone()));
    }

    /// Reject with collected comments.
    fn reject(&mut self) {
        let comments = self.session.comments.clone();
        self.verdict = Some(ReviewVerdict::RejectWithComments(comments));
    }

    /// Start comment input for the current line.
    /// If the line already has a comment, loads it for editing.
    fn start_comment(&mut self) {
        self.comment_mode = true;
        self.comment_buffer.clear();
        self.comment_cursor = 0;
        if let Some(c) = self.current_line_comments().first() {
            self.comment_buffer = c.content.clone();
            self.comment_cursor = self.comment_buffer.len();
        }
    }

    /// Submit the current comment. Updates existing comment or adds a new one.
    fn submit_comment(&mut self) {
        if !self.comment_buffer.is_empty() {
            let line = match self.current_line() {
                Some(l) => l.clone(),
                None => {
                    self.comment_mode = false;
                    self.comment_buffer.clear();
                    self.comment_cursor = 0;
                    return;
                }
            };
            let line_no = line.old_line_no.max(line.new_line_no);
            let file = self.session.hunks[self.hunk_idx].file.clone();

            if let Some(existing) = self
                .session
                .comments
                .iter_mut()
                .find(|c| c.file == file && (c.line_start == line_no || c.line_end == line_no))
            {
                existing.content = self.comment_buffer.clone();
            } else {
                self.session.add_comment(ReviewComment {
                    file,
                    line_start: line_no,
                    line_end: line_no,
                    content: self.comment_buffer.clone(),
                    severity: CommentSeverity::Info,
                });
            }
        }
        self.comment_mode = false;
        self.comment_buffer.clear();
        self.comment_cursor = 0;
    }

    /// Cancel comment input.
    fn cancel_comment(&mut self) {
        self.comment_mode = false;
        self.comment_buffer.clear();
        self.comment_cursor = 0;
    }

    /// Add a character to the comment buffer.
    fn comment_char(&mut self, c: char) {
        self.comment_buffer.insert(self.comment_cursor, c);
        self.comment_cursor += c.len_utf8();
    }

    /// Delete character before cursor in comment buffer.
    fn comment_backspace(&mut self) {
        if self.comment_cursor > 0 {
            let prev = (0..self.comment_cursor)
                .rev()
                .find(|&i| self.comment_buffer.is_char_boundary(i))
                .unwrap_or(0);
            self.comment_buffer.remove(prev);
            self.comment_cursor = prev;
        }
    }

    /// Check if a specific line has any comments.
    fn line_has_comment(&self, hi: usize, li: usize) -> bool {
        let hunk = match self.session.hunks.get(hi) {
            Some(h) => h,
            None => return false,
        };
        let line = match hunk.lines.get(li) {
            Some(l) => l,
            None => return false,
        };
        let line_no = line.old_line_no.max(line.new_line_no);
        self.session
            .comments
            .iter()
            .any(|c| c.file == hunk.file && (c.line_start == line_no || c.line_end == line_no))
    }

    /// Get comments attached to the current line.
    fn current_line_comments(&self) -> Vec<&ReviewComment> {
        let hunk = match self.session.hunks.get(self.hunk_idx) {
            Some(h) => h,
            None => return vec![],
        };
        let line = match hunk.lines.get(self.line_idx) {
            Some(l) => l,
            None => return vec![],
        };
        let line_no = line.old_line_no.max(line.new_line_no);
        self.session
            .comments
            .iter()
            .filter(|c| c.file == hunk.file && (c.line_start == line_no || c.line_end == line_no))
            .collect()
    }

    /// Return all (hunk_idx, line_idx) positions that have comments.
    fn commented_lines(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for (hi, hunk) in self.session.hunks.iter().enumerate() {
            for (li, line) in hunk.lines.iter().enumerate() {
                let line_no = line.old_line_no.max(line.new_line_no);
                if self.session.comments.iter().any(|c| {
                    c.file == hunk.file && (c.line_start == line_no || c.line_end == line_no)
                }) {
                    result.push((hi, li));
                }
            }
        }
        result
    }

    /// Jump to the next commented line.
    fn next_comment(&mut self) {
        let positions = self.commented_lines();
        if positions.is_empty() {
            return;
        }
        let current = (self.hunk_idx, self.line_idx);
        let next = positions
            .iter()
            .find(|&&p| p > current)
            .unwrap_or(&positions[0]);
        self.hunk_idx = next.0;
        self.line_idx = next.1;
        self.adjust_scroll();
    }

    /// Jump to the previous commented line.
    fn prev_comment(&mut self) {
        let positions = self.commented_lines();
        if positions.is_empty() {
            return;
        }
        let current = (self.hunk_idx, self.line_idx);
        let prev = positions
            .iter()
            .rev()
            .find(|&&p| p < current)
            .unwrap_or(&positions[positions.len() - 1]);
        self.hunk_idx = prev.0;
        self.line_idx = prev.1;
        self.adjust_scroll();
    }

    /// Show a popup with the comment on the current line.
    fn view_comment(&mut self) {
        // Extract info first to avoid borrow conflict.
        let info: Vec<(String, u32, String, String)> = self
            .current_line_comments()
            .iter()
            .map(|c| {
                (
                    c.file.clone(),
                    c.line_start,
                    c.severity.label().to_string(),
                    c.content.clone(),
                )
            })
            .collect();
        if info.is_empty() {
            return;
        }
        let (file, line_start, sev, content) = &info[0];
        self.popup_message = format!("Line {}:{} — {}:\n\n{}", file, line_start, sev, content);
        if info.len() > 1 {
            self.popup_message.push_str(&format!(
                "\n\n(+ {} more comment{})",
                info.len() - 1,
                plural(info.len() - 1)
            ));
        }
        self.show_popup = true;
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render the full review UI into the given frame.
fn render_app(frame: &mut Frame, app: &mut ReviewApp) {
    let area = frame.area();
    app.term_width = area.width;

    // Layout: header | diff body
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    render_header(frame, chunks[0], app);
    render_body(frame, chunks[1], app);

    // Help overlay
    if app.show_help {
        render_help(frame, area);
    }

    // Comment popup (rendered on top of everything)
    if app.show_popup {
        render_popup(frame, area, &app.popup_message);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &ReviewApp) {
    let hunks = app.session.hunks.len();
    let comments = app.session.comments.len();
    let cursor = app.absolute_cursor_line().saturating_add(1);
    let id_short: String = app.session.review_id.chars().take(8).collect();

    // Left: identity
    let left = format!(
        " Diff Review [{}] — {} hunk{} ",
        id_short,
        hunks,
        plural(hunks)
    );
    // Right: line + comments
    let right = format!(
        " Line {}/{} · {} comment{} ",
        cursor,
        app.total_visible_lines,
        comments,
        plural(comments),
    );

    let text = Line::from(vec![
        Span::styled(
            left,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│"),
        Span::styled(right, Style::default().fg(Color::Yellow)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let para = Paragraph::new(text).block(block);
    frame.render_widget(para, area);
}

fn render_body(frame: &mut Frame, area: Rect, app: &mut ReviewApp) {
    // Split area if in comment mode (diff + input area)
    let (diff_area, input_area) = if app.comment_mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    app.body_height = (diff_area.height.saturating_sub(2)) as usize;

    if app.session.hunks.is_empty() {
        let text = Text::from(Line::from("No changes to review."));
        let para = Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, diff_area);
        return;
    }

    // Build visible lines.
    let mut all_lines: Vec<Line> = Vec::new();
    let mut line_map: Vec<(usize, usize)> = Vec::new(); // (hunk_idx, line_idx)

    for (hi, hunk) in app.session.hunks.iter().enumerate() {
        // Hunk header
        let header = format!(
            "@@ -{},{} +{},{} @@ {}",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count, hunk.file
        );
        all_lines.push(Line::from(Span::styled(
            header,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        line_map.push((hi, usize::MAX)); // header marker

        for (li, line) in hunk.lines.iter().enumerate() {
            let is_this_line = hi == app.hunk_idx && li == app.line_idx;
            let has_comment = app.line_has_comment(hi, li);
            let prefix = match line.kind {
                DiffLineKind::Add => "+",
                DiffLineKind::Delete => "-",
                DiffLineKind::Context => " ",
            };
            let (fg, _) = match line.kind {
                DiffLineKind::Add => (Color::Green, Style::default()),
                DiffLineKind::Delete => (Color::Red, Style::default()),
                DiffLineKind::Context => (Color::Reset, Style::default()),
            };

            let content = &line.content;
            let display = if content.ends_with('\n') {
                &content[..content.len().saturating_sub(1)]
            } else {
                content
            };

            let style = if app.comment_mode && is_this_line {
                // Comment mode highlight (gold)
                Style::default()
                    .bg(Color::Rgb(60, 50, 20))
                    .fg(fg)
                    .add_modifier(Modifier::BOLD)
            } else if !app.comment_mode && is_this_line {
                // Navigation selection (blue)
                Style::default()
                    .bg(Color::Rgb(50, 50, 80))
                    .fg(fg)
                    .add_modifier(Modifier::BOLD)
            } else if has_comment {
                // Has a comment (subtle tint)
                Style::default().bg(Color::Rgb(40, 35, 20)).fg(fg)
            } else {
                Style::default().fg(fg)
            };

            let line_no = match line.kind {
                DiffLineKind::Add => format!("    {}", line.new_line_no),
                DiffLineKind::Delete => format!("{}    ", line.old_line_no),
                DiffLineKind::Context => format!("{:<4}{}", line.old_line_no, line.new_line_no),
            };

            let marker = if has_comment { " ●" } else { "" };
            let line_content = format!("{}{}|{}{}", prefix, line_no, display, marker);
            all_lines.push(Line::from(Span::styled(line_content, style)));
            line_map.push((hi, li));
        }

        // Blank line at hunk end
        all_lines.push(Line::from(""));
        line_map.push((hi, usize::MAX));
    }

    app.total_visible_lines = all_lines.len();

    // Apply scroll offset.
    let visible_lines: Vec<Line> = if app.scroll_offset < all_lines.len() {
        all_lines[app.scroll_offset..].to_vec()
    } else {
        all_lines.clone()
    };

    let text = Text::from(visible_lines);
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, diff_area);

    // Comment mode input area
    if let Some(input_area) = input_area {
        render_comment_input(frame, input_area, app);
    }
}

fn render_comment_input(frame: &mut Frame, area: Rect, app: &ReviewApp) {
    let before = &app.comment_buffer[..app.comment_cursor];
    let after = &app.comment_buffer[app.comment_cursor..];
    let text = if app.comment_buffer.is_empty() {
        Line::from(Span::styled(" █  ", Style::default().fg(Color::Yellow)))
    } else {
        Line::from(vec![
            Span::styled(" ", Style::default().fg(Color::Yellow)),
            Span::styled(before, Style::default().fg(Color::Yellow)),
            Span::styled(
                "█",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(after, Style::default().fg(Color::Yellow)),
        ])
    };

    let is_editing = app.line_has_comment(app.hunk_idx, app.line_idx);
    let title = if is_editing {
        " Edit Comment "
    } else {
        " Comment "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));
    let para = Paragraph::new(Text::from(text)).block(block);
    frame.render_widget(para, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_lines = vec![
        Line::from("  j / Up      Move cursor down"),
        Line::from("  k / Down    Move cursor up"),
        Line::from("  g           Go to top"),
        Line::from("  G           Go to bottom"),
        Line::from("  c           Add comment on current line"),
        Line::from("  v           View comment on current line"),
        Line::from("  n / N       Next / previous commented line"),
        Line::from("  Ctrl+a      Accept all changes"),
        Line::from("  Ctrl+r      Reject with comments"),
        Line::from("  Enter       Submit comment (in comment mode)"),
        Line::from("  Esc         Cancel comment"),
        Line::from("  ?           Toggle this help"),
    ];

    let width = (area.width as usize).min(50);
    let height = (help_lines.len() + 4) as u16;
    let x = (area.width.saturating_sub(width as u16)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let help_area = Rect {
        x,
        y,
        width: width as u16,
        height,
    };

    // Clear area behind the help so no content shows through.
    frame.render_widget(Clear, help_area);

    // Solid background fill.
    let bg = Block::default().style(Style::default().bg(Color::Rgb(20, 20, 40)));
    frame.render_widget(bg, help_area);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::White));
    let para = Paragraph::new(Text::from(help_lines))
        .block(block)
        .style(Style::default().bg(Color::Rgb(20, 20, 40)))
        .alignment(Alignment::Left);
    frame.render_widget(para, help_area);
}

fn render_popup(frame: &mut Frame, area: Rect, message: &str) {
    let lines: Vec<Line> = message
        .lines()
        .map(|l| {
            if l.starts_with('✓') {
                Line::from(Span::styled(
                    l,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(l, Style::default().fg(Color::White)))
            }
        })
        .collect();

    let width = (area.width as usize).min(60);
    let height = (lines.len() + 4) as u16;
    let x = (area.width.saturating_sub(width as u16)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let popup_area = Rect {
        x,
        y,
        width: width as u16,
        height,
    };

    // Clear area behind the popup so no content shows through.
    frame.render_widget(Clear, popup_area);

    // Solid background fill.
    let bg = Block::default().style(Style::default().bg(Color::Rgb(10, 30, 10)));
    frame.render_widget(bg, popup_area);

    let block = Block::default()
        .title(" Comment ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Green));
    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .style(Style::default().bg(Color::Rgb(10, 30, 10)))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, popup_area);
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

/// Process a single keyboard event and update the app state.
/// Returns `true` if the app should continue, `false` to exit.
fn handle_input(app: &mut ReviewApp, key: KeyCode, modifiers: KeyModifiers) -> bool {
    // Dismiss popup on any key
    if app.show_popup {
        app.show_popup = false;
        return true;
    }

    if app.comment_mode {
        return handle_comment_input(app, key, modifiers);
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => {
            if modifiers == KeyModifiers::SHIFT {
                app.go_bottom();
            } else {
                app.go_top();
            }
        }
        KeyCode::Char('G') => app.go_bottom(),
        KeyCode::Char('c') => app.start_comment(),
        KeyCode::Char('v') => app.view_comment(),
        KeyCode::Char('n') => {
            if modifiers == KeyModifiers::SHIFT {
                app.prev_comment();
            } else {
                app.next_comment();
            }
        }
        KeyCode::Char('N') => app.prev_comment(),
        KeyCode::Char('a') if modifiers == KeyModifiers::CONTROL => app.accept(),
        KeyCode::Char('r') if modifiers == KeyModifiers::CONTROL => app.reject(),
        KeyCode::Char('?') => app.show_help = !app.show_help,
        _ => {}
    }

    // Return false when verdict is set (review complete).
    app.verdict.is_none()
}

fn handle_comment_input(app: &mut ReviewApp, key: KeyCode, modifiers: KeyModifiers) -> bool {
    match key {
        KeyCode::Enter => app.submit_comment(),
        KeyCode::Esc => app.cancel_comment(),
        KeyCode::Char(c) if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
            app.comment_char(c);
        }
        KeyCode::Backspace => app.comment_backspace(),
        KeyCode::Left if app.comment_cursor > 0 => {
            let prev = (0..app.comment_cursor)
                .rev()
                .find(|&i| app.comment_buffer.is_char_boundary(i))
                .unwrap_or(0);
            app.comment_cursor = prev;
        }
        KeyCode::Right if app.comment_cursor < app.comment_buffer.len() => {
            let next = (app.comment_cursor + 1..=app.comment_buffer.len())
                .find(|&i| app.comment_buffer.is_char_boundary(i))
                .unwrap_or(app.comment_cursor);
            app.comment_cursor = next;
        }
        KeyCode::Home => app.comment_cursor = 0,
        KeyCode::End => app.comment_cursor = app.comment_buffer.len(),
        _ => {}
    }
    true
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the CLI review for the given session.
///
/// Returns the user's [`ReviewVerdict`].
pub(crate) async fn run_cli_review(session: &mut ReviewSession) -> io::Result<ReviewVerdict> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let app_state = ReviewApp::new(session.clone());
    let result = run_app(&mut terminal, app_state).await;

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(Some(verdict)) => {
            session.verdict = Some(verdict.clone());
            Ok(verdict)
        }
        Ok(None) => {
            let verdict = ReviewVerdict::AcceptAll(session.comments.clone());
            session.verdict = Some(verdict.clone());
            Ok(verdict)
        }
        Err(e) => Err(e),
    }
}

/// Run the event loop.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: ReviewApp,
) -> io::Result<Option<ReviewVerdict>> {
    loop {
        terminal.draw(|frame| render_app(frame, &mut app))?;

        if app.verdict.is_some() {
            return Ok(app.verdict.take());
        }

        // Handle input (50ms timeout for responsiveness while keeping CPU low).
        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if !handle_input(&mut app, key.code, key.modifiers) => {
                    return Ok(app.verdict.take());
                }
                Event::Resize(cols, _rows) => {
                    app.term_width = cols;
                }
                _ => {}
            }
        }
    }
}
