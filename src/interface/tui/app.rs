//! Root app component and event loop for the TUI.

use std::sync::Arc;

use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use tokio::sync::mpsc;

use adk_session::SessionService;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use crate::agent::profile::ResolvedProfile;
use crate::agent::tools::ToolRegistry;
use crate::infra::config::AppConfig;

use super::chat::ChatComponent;
use super::component::Component;
use super::component::OverlayStack;
use super::event::{AppAction, TuiEvent};
use super::history::HistoryStore;
use super::input::InputComponent;
use super::markdown::MarkdownRenderer;
use super::overlays::HelpOverlay;
use super::slash::{Completer, SlashCommand};
use super::status_bar::StatusBar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Input,
    Chat,
}

pub(crate) struct App {
    chat: ChatComponent,
    input: InputComponent,
    status_bar: StatusBar,
    overlays: OverlayStack,

    running: bool,
    should_quit: bool,
    queued_prompts: Vec<String>,

    focus: Focus,
    session_id: String,

    history: HistoryStore,

    last_area: Rect,
    last_input_height: u16,
}

impl App {
    pub(crate) fn new(
        _tool_registry: ToolRegistry,
        _app_config: AppConfig,
        profile: ResolvedProfile,
        _session_service: Arc<dyn SessionService>,
    ) -> Self {
        let markdown = MarkdownRenderer::default();
        let chat = ChatComponent::new(markdown);
        let completer = Completer::new();
        let mut input = InputComponent::new(completer);
        let mut status_bar = StatusBar::new();

        status_bar.set_model(format!(
            "{}:{}",
            profile.model_config.provider_name(),
            profile.model_config.model
        ));
        status_bar.set_session("default");

        let history = HistoryStore::load(200).unwrap_or_else(|_| {
            let path = std::env::temp_dir().join("xylitol-history");
            HistoryStore::load_from(path, 200).unwrap()
        });
        input.set_history(history.iter().map(|s| s.to_string()).collect());

        Self {
            chat,
            input,
            status_bar,
            overlays: OverlayStack::new(),
            running: false,
            should_quit: false,
            queued_prompts: Vec::new(),
            focus: Focus::Input,
            session_id: "tui-session".into(),
            history,
            last_area: Rect::new(0, 0, 0, 0),
            last_input_height: 0,
        }
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn set_running(&mut self, running: bool) {
        self.running = running;
        self.status_bar.set_running(running);
    }

    pub(crate) fn clear(&mut self) {
        self.chat.clear();
        self.queued_prompts.clear();
        self.status_bar.set_queue_len(0);
        self.status_bar.set_message("Cleared.");
    }

    pub(crate) fn update(&mut self, event: TuiEvent) -> Option<AppAction> {
        // Overlays always get first right of refusal for key events.
        if !self.overlays.is_empty()
            && let Some(result) = self.overlays.route_event(&event)
        {
            if let Some(action) = result.action {
                return Some(action);
            }
            return None;
        }

        match event {
            TuiEvent::Agent(agent_event) => {
                // Update running flag on terminal events.
                match agent_event {
                    AgentEvent::StepComplete { .. }
                    | AgentEvent::Error(_)
                    | AgentEvent::RepeatDetected { .. } => {
                        self.set_running(false);

                        // Auto-submit queued prompt after agent completion.
                        if let Some(next) = self.queued_prompts.first().cloned() {
                            self.queued_prompts.remove(0);
                            self.status_bar.set_queue_len(self.queued_prompts.len());
                            return Some(AppAction::RunPrompt(next));
                        }
                    }
                    _ => {}
                }

                self.chat.handle_event(&TuiEvent::Agent(agent_event));
                None
            }
            TuiEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyModifiers};

                // Global shortcuts.
                match key.code {
                    KeyCode::Char('?') => {
                        self.overlays.push(Box::new(HelpOverlay::new()));
                        return None;
                    }
                    KeyCode::Tab => {
                        self.focus = match self.focus {
                            Focus::Input => Focus::Chat,
                            Focus::Chat => Focus::Input,
                        };
                        return None;
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                        return None;
                    }
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) && self.running =>
                    {
                        return Some(AppAction::Interrupt);
                    }
                    KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Some(AppAction::Clear);
                    }
                    _ => {}
                }

