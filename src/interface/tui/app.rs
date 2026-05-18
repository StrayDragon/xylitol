//! Root App component and event loop for the TUI.
//!
//! Manages the component tree, routes events, and drives the ratatui render loop.

use std::sync::Arc;

use futures::StreamExt;
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use tokio::sync::mpsc;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use adk_session::SessionService;

use crate::infra::config::AppConfig;

use super::approval::ApprovalOverlay;
use super::chat::ChatComponent;
use super::diff_preview::DiffPreviewComponent;
use super::help::HelpOverlay;
use super::input::InputComponent;
use super::selectors::{ListSelector, SelectorKind, session_selector};
use super::status_bar::StatusBar;
use super::tool_output::ToolOutputComponent;

/// Active focus area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Chat,
    Input,
    ToolPanel,
}

/// Application state — root component tree.
pub(crate) struct App {
    // Components
    chat: ChatComponent,
    tool_output: ToolOutputComponent,
    input: InputComponent,
    status_bar: StatusBar,
    diff_preview: DiffPreviewComponent,
    approval: ApprovalOverlay,
    help: HelpOverlay,

    // Selectors
    session_selector: ListSelector,
    model_selector: ListSelector,
    theme_selector: ListSelector,
    active_selector: Option<SelectorKind>,

    // State
    focus: FocusArea,
    running: bool,
    should_quit: bool,
    session_id: String,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            chat: ChatComponent::new(),
            tool_output: ToolOutputComponent::new(),
            input: InputComponent::new(),
            status_bar: StatusBar::new(),
            diff_preview: DiffPreviewComponent::new(),
            approval: ApprovalOverlay::new(),
            help: HelpOverlay::new(),
            session_selector: session_selector(),
            model_selector: session_selector(),
            theme_selector: session_selector(),
            active_selector: None,
            focus: FocusArea::Input,
            running: false,
            should_quit: false,
            session_id: "tui-session".into(),
        }
    }

    /// Handle a keyboard event.
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<AppAction> {
        // If an overlay is active, route keys there first.
        if self.help.is_visible() {
            match key.code {
                crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('?') => {
                    self.help.toggle();
                }
                _ => {}
            }
            return None;
        }

        if self.approval.is_active() {
            match key.code {
                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                    self.approval.select_prev();
                }
                crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                    self.approval.select_next();
                }
                crossterm::event::KeyCode::Enter => {
                    self.approval.confirm();
                }
                crossterm::event::KeyCode::Esc => {
                    self.approval.cancel();
                }
                _ => {}
            }
            return None;
        }

        if let Some(ref kind) = self.active_selector {
            match key.code {
                crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                    match kind {
                        SelectorKind::Session => self.session_selector.select_prev(),
                        SelectorKind::Model => self.model_selector.select_prev(),
                        SelectorKind::Theme => self.theme_selector.select_prev(),
                    }
                }
                crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                    match kind {
                        SelectorKind::Session => self.session_selector.select_next(),
                        SelectorKind::Model => self.model_selector.select_next(),
                        SelectorKind::Theme => self.theme_selector.select_next(),
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    let result = match kind {
                        SelectorKind::Session => self.session_selector.confirm(),
                        SelectorKind::Model => self.model_selector.confirm(),
                        SelectorKind::Theme => self.theme_selector.confirm(),
                    };
                    if let Some(selected) = result {
                        self.status_bar.set_message(format!("Selected: {selected}"));
                    }
                    self.active_selector = None;
                }
                crossterm::event::KeyCode::Esc => {
                    match kind {
                        SelectorKind::Session => self.session_selector.cancel(),
                        SelectorKind::Model => self.model_selector.cancel(),
                        SelectorKind::Theme => self.theme_selector.cancel(),
                    }
                    self.active_selector = None;
                }
                _ => {}
            }
            return None;
        }

        // Normal mode key handling.
        match key.code {
            crossterm::event::KeyCode::Char('?') => {
                self.help.toggle();
            }
            crossterm::event::KeyCode::Char('c')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL && self.running =>
            {
                return Some(AppAction::Interrupt);
            }
            crossterm::event::KeyCode::Char('d')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                self.should_quit = true;
            }
            crossterm::event::KeyCode::Char('l')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                return Some(AppAction::Clear);
            }
            crossterm::event::KeyCode::Char('r')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                self.diff_preview.toggle();
            }
            crossterm::event::KeyCode::Tab => {
                self.cycle_focus();
            }
            crossterm::event::KeyCode::Enter if !self.running => {
                let prompt = self.input.submit();
                if let Some(text) = prompt {
                    self.chat.add_user_message(&text);
                    self.running = true;
                    self.input.set_disabled(true);
                    self.status_bar.set_running(true);
                    self.status_bar
                        .set_message(format!("Running... (prompt: {text})"));
                    return Some(AppAction::RunPrompt(text));
                }
            }
            crossterm::event::KeyCode::Backspace => {
                self.input.delete_before();
            }
            crossterm::event::KeyCode::Delete => {
                self.input.delete_at();
            }
            crossterm::event::KeyCode::Left => {
                self.input.cursor_left();
            }
            crossterm::event::KeyCode::Right => {
                self.input.cursor_right();
            }
            crossterm::event::KeyCode::Up => {
                self.input.history_back();
            }
            crossterm::event::KeyCode::Down => {
                self.input.history_forward();
            }
            crossterm::event::KeyCode::Home => {
                self.input.cursor_home();
            }
            crossterm::event::KeyCode::End => {
                self.input.cursor_end();
            }
            crossterm::event::KeyCode::Char(c) => {
                self.input.insert_char(c);
            }
            crossterm::event::KeyCode::Esc => {
                // May also close help/overlays.
            }
            _ => {}
        }

        None
    }

    /// Handle an agent event.
    fn handle_agent_event(&mut self, event: AgentEvent) {
        match &event {
            AgentEvent::TextDelta(_) => {
                self.chat.handle_event(&event);
            }
            AgentEvent::ToolCallStart { id, name, .. } => {
                self.tool_output.handle_tool_start(id.clone(), name.clone());
                self.chat.handle_event(&event);
                self.status_bar.set_message(format!("Tool call: {name}..."));
            }
            AgentEvent::ToolCallEnd { id, result } => {
                self.tool_output.handle_tool_end(id.clone(), result);
                self.status_bar.set_message("Tool call completed.");
            }
            AgentEvent::StepComplete { summary, .. } => {
                self.chat.handle_event(&event);
                self.status_bar.set_message(format!("Step done: {summary}"));
            }
            AgentEvent::Error(err) => {
                self.status_bar.set_message(format!("Error: {err}"));
                self.chat.handle_event(&event);
            }
            AgentEvent::RepeatDetected { .. } => {
                self.status_bar
                    .set_message("Repeat detected — interrupting.");
            }
        }
    }

    /// Mark agent as completed.
    fn agent_completed(&mut self) {
        self.running = false;
        self.input.set_disabled(false);
        self.status_bar.set_running(false);
        self.status_bar.set_message("Ready.");
    }

    /// Cycle focus between Chat, Input, and ToolPanel.
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Chat => FocusArea::Input,
            FocusArea::Input => FocusArea::ToolPanel,
            FocusArea::ToolPanel => FocusArea::Chat,
        };
    }

    /// Render the entire TUI.
    fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // ── Layout ──────────────────────────────────────────
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(3),    // Chat + Tools
                Constraint::Length(4), // Tool output
                Constraint::Length(3), // Input
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        // ── Header ──────────────────────────────────────────
        let header_style = if self.running {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 50))
        };
        let mode_indicator = if self.running { " RUNNING " } else { " READY " };
        let focus_name = match self.focus {
            FocusArea::Chat => "Chat",
            FocusArea::Input => "Input",
            FocusArea::ToolPanel => "Tools",
        };
        let header_text = format!(
            " xylitol  |  {}  |  [{}]  |  ? for help",
            mode_indicator, focus_name
        );

        // Fill header background.
        let header_buf = frame.buffer_mut();
        for x in chunks[0].x..chunks[0].right() {
            if let Some(cell) = header_buf.cell_mut((x, chunks[0].y)) {
                cell.set_style(header_style);
                cell.set_symbol(" ");
            }
        }
        header_buf.set_string(chunks[0].x, chunks[0].y, &header_text, header_style);

        // ── Chat area ───────────────────────────────────────
        let chat_area = if self.tool_output_is_empty() {
            chunks[1]
        } else {
            // Split chat and tool output vertically.
            let chat_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(4), // tool output
                ])
                .split(chunks[1]);
            self.tool_output.render(frame, chat_chunks[1]);
            chat_chunks[0]
        };
        self.chat.render(frame, chat_area);

        // ── Input area ──────────────────────────────────────
        self.input.render(frame, chunks[3]);

        // ── Status bar ──────────────────────────────────────
        frame.render_widget(&self.status_bar, chunks[4]);

        // ── Overlays ────────────────────────────────────────
        self.diff_preview.render(frame, area);
        self.approval.render(frame, area);
        self.help.render(frame, area);

        if let Some(ref kind) = self.active_selector {
            match kind {
                SelectorKind::Session => self.session_selector.render(frame, area),
                SelectorKind::Model => self.model_selector.render(frame, area),
                SelectorKind::Theme => self.theme_selector.render(frame, area),
            }
        }
    }

    fn tool_output_is_empty(&self) -> bool {
        // Check if there are visible tool entries.
        // ToolOutputComponent doesn't expose this directly; approximate.
        false
    }
}