                // Focused component routing.
                match self.focus {
                    Focus::Input => {
                        let result = self.input.handle_event(&TuiEvent::Key(key));
                        if let Some(AppAction::RunPrompt(text)) = result.action {
                            // Slash commands run locally.
                            if let Some(cmd) = SlashCommand::parse(&text) {
                                match cmd {
                                    SlashCommand::Clear => return Some(AppAction::Clear),
                                    SlashCommand::Help => {
                                        self.overlays.push(Box::new(HelpOverlay::new()));
                                        return None;
                                    }
                                    SlashCommand::Quit => {
                                        self.should_quit = true;
                                        return None;
                                    }
                                }
                            }

                            // Persist history.
                            let _ = self.history.add(&text);
                            self.input
                                .set_history(self.history.iter().map(|s| s.to_string()).collect());

                            // Add to chat now (optimistic).
                            self.chat.add_user_message(&text);

                            if self.running {
                                self.queued_prompts.push(text.clone());
                                self.status_bar.set_queue_len(self.queued_prompts.len());
                                return Some(AppAction::QueuePrompt(text));
                            }

                            self.set_running(true);
                            return Some(AppAction::RunPrompt(text));
                        }
                        None
                    }
                    Focus::Chat => {
                        let _ = self.chat.handle_event(&TuiEvent::Key(key));
                        None
                    }
                }
            }
            TuiEvent::Tick => {
                // Currently a no-op; components that animate should mark themselves dirty.
                None
            }
            TuiEvent::Shutdown => {
                self.should_quit = true;
                None
            }
        }
    }

    pub(crate) fn render(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let input_h = self.input.desired_height().saturating_add(2);

        let size_changed = self.last_area != area;
        let input_changed = self.last_input_height != input_h;
        if size_changed || input_changed {
            self.last_area = area;
            self.last_input_height = input_h;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(input_h),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        if size_changed || input_changed || self.chat.is_dirty() {
            self.chat.render(frame, chunks[0]);
        }
        if size_changed || input_changed || self.input.is_dirty() {
            self.input.render(frame, chunks[1]);
        }
        if size_changed || input_changed || self.status_bar.is_dirty() {
            self.status_bar.render(frame, chunks[2]);
        }

        if !self.overlays.is_empty() {
            self.overlays.render_all(frame, area);
        }
    }
}

/// Run the TUI event loop. This is the main entry point called from the CLI.
pub(crate) async fn run_tui(
    tool_registry: ToolRegistry,
    app_config: AppConfig,
    profile: ResolvedProfile,
    session_service: Arc<dyn SessionService>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::execute;
    use crossterm::terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    };
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
            profile.clone(),
            session_service.clone(),
            "xylitol".into(),
            Some(&app_config.hooks),
        )
        .await?,
    );

    // ── Channels ────────────────────────────────────────────
    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let (key_tx, mut key_rx) = mpsc::unbounded_channel::<crossterm::event::KeyEvent>();

    // Spawn crossterm keyboard reader on a blocking thread.
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = crossterm::event::read() {
            if let crossterm::event::Event::Key(key) = event
                && key_tx.send(key).is_err()
            {
                break;
            }
        }
    });

    // ── App ─────────────────────────────────────────────────
    let mut app = App::new(
        tool_registry.clone(),
        app_config.clone(),
        profile.clone(),
        session_service.clone(),
    );
    let mut current_agent_handle: Option<tokio::task::JoinHandle<()>> = None;

    let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));

    // ── Event loop ──────────────────────────────────────────
    let result: Result<(), Box<dyn std::error::Error>> = 'event_loop: loop {
        tokio::select! {
            Some(agent_event) = agent_rx.recv() => {
                if let Some(action) = app.update(TuiEvent::Agent(agent_event)) {
                    handle_action(&mut app, action, &agent_loop, &agent_tx, &mut current_agent_handle);
                }
            }
            Some(key) = key_rx.recv() => {
                if let Some(action) = app.update(TuiEvent::Key(key)) {
                    handle_action(&mut app, action, &agent_loop, &agent_tx, &mut current_agent_handle);
                }
            }
            _ = tick.tick() => {
                let _ = app.update(TuiEvent::Tick);
            }
        }

        if app.should_quit() {
            break 'event_loop Ok(());
        }

        // Redraw.
        execute!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
        terminal.draw(|f| app.render(f))?;
        execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
    };

    // ── Cleanup ─────────────────────────────────────────────
    if let Some(handle) = current_agent_handle.take() {
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

fn handle_action(
    app: &mut App,
    action: AppAction,
    agent_loop: &Arc<AgentLoop>,
    agent_tx: &mpsc::UnboundedSender<AgentEvent>,
    current_agent_handle: &mut Option<tokio::task::JoinHandle<()>>,
) {
    match action {
        AppAction::RunPrompt(prompt) => {
            app.set_running(true);
            // Start agent execution in background.
            let agent_loop = agent_loop.clone();
            let agent_tx = agent_tx.clone();
            let session_id = app.session_id().to_string();
            let handle = tokio::spawn(async move {
                let result = agent_loop.run(&prompt, &session_id, None).await;
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
            *current_agent_handle = Some(handle);
        }
        AppAction::Interrupt => {
            if let Some(handle) = current_agent_handle.take() {
                handle.abort();
            }
            app.set_running(false);
        }
        AppAction::Clear => {
            app.clear();
        }
        AppAction::SetDiff(_diff) => {}
        AppAction::QueuePrompt(_prompt) => {}
    }
}