/// Actions that the event loop performs on behalf of the App.
enum AppAction {
    RunPrompt(String),
    Interrupt,
    Clear,
}

/// Run the TUI event loop. This is the main entry point called from the CLI.
pub(crate) async fn run_tui(
    tool_registry: crate::agent::tools::ToolRegistry,
    app_config: AppConfig,
    profile: crate::agent::profile::ResolvedProfile,
    session_service: Arc<dyn SessionService>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use ratatui::backend::CrosstermBackend;
    use std::io::stdout;

    // ── Terminal setup ──────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // ── Agent loop setup ────────────────────────────────────
    let agent_loop = Arc::new(
        AgentLoop::new(
            &tool_registry,
            profile,
            session_service.clone(),
            "xylitol".into(),
            Some(&app_config.hooks),
        )
        .await?,
    );

    // ── Channels ────────────────────────────────────────────
    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<crossterm::event::Event>();

    // Spawn crossterm keyboard reader on a blocking thread.
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = crossterm::event::read() {
            if key_tx.send(event).is_err() {
                break;
            }
        }
    });

    // ── App ─────────────────────────────────────────────────
    let mut app = App::new();
    let mut current_agent_handle: Option<tokio::task::JoinHandle<()>> = None;

    // ── Event loop ──────────────────────────────────────────
    let result: Result<(), Box<dyn std::error::Error>> = 'event_loop: loop {
        // Tick timer for frame rate control.
        let tick = tokio::time::sleep(std::time::Duration::from_millis(250));
        tokio::pin!(tick);

        tokio::select! {
            Some(agent_event) = agent_rx.recv() => {
                app.handle_agent_event(agent_event);
            }
            key_event = key_rx.recv() => {
                let Some(crossterm::event::Event::Key(key)) = key_event else {
                    // Keyboard reader thread exited — quit.
                    break 'event_loop Ok(());
                };
                if let Some(action) = app.handle_key(key) {
                        match action {
                            AppAction::RunPrompt(prompt) => {
                                // Start agent execution in background.
                                let agent_loop = agent_loop.clone();
                                let agent_tx = agent_tx.clone();
                                let session_id = app.session_id.clone();
                                let handle = tokio::spawn(async move {
                                    let result = agent_loop
                                        .run(&prompt, &session_id, None)
                                        .await;
                                    match result {
                                        Ok(mut stream) => {
                                            while let Some(event) = stream.next().await {
                                                if agent_tx.send(event).is_err() {
                                                    break;
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            let _ = agent_tx.send(AgentEvent::Error(err));
                                        }
                                    }
                                });
                                current_agent_handle = Some(handle);
                            }
                            AppAction::Interrupt => {
                                // Cancel current agent task.
                                if let Some(handle) = current_agent_handle.take() {
                                    handle.abort();
                                }
                                app.agent_completed();
                            }
                            AppAction::Clear => {
                                // Clear is not fully implemented yet.
                            }
                        }
                    }

                    if app.should_quit {
                        break 'event_loop Ok(());
                    }
            }
            _ = &mut tick => {
                // Tick — redraw.
            }
        }

        // Check if agent task has completed.
        if app.running
            && let Some(ref handle) = current_agent_handle
            && handle.is_finished()
        {
            app.agent_completed();
            current_agent_handle = None;
        }

        // Redraw on every iteration.
        terminal.draw(|f| app.render(f))?;
    };

    // ── Cleanup ─────────────────────────────────────────────
    if let Some(handle) = current_agent_handle {
        handle.abort();
    }
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
