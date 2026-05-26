//! Root app component and event loop for the TUI.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use crate::agent::profile::ResolvedProfile;
use crate::agent::session::XySession;
use crate::agent::tools::ToolRegistry;
use crate::infra::config::AppConfig;
use crate::infra::security::SecurityEngine;

use super::approval::{ApprovalHub, SecureApprovalToolWrapper, requires_approval};
use super::bottom_pane::footer::{FooterMode, FooterState};
use super::chat::ChatComponent;
use super::component::Component;
use super::component::OverlayStack;
use super::event::{AppAction, TuiEvent};
use super::history::HistoryStore;
use super::input::InputComponent;
use super::keyboard_modes;
use super::keymap::{AppKeyAction, ComposerKeyAction, RuntimeKeymap};
use super::markdown::MarkdownRenderer;
use super::overlays::{
    ApprovalOverlay, HelpOverlay, HistorySearchOverlay, SelectorKind, SelectorOverlay,
    TranscriptOverlay,
};
use super::slash::{Completer, SlashCommand};
use super::status_bar::StatusBar;
use super::tool_panel::ToolPanelComponent;

#[cfg(feature = "ui-review")]
use super::overlays::DiffPreviewOverlay;

#[cfg(feature = "ui-review")]
use crate::interface::diff_review::types::DiffHunk;
#[cfg(feature = "ui-review")]
use crate::interface::diff_review::{ReviewBackend, ReviewEngine, ReviewEngineConfig, ReviewMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Input,
    Chat,
}

#[derive(Clone)]
struct InputThreadControl {
    paused: Arc<AtomicBool>,
    drain_requested: Arc<AtomicBool>,
}

impl InputThreadControl {
    fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            drain_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    fn request_drain(&self) {
        self.drain_requested.store(true, Ordering::Relaxed);
    }
}

pub(crate) struct App {
    chat: ChatComponent,
    tool_panel: ToolPanelComponent,
    input: InputComponent,
    status_bar: StatusBar,
    footer: FooterState,
    overlays: OverlayStack,

    keymap: RuntimeKeymap,

    app_config: AppConfig,
    session_service: Arc<dyn XySession>,
    active_profile: String,
    approvals: Arc<ApprovalHub>,

    running: bool,
    should_quit: bool,
    queued_prompts: Vec<String>,

    backtrack_primed: bool,

    raw_output: bool,

    focus: Focus,
    session_id: String,

    history: HistoryStore,

    profile_choices: Vec<String>,
    session_choices: Vec<String>,
    theme_choices: Vec<String>,

    last_area: Rect,
    chat_area: Rect,
    tool_area: Rect,
    input_area: Rect,
    status_area: Rect,
    last_input_height: u16,
    last_tool_panel_height: u16,

    tool_panel_height: u16,

    #[cfg(feature = "ui-review")]
    tool_calls: HashMap<String, (String, serde_json::Value)>,
    #[cfg(feature = "ui-review")]
    file_changes: HashMap<String, (String, Option<String>)>,
    #[cfg(feature = "ui-review")]
    diff_hunks: Vec<DiffHunk>,
}

impl App {
    pub(crate) async fn new(
        _tool_registry: ToolRegistry,
        app_config: AppConfig,
        profile: ResolvedProfile,
        session_service: Arc<dyn XySession>,
        approvals: Arc<ApprovalHub>,
    ) -> Self {
        let markdown = MarkdownRenderer::default();
        let chat = ChatComponent::new(markdown);
        let completer = Completer::new();
        let mut input = InputComponent::new(completer);
        let mut status_bar = StatusBar::new();
        let mut footer = FooterState::new();

        let keymap = RuntimeKeymap::built_in_defaults();

        status_bar.set_model(format!(
            "{}:{}",
            profile.model_config.provider_name(),
            profile.model_config.model
        ));
        status_bar.set_session("default");
        footer.model_label = format!(
            "{}:{}",
            profile.model_config.provider_name(),
            profile.model_config.model
        );
        footer.session_label = "default".to_string();

        let history = match HistoryStore::load(200) {
            Ok(store) => store,
            Err(err) => {
                tracing::warn!(error = %err, "Failed to load history store; using temp file.");
                let path = std::env::temp_dir().join("xylitol-history");
                HistoryStore::load_from(path.clone(), 200).unwrap_or_else(|err| {
                    tracing::warn!(error = %err, "Failed to load temp history store; disabling.");
                    HistoryStore::empty(path, 200)
                })
            }
        };
        input.set_history(history.iter().map(|s| s.to_string()).collect());

        let mut profile_choices: Vec<String> = app_config.agents.profiles.keys().cloned().collect();
        profile_choices.sort();
        if profile_choices.is_empty() && !app_config.agents.default_profile.is_empty() {
            profile_choices.push(app_config.agents.default_profile.clone());
        }

        let mut session_choices: Vec<String> = match session_service.list_session_ids().await {
            Ok(ids) => ids,
            Err(err) => {
                tracing::warn!(error = %err, "Failed to list sessions.");
                Vec::new()
            }
        };
        session_choices.sort();

        let mut theme_choices: Vec<String> = super::markdown::MarkdownRenderer::available_themes()
            .into_iter()
            .map(String::from)
            .collect();
        theme_choices.sort();

        Self {
            chat,
            tool_panel: ToolPanelComponent::new(),
            input,
            status_bar,
            footer,
            overlays: OverlayStack::new(),

            keymap,
            app_config,
            session_service,
            active_profile: profile.name.clone(),
            approvals,
            running: false,
            should_quit: false,
            queued_prompts: Vec::new(),

            backtrack_primed: false,

            raw_output: false,
            focus: Focus::Input,
            session_id: "tui-session".into(),
            history,
            profile_choices,
            session_choices,
            theme_choices,
            last_area: Rect::new(0, 0, 0, 0),
            chat_area: Rect::new(0, 0, 0, 0),
            tool_area: Rect::new(0, 0, 0, 0),
            input_area: Rect::new(0, 0, 0, 0),
            status_area: Rect::new(0, 0, 0, 0),
            last_input_height: 0,
            last_tool_panel_height: 0,

            tool_panel_height: 8,

            #[cfg(feature = "ui-review")]
            tool_calls: HashMap::new(),
            #[cfg(feature = "ui-review")]
            file_changes: HashMap::new(),
            #[cfg(feature = "ui-review")]
            diff_hunks: Vec::new(),
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
        self.footer.is_task_running = running;
    }

    pub(crate) fn clear(&mut self) {
        self.chat.clear();
        self.tool_panel.clear();
        self.queued_prompts.clear();
        self.status_bar.set_queue_len(0);
        self.footer.queue_len = 0;
        self.footer
            .set_message("Cleared.", std::time::Duration::from_secs(2));
    }

    fn last_user_message(&self) -> Option<String> {
        self.chat.last_user_message()
    }

    fn apply_profile_selection(&mut self, name: String) {
        self.active_profile = name.clone();

        match self.app_config.resolve_profile(&name) {
            Ok(profile) => {
                self.status_bar.set_model(format!(
                    "{}:{}",
                    profile.model_config.provider_name(),
                    profile.model_config.model
                ));
                self.footer.model_label = format!(
                    "{}:{}",
                    profile.model_config.provider_name(),
                    profile.model_config.model
                );
                self.footer.set_message(
                    format!("Profile: {name}"),
                    std::time::Duration::from_secs(2),
                );
            }
            Err(err) => {
                self.status_bar.set_model(name.clone());
                self.footer.model_label = name.clone();
                self.footer.set_message(
                    format!("Profile '{name}' not ready: {err}"),
                    std::time::Duration::from_secs(4),
                );
            }
        }
    }

    fn apply_session_selection(&mut self, session_id: String) {
        self.session_id = session_id.clone();
        self.status_bar.set_session(session_id);
        self.footer.session_label = self.session_id.clone();
        self.footer
            .set_message("Session switched.", std::time::Duration::from_secs(2));
    }

    fn apply_theme_selection(&mut self, theme: String) {
        if self.chat.set_theme(&theme) {
            self.footer
                .set_message(format!("Theme: {theme}"), std::time::Duration::from_secs(2));
        } else {
            self.footer.set_message(
                format!("Theme not found: {theme}"),
                std::time::Duration::from_secs(4),
            );
        }
    }

    fn handle_app_key_action(&mut self, action: AppKeyAction) -> Option<AppAction> {
        match action {
            AppKeyAction::ToggleShortcutOverlay => {
                if self.running || !self.input.is_empty() || !self.overlays.is_empty() {
                    return None;
                }

                self.footer.toggle_shortcuts_overlay();
                None
            }
            AppKeyAction::Quit => {
                self.should_quit = true;
                None
            }
            AppKeyAction::Interrupt => {
                if self.running {
                    Some(AppAction::Interrupt)
                } else {
                    None
                }
            }
            AppKeyAction::Clear => Some(AppAction::Clear),
            AppKeyAction::PreviewDiff => {
                #[cfg(feature = "ui-review")]
                if !self.diff_hunks.is_empty() {
                    self.overlays
                        .push(Box::new(DiffPreviewOverlay::new(self.diff_hunks.clone())));
                } else {
                    self.footer
                        .set_message("No diffs to preview.", std::time::Duration::from_secs(2));
                }

                None
            }

            AppKeyAction::OpenTranscript => {
                let transcript = self.chat.transcript_as_markdown();
                self.overlays
                    .push(Box::new(TranscriptOverlay::new(transcript)));
                None
            }
            AppKeyAction::CopyLastResponse => {
                use std::time::Duration;

                if let Some(text) = self.chat.last_assistant_message() {
                    match copy_to_clipboard(&text) {
                        Ok(method) => {
                            self.footer.set_message(
                                format!("Copied last response ({})", method.label()),
                                Duration::from_secs(2),
                            );
                        }
                        Err(err) => {
                            self.footer.set_message(err, Duration::from_secs(4));
                        }
                    }
                } else {
                    self.footer
                        .set_message("No assistant response yet.", Duration::from_secs(2));
                }
                None
            }
            AppKeyAction::ToggleRawOutput => {
                use std::time::Duration;

                self.raw_output = !self.raw_output;
                self.chat.set_raw_output(self.raw_output);
                self.input.set_raw_output(self.raw_output);
                let msg = if self.raw_output {
                    "Raw output: ON"
                } else {
                    "Raw output: OFF"
                };
                self.footer.set_message(msg, Duration::from_secs(2));
                None
            }
            AppKeyAction::ViewThinking => {
                use std::time::Duration;

                let Some((thinking, pending)) = self.chat.thinking_snapshot() else {
                    self.footer
                        .set_message("No thinking to show.", Duration::from_secs(2));
                    return None;
                };

                let title = if pending {
                    "## Thinking (live)\n\n"
                } else {
                    "## Thinking\n\n"
                };
                let mut md = String::new();
                md.push_str(title);
                md.push_str(&thinking);
                self.overlays.push(Box::new(TranscriptOverlay::new(md)));
                None
            }
        }
    }

    fn handle_composer_key_action(&mut self, action: ComposerKeyAction) -> Option<AppAction> {
        match action {
            ComposerKeyAction::OpenExternalEditor => Some(AppAction::OpenEditor(self.input.text())),
            ComposerKeyAction::ShowHistorySearch => Some(AppAction::ShowHistorySearch),
        }
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
                let mut next_action = None;

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
                            self.footer.queue_len = self.queued_prompts.len();
                            next_action = Some(AppAction::RunPrompt(next));
                        }
                    }
                    _ => {}
                }

                // Tool approval modal.
                if let AgentEvent::ToolCallStart { id, name, args } = &agent_event
                    && requires_approval(self.app_config.security.enabled, name)
                {
                    #[cfg(feature = "ui-review")]
                    self.capture_pre_tool_file_state(id, name, args);
                    let tx = self.approvals.register(id.clone());
                    #[cfg(feature = "ui-review")]
                    let diff_hunks = self.compute_approval_diff_hunks(name, args);
                    self.overlays.push(Box::new(ApprovalOverlay::prompt(
                        id.clone(),
                        name.clone(),
                        args.clone(),
                        #[cfg(feature = "ui-review")]
                        diff_hunks,
                        tx,
                    )));
                    self.footer
                        .set_sticky_message(format!("Approval required: {name}"));
                }

                #[cfg(feature = "ui-review")]
                {
                    match &agent_event {
                        AgentEvent::ToolCallStart { id, name, args } => {
                            self.tool_calls
                                .insert(id.clone(), (name.clone(), args.clone()));
                            self.capture_pre_tool_file_state(id, name, args);
                        }
                        AgentEvent::ToolCallEnd { id, .. } => {
                            self.capture_post_tool_file_state(id);
                        }
                        AgentEvent::StepComplete { .. } => {
                            self.collect_step_diffs();
                        }
                        _ => {}
                    }
                }

                let tool_event = agent_event.clone();
                self.chat.handle_event(&TuiEvent::Agent(agent_event));
                self.tool_panel.handle_event(&TuiEvent::Agent(tool_event));
                next_action
            }
            TuiEvent::Key(key) => {
                use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    return None;
                }

                // Focus switch (Input <-> Chat).
                if matches!(key.code, KeyCode::BackTab)
                    || (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
                {
                    self.focus = match self.focus {
                        Focus::Input => Focus::Chat,
                        Focus::Chat => Focus::Input,
                    };
                    return None;
                }

                // Global chat scrolling keys (work even when the composer is focused).
                if key.modifiers.is_empty() {
                    match key.code {
                        KeyCode::PageUp => {
                            self.chat.scroll_up(10);
                            return None;
                        }
                        KeyCode::PageDown => {
                            self.chat.scroll_down(10);
                            return None;
                        }
                        _ => {}
                    }
                }

                if let Some(app_action) = self.keymap.resolve_app(&key) {
                    if let Some(out) = self.handle_app_key_action(app_action) {
                        return Some(out);
                    }
                    return None;
                }

                if self.focus == Focus::Input
                    && let Some(composer_action) = self.keymap.resolve_composer(&key)
                {
                    if let Some(out) = self.handle_composer_key_action(composer_action) {
                        return Some(out);
                    }
                    return None;
                }

                // Focused component routing.
                match self.focus {
                    Focus::Input => {
                        let result = self.input.handle_event(&TuiEvent::Key(key));
                        match result.action {
                            Some(AppAction::OpenTranscript) => {
                                let transcript = self.chat.transcript_as_markdown();
                                self.overlays
                                    .push(Box::new(TranscriptOverlay::new(transcript)));
                                None
                            }
                            Some(AppAction::CopyLastResponse) => {
                                use std::time::Duration;

                                if let Some(text) = self.chat.last_assistant_message() {
                                    match copy_to_clipboard(&text) {
                                        Ok(method) => {
                                            self.footer.set_message(
                                                format!(
                                                    "Copied last response ({})",
                                                    method.label()
                                                ),
                                                Duration::from_secs(2),
                                            );
                                        }
                                        Err(err) => {
                                            self.footer.set_message(err, Duration::from_secs(4));
                                        }
                                    }
                                } else {
                                    self.footer.set_message(
                                        "No assistant response yet.",
                                        Duration::from_secs(2),
                                    );
                                }
                                None
                            }
                            Some(AppAction::ToggleRawOutput) => {
                                use std::time::Duration;

                                self.raw_output = !self.raw_output;
                                self.chat.set_raw_output(self.raw_output);
                                self.input.set_raw_output(self.raw_output);
                                let msg = if self.raw_output {
                                    "Raw output: ON"
                                } else {
                                    "Raw output: OFF"
                                };
                                self.footer.set_message(msg, Duration::from_secs(2));
                                None
                            }
                            Some(AppAction::RunPrompt(text)) => {
                                self.backtrack_primed = false;
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
                                        SlashCommand::Model => {
                                            let items = self.profile_choices.clone();
                                            self.overlays.push(Box::new(SelectorOverlay::new(
                                                SelectorKind::Profile,
                                                "Model",
                                                items,
                                            )));
                                            return None;
                                        }
                                        SlashCommand::Session => {
                                            let items = self.session_choices.clone();
                                            self.overlays.push(Box::new(SelectorOverlay::new(
                                                SelectorKind::Session,
                                                "Session",
                                                items,
                                            )));
                                            return None;
                                        }
                                        SlashCommand::Theme => {
                                            let items = self.theme_choices.clone();
                                            self.overlays.push(Box::new(SelectorOverlay::new(
                                                SelectorKind::Theme,
                                                "Theme",
                                                items,
                                            )));
                                            return None;
                                        }
                                    }
                                }

                                // Persist history.
                                let _ = self.history.add(&text);
                                self.input.set_history(
                                    self.history.iter().map(|s| s.to_string()).collect(),
                                );

                                // Add to chat now (optimistic).
                                self.chat.add_user_message(&text);

                                if self.running {
                                    self.queued_prompts.push(text.clone());
                                    self.status_bar.set_queue_len(self.queued_prompts.len());
                                    self.footer.queue_len = self.queued_prompts.len();
                                    return Some(AppAction::QueuePrompt(text));
                                }

                                self.set_running(true);
                                Some(AppAction::RunPrompt(text))
                            }
                            Some(AppAction::QueueOrSubmit(text)) => {
                                self.backtrack_primed = false;
                                // Persist history.
                                let _ = self.history.add(&text);
                                self.input.set_history(
                                    self.history.iter().map(|s| s.to_string()).collect(),
                                );

                                // Add to chat now (optimistic).
                                self.chat.add_user_message(&text);

                                if self.running {
                                    self.queued_prompts.push(text.clone());
                                    self.status_bar.set_queue_len(self.queued_prompts.len());
                                    self.footer.queue_len = self.queued_prompts.len();
                                    return Some(AppAction::QueuePrompt(text));
                                }

                                self.set_running(true);
                                Some(AppAction::RunPrompt(text))
                            }
                            Some(AppAction::BacktrackPrime) => {
                                if self.running {
                                    return None;
                                }
                                if self.backtrack_primed {
                                    return Some(AppAction::BacktrackEditLast);
                                }

                                self.backtrack_primed = true;
                                self.footer.set_message(
                                    "Backtrack: press Esc again to edit last message.",
                                    std::time::Duration::from_secs(2),
                                );
                                None
                            }
                            Some(AppAction::BacktrackEditLast) => {
                                if self.running {
                                    return None;
                                }
                                self.backtrack_primed = false;
                                if let Some(last) = self.last_user_message() {
                                    self.input.load_text(&last);
                                    self.footer.set_message(
                                        "Editing last message.",
                                        std::time::Duration::from_secs(2),
                                    );
                                } else {
                                    self.footer.set_message(
                                        "No previous user message.",
                                        std::time::Duration::from_secs(2),
                                    );
                                }
                                None
                            }
                            Some(other) => Some(other),
                            None => None,
                        }
                    }
                    Focus::Chat => {
                        let _ = self.chat.handle_event(&TuiEvent::Key(key));
                        None
                    }
                }
            }
            TuiEvent::Paste(text) => {
                if self.focus == Focus::Input {
                    let _ = self.input.handle_event(&TuiEvent::Paste(text));
                }
                None
            }
            TuiEvent::Mouse(mouse) => {
                use crossterm::event::MouseEventKind;
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        self.chat.scroll_up(3);
                    }
                    MouseEventKind::ScrollDown => {
                        self.chat.scroll_down(3);
                    }
                    _ => {}
                }
                None
            }
            TuiEvent::Tick => {
                self.footer.tick();
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
        let footer_h = 1u16;
        let input_h = self
            .input
            .desired_height()
            .min(area.height.saturating_sub(footer_h).max(1));

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
                    Constraint::Min(0),
                    Constraint::Length(input_h),
                    Constraint::Length(footer_h),
                ]
                .as_ref(),
            )
            .split(area);

        self.chat_area = chunks[0];
        self.tool_area = Rect::new(0, 0, 0, 0);
        self.input_area = chunks[1];
        self.status_area = chunks[2];

        self.chat.set_focused(false);
        self.input.set_focused(self.focus == Focus::Input);

        // Ratatui uses immediate-mode rendering: every `Terminal::draw` starts from an empty buffer.
        // We must render all visible components every frame; "dirty" flags should only be used to
        // decide whether a draw is needed at all, not to skip rendering within a draw.
        self.chat.render_live_preview(frame, self.chat_area);
        self.input.render(frame, self.input_area);

        // Anchor the terminal cursor inside the composer so IME/preedit placement stays stable
        // while the model is streaming output.
        if self.overlays.is_empty() && self.focus == Focus::Input {
            let (row, col) = self.input.cursor_display_col();
            let max_row = self.input_area.height.saturating_sub(1) as usize;
            let max_col = self.input_area.width.saturating_sub(1) as usize;
            let row = row.min(max_row) as u16;
            let col = col.min(max_col) as u16;
            frame.set_cursor_position((self.input_area.x + col, self.input_area.y + row));
        }

        // Footer mode: reflect current composer state and backtrack priming.
        // Preserve overlay state so `?` toggles persist until dismissed.
        if self.footer.mode != FooterMode::ShortcutOverlay {
            self.footer.mode = if self.backtrack_primed {
                FooterMode::EscHint
            } else if self.input.is_empty() {
                FooterMode::ComposerEmpty
            } else {
                FooterMode::ComposerHasDraft
            };
        }
        self.footer.render(self.status_area, frame.buffer_mut());

        if !self.overlays.is_empty() {
            self.overlays.render_all(frame, area);
        }
    }

    #[cfg(feature = "ui-review")]
    fn capture_pre_tool_file_state(
        &mut self,
        _call_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) {
        if !matches!(tool_name, "write" | "edit") {
            return;
        }

        let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) else {
            return;
        };

        if self.file_changes.contains_key(file_path) {
            return;
        }

        let old = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(err) => {
                tracing::debug!(path = file_path, error = %err, "Failed to read file before tool call.");
                String::new()
            }
        };

        self.file_changes.insert(file_path.to_string(), (old, None));
    }

    #[cfg(feature = "ui-review")]
    fn capture_post_tool_file_state(&mut self, call_id: &str) {
        let Some((tool_name, args)) = self.tool_calls.remove(call_id) else {
            return;
        };
        if !matches!(tool_name.as_str(), "write" | "edit") {
            return;
        }

        let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) else {
            return;
        };

        let new = match std::fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(err) => {
                tracing::debug!(path = file_path, error = %err, "Failed to read file after tool call.");
                String::new()
            }
        };

        self.file_changes
            .entry(file_path.to_string())
            .and_modify(|entry| entry.1 = Some(new.clone()))
            .or_insert_with(|| (String::new(), Some(new)));
    }

    #[cfg(feature = "ui-review")]
    fn collect_step_diffs(&mut self) {
        if self.file_changes.is_empty() {
            self.diff_hunks.clear();
            return;
        }

        let mut files = Vec::new();
        for (path, (old, new)) in std::mem::take(&mut self.file_changes) {
            let new_text =
                new.unwrap_or_else(|| std::fs::read_to_string(&path).unwrap_or_default());
            files.push((path, old, new_text));
        }

        let engine = ReviewEngine::new(ReviewEngineConfig {
            backend: ReviewBackend::Cli,
            mode: ReviewMode::OnStep,
        });
        let session = engine.create_session(&files);
        self.diff_hunks = session.hunks;
    }

    #[cfg(feature = "ui-review")]
    fn compute_approval_diff_hunks(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Vec<DiffHunk> {
        let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) else {
            return Vec::new();
        };

        let old_text = std::fs::read_to_string(file_path).unwrap_or_default();

        let new_text = match tool_name {
            "write" => args
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "edit" => {
                let old_string = args
                    .get("old_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_string = args
                    .get("new_string")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if old_string.is_empty() {
                    None
                } else if old_text.contains(old_string) {
                    Some(old_text.replace(old_string, new_string))
                } else {
                    crate::agent::tools::patch::fudiff_replace(&old_text, old_string, new_string)
                        .or_else(|| {
                            crate::agent::tools::patch::patch_fallback(
                                &old_text, old_string, new_string,
                            )
                        })
                }
            }
            _ => None,
        };

        let Some(new_text) = new_text else {
            return Vec::new();
        };
        if new_text == old_text {
            return Vec::new();
        }

        let engine = ReviewEngine::new(ReviewEngineConfig {
            backend: ReviewBackend::Cli,
            mode: ReviewMode::OnStep,
        });
        let session = engine.create_session(&[(file_path.to_string(), old_text, new_text)]);
        session.hunks
    }
}

/// Run the TUI event loop. This is the main entry point called from the CLI.
pub(crate) async fn run_tui(
    mut tool_registry: ToolRegistry,
    app_config: AppConfig,
    profile: ResolvedProfile,
    session_service: Arc<dyn XySession>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::cursor::{SetCursorStyle, Show};
    use crossterm::event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    };
    use crossterm::execute;
    use crossterm::terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, disable_raw_mode, enable_raw_mode,
    };
    use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    use std::io::stdout;

    // ── Terminal setup ──────────────────────────────────────
    let mut stdout = stdout();
    execute!(stdout, EnableBracketedPaste)?;
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;
    keyboard_modes::enable_keyboard_enhancement();
    let _ = execute!(stdout, super::terminal_modes::DisableAlternateScroll);
    let _ = execute!(stdout, EnableFocusChange);
    let _ = execute!(stdout, EnableMouseCapture);

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Detect whether progressive keyboard enhancement is actually supported so we can present the
    // right newline hint (Shift+Enter vs Ctrl+J). Keep it bounded to avoid slow startup.
    flush_terminal_input_buffer();
    let enhanced_keys_supported = if keyboard_modes::keyboard_enhancement_disabled() {
        false
    } else {
        super::terminal_probe::keyboard_enhancement_supported(std::time::Duration::from_millis(100))
    };
    flush_terminal_input_buffer();

    // ── Tool policy wiring ──────────────────────────────────
    let approvals = Arc::new(ApprovalHub::new());
    if app_config.security.enabled {
        let engine = SecurityEngine::new(&app_config.security);
        let approval_tools = std::sync::Arc::new(
            ["write", "edit"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<std::collections::HashSet<_>>(),
        );

        tool_registry.map_tools(|tool| {
            Arc::new(SecureApprovalToolWrapper::new(
                tool,
                engine.clone(),
                approvals.clone(),
                approval_tools.clone(),
                app_config.security.enabled,
            )) as Arc<dyn crate::agent::traits::XyTool>
        });
    }

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
    let (evt_tx, mut evt_rx) = mpsc::unbounded_channel::<crossterm::event::Event>();
    let input_ctrl = InputThreadControl::new();
    let input_shutdown = Arc::new(AtomicBool::new(false));

    // Spawn crossterm keyboard reader on a dedicated OS thread.
    //
    // NOTE: avoid `tokio::task::spawn_blocking` for this long-lived loop. Tokio waits for all
    // blocking tasks to finish when shutting down the runtime, which can cause quit to hang.
    let input_thread = std::thread::Builder::new()
        .name("xylitol-tui-input".into())
        .spawn({
            let input_paused = input_ctrl.paused.clone();
            let input_drain_requested = input_ctrl.drain_requested.clone();
            let input_shutdown = input_shutdown.clone();
            move || {
                use std::time::Duration;
                loop {
                    if input_shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    if input_paused.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(25));
                        continue;
                    }

                    if input_drain_requested.swap(false, Ordering::Relaxed) {
                        // Best-effort: drain any buffered events so stray terminal replies
                        // (e.g. OSC queries from external editors) don't get interpreted as input.
                        for _ in 0..256 {
                            match crossterm::event::poll(Duration::from_millis(0)) {
                                Ok(false) => break,
                                Ok(true) => {
                                    let _ = crossterm::event::read();
                                }
                                Err(_) => break,
                            }
                        }
                        continue;
                    }

                    match crossterm::event::poll(Duration::from_millis(50)) {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(_) => break,
                    }

                    if input_shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    if input_paused.load(Ordering::Relaxed) {
                        continue;
                    }

                    let Ok(event) = crossterm::event::read() else {
                        break;
                    };
                    if evt_tx.send(event).is_err() {
                        break;
                    }
                }
            }
        })?;

    // ── App ─────────────────────────────────────────────────
    let mut app = App::new(
        tool_registry.clone(),
        app_config.clone(),
        profile.clone(),
        session_service.clone(),
        approvals.clone(),
    )
    .await;
    app.input.set_use_shift_enter_hint(enhanced_keys_supported);
    app.footer.use_shift_enter_hint = enhanced_keys_supported;
    let mut current_agent_handle: Option<tokio::task::JoinHandle<()>> = None;

    let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));

    // First paint: avoid showing a blank alternate screen until the first event arrives.
    execute!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
    terminal.clear()?;
    terminal.draw(|f| app.render(f))?;
    execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;

    // ── Event loop ──────────────────────────────────────────
    let result: Result<(), Box<dyn std::error::Error>> = 'event_loop: loop {
        tokio::select! {
            Some(agent_event) = agent_rx.recv() => {
                if let Some(action) = app.update(TuiEvent::Agent(agent_event)) {
                    handle_action(
                        &mut app,
                        action,
                        &agent_loop,
                        &agent_tx,
                        &mut current_agent_handle,
                        &mut terminal,
                        &input_ctrl,
                    );
                }
            }
            Some(evt) = evt_rx.recv() => {
                let event = match evt {
                    crossterm::event::Event::Key(key) => TuiEvent::Key(key),
                    crossterm::event::Event::Mouse(mouse) => TuiEvent::Mouse(mouse),
                    crossterm::event::Event::Paste(text) => TuiEvent::Paste(text),
                    crossterm::event::Event::Resize(_, _) => TuiEvent::Tick,
                    _ => TuiEvent::Tick,
                };
                if let Some(action) = app.update(event) {
                    handle_action(
                        &mut app,
                        action,
                        &agent_loop,
                        &agent_tx,
                        &mut current_agent_handle,
                        &mut terminal,
                        &input_ctrl,
                    );
                }
            }
            _ = tick.tick() => {
                let _ = app.update(TuiEvent::Tick);
            }
        }

        if app.should_quit() {
            break 'event_loop Ok(());
        }

        // Append any newly committed transcript lines above the inline viewport (Codex-style).
        // This keeps chat history in the terminal's normal scrollback so multiplexers can copy it.
        // (Disabled) Inline viewport transcript injection is not compatible with stable cursor
        // placement for IME/composition while the model is streaming output.

        // Redraw.
        execute!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
        terminal.draw(|f| app.render(f))?;
        execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
    };

    // ── Cleanup ─────────────────────────────────────────────
    if let Some(handle) = current_agent_handle.take() {
        handle.abort();
    }
    input_shutdown.store(true, Ordering::Relaxed);
    drop(evt_rx);
    let _ = input_thread.join();
    keyboard_modes::reset_keyboard_reporting_after_exit();

    // Restore terminal modes best-effort (Codex-style): prefer returning the app error over
    // restoration failures, but avoid leaving the shell in a broken state.
    let mut restore_error: Option<std::io::Error> = None;
    let _ = execute!(
        std::io::stdout(),
        super::terminal_modes::DisableAlternateScroll
    );
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    if let Err(err) = execute!(std::io::stdout(), DisableBracketedPaste) {
        restore_error.get_or_insert(err);
    }
    let _ = execute!(std::io::stdout(), DisableFocusChange);
    if let Err(err) = disable_raw_mode() {
        restore_error.get_or_insert(err);
    }
    let _ = execute!(std::io::stdout(), SetCursorStyle::DefaultUserShape, Show);
    flush_terminal_input_buffer();

    match (result, restore_error) {
        (Err(err), _) => Err(err),
        (Ok(()), Some(err)) => Err(Box::new(err)),
        (Ok(()), None) => Ok(()),
    }
}

fn handle_action(
    app: &mut App,
    action: AppAction,
    agent_loop: &Arc<AgentLoop>,
    agent_tx: &mpsc::UnboundedSender<AgentEvent>,
    current_agent_handle: &mut Option<tokio::task::JoinHandle<()>>,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input_ctrl: &InputThreadControl,
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
        AppAction::QueueOrSubmit(_prompt) => {
            // This action is intended to be handled in `App::update` when the
            // composer has focus. If it reaches here, ignore it.
        }
        AppAction::BacktrackPrime | AppAction::BacktrackEditLast => {
            // These actions are also handled in `App::update`.
        }
        AppAction::OpenTranscript | AppAction::CopyLastResponse | AppAction::ToggleRawOutput => {
            // These actions are also handled in `App::update`.
        }
        AppAction::CopyText(text) => match copy_to_clipboard(&text) {
            Ok(method) => {
                app.footer.set_message(
                    format!("Copied ({})", method.label()),
                    std::time::Duration::from_secs(2),
                );
            }
            Err(err) => {
                app.footer
                    .set_message(err, std::time::Duration::from_secs(4));
            }
        },
        AppAction::SelectProfile(name) => {
            app.apply_profile_selection(name);
        }
        AppAction::SelectSession(session_id) => {
            app.apply_session_selection(session_id);
        }
        AppAction::SelectTheme(theme) => {
            app.apply_theme_selection(theme);
        }
        AppAction::OpenEditor(text) => match open_editor(terminal, input_ctrl, &text) {
            Ok(edited) => {
                app.input.load_text(&edited);
                app.footer
                    .set_message("Edited in $EDITOR.", std::time::Duration::from_secs(2));
            }
            Err(err) => {
                app.footer
                    .set_message(err, std::time::Duration::from_secs(4));
            }
        },
        AppAction::ShowHistorySearch => {
            let mut entries = app
                .history
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            entries.reverse();
            app.overlays
                .push(Box::new(HistorySearchOverlay::new(entries)));
        }
        AppAction::LoadInput(text) => {
            app.input.load_text(&text);
            app.footer
                .set_message("Loaded from history.", std::time::Duration::from_secs(2));
        }
    }
}

fn open_editor(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input_ctrl: &InputThreadControl,
    initial: &str,
) -> Result<String, String> {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use std::io::Write;
    use std::process::{Command, Stdio};

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis();
    let path = std::env::temp_dir().join(format!("xylitol-input-{}-{}.md", std::process::id(), ts));

    if let Err(err) = std::fs::write(&path, initial) {
        return Err(format!("Failed to write temp file: {err}"));
    }

    input_ctrl.pause();
    // Give the input thread a brief window to exit poll/read loops before we
    // hand terminal ownership to an external program.
    std::thread::sleep(std::time::Duration::from_millis(75));

    // Suspend TUI.
    keyboard_modes::restore_keyboard_enhancement_stack();
    if let Err(err) = disable_raw_mode() {
        keyboard_modes::enable_keyboard_enhancement();
        input_ctrl.resume();
        return Err(format!("disable_raw_mode failed: {err}"));
    }
    let _ = terminal.show_cursor();
    let _ = std::io::stdout().flush();

    // Launch editor.
    let editor_cmd = resolve_editor_command();
    let Some(bin) = editor_cmd.first() else {
        return Err("Editor command is empty.".to_string());
    };

    let status = Command::new(bin)
        .args(editor_cmd.iter().skip(1))
        .arg(&path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run editor '{bin}': {e}"))?;
    if !status.success() {
        // Still attempt to restore and read back.
        tracing::warn!("Editor exited with status: {status}");
    }

    // Restore TUI.
    if let Err(err) = enable_raw_mode() {
        keyboard_modes::enable_keyboard_enhancement();
        input_ctrl.resume();
        return Err(format!("enable_raw_mode failed: {err}"));
    }
    keyboard_modes::enable_keyboard_enhancement();
    flush_terminal_input_buffer();
    input_ctrl.request_drain();
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();

    input_ctrl.resume();

    let edited = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Ok(edited)
}

fn resolve_editor_command() -> Vec<String> {
    let raw = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vim".into());

    #[cfg(windows)]
    {
        raw.split_whitespace().map(|s| s.to_string()).collect()
    }

    #[cfg(not(windows))]
    {
        shlex::split(&raw)
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| raw.split_whitespace().map(|s| s.to_string()).collect())
    }
}

fn flush_terminal_input_buffer() {
    #[cfg(unix)]
    {
        // Safety: flushing the stdin queue is safe and does not move ownership.
        let result = unsafe { libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            tracing::warn!("failed to tcflush stdin: {err}");
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<CopyMethod, String> {
    copy_to_clipboard_with_context(text, detect_copy_context())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyContext {
    LocalDirect,
    RemoteOrMux { in_tmux: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyMethod {
    Osc52,
    Osc52Tmux,
    Pbcopy,
    WlCopy,
    Xclip,
    Xsel,
    TempFile,
}

impl CopyMethod {
    fn label(self) -> &'static str {
        match self {
            CopyMethod::Osc52 => "OSC52",
            CopyMethod::Osc52Tmux => "OSC52/tmux",
            CopyMethod::Pbcopy => "pbcopy",
            CopyMethod::WlCopy => "wl-copy",
            CopyMethod::Xclip => "xclip",
            CopyMethod::Xsel => "xsel",
            CopyMethod::TempFile => "temp file",
        }
    }
}

fn detect_copy_context() -> CopyContext {
    fn has(name: &str) -> bool {
        std::env::var_os(name).is_some()
    }

    let in_tmux = has("TMUX");
    let in_zellij = has("ZELLIJ");
    let in_ssh = has("SSH_CONNECTION") || has("SSH_CLIENT") || has("SSH_TTY");
    if in_ssh || in_tmux || in_zellij {
        CopyContext::RemoteOrMux { in_tmux }
    } else {
        CopyContext::LocalDirect
    }
}

fn copy_to_clipboard_with_context(text: &str, context: CopyContext) -> Result<CopyMethod, String> {
    match context {
        CopyContext::RemoteOrMux { in_tmux } => {
            // In SSH / multiplexers prefer OSC52 so the clipboard lands on the local machine.
            match copy_via_osc52(text, in_tmux) {
                Ok(method) => Ok(method),
                Err(_err) => {
                    let path = write_temp_copy_file(text)?;
                    Err(format!(
                        "Clipboard not supported; wrote to {}",
                        path.display()
                    ))
                }
            }
        }
        CopyContext::LocalDirect => {
            // In a local terminal, prefer the OS clipboard tooling; fall back to OSC52.
            copy_via_system_clipboard(text)
                .or_else(|_| copy_via_osc52(text, /*in_tmux*/ false))
                .or_else(|_err| {
                    let path = write_temp_copy_file(text)?;
                    Err(format!(
                        "Clipboard not supported; wrote to {}",
                        path.display()
                    ))
                })
        }
    }
}

fn copy_via_system_clipboard(text: &str) -> Result<CopyMethod, String> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run pbcopy: {e}"))?;
        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| "Failed to open pbcopy stdin".to_string())?;
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("Failed to write to pbcopy: {e}"))?;
        }
        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for pbcopy: {e}"))?;
        if status.success() {
            return Ok(CopyMethod::Pbcopy);
        }
        return Err(format!("pbcopy exited with {status}"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(windows)]
        {
            let _ = text;
            Err("Clipboard not supported on this platform.".to_string())
        }

        #[cfg(not(windows))]
        {
            // Linux/BSD: prefer wl-copy on Wayland, then xclip/xsel on X11.
            if std::env::var_os("WAYLAND_DISPLAY").is_some()
                && let Ok(method) = copy_via_command_stdin("wl-copy", &[], text, CopyMethod::WlCopy)
            {
                return Ok(method);
            }

            if std::env::var_os("DISPLAY").is_some() {
                if let Ok(method) = copy_via_command_stdin(
                    "xclip",
                    &["-selection", "clipboard"],
                    text,
                    CopyMethod::Xclip,
                ) {
                    return Ok(method);
                }
                if let Ok(method) = copy_via_command_stdin("xsel", &["-ib"], text, CopyMethod::Xsel)
                {
                    return Ok(method);
                }
            }

            Err("No system clipboard helper found (wl-copy/xclip/xsel).".to_string())
        }
    }
}

fn copy_via_command_stdin(
    bin: &str,
    args: &[&str],
    text: &str,
    method: CopyMethod,
) -> Result<CopyMethod, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to run {bin}: {e}"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| format!("Failed to open {bin} stdin"))?;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to {bin}: {e}"))?;
    }
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for {bin}: {e}"))?;
    if status.success() {
        Ok(method)
    } else {
        Err(format!("{bin} exited with {status}"))
    }
}

fn copy_via_osc52(text: &str, in_tmux: bool) -> Result<CopyMethod, String> {
    use std::io::Write;

    // Many terminals impose size limits on OSC52 payloads; keep it bounded and fall back.
    const MAX_B64_CHARS: usize = 100_000;
    let b64 = base64_encode(text.as_bytes());
    if b64.len() > MAX_B64_CHARS {
        return Err(format!(
            "OSC52 payload too large ({} chars); falling back.",
            b64.len()
        ));
    }

    let seq = if in_tmux {
        // Wrap the OSC sequence in a DCS passthrough so tmux forwards it.
        format!("\x1bPtmux;\x1b\x1b]52;c;{}\x07\x1b\\", b64)
    } else {
        format!("\x1b]52;c;{}\x07", b64)
    };

    let mut out = std::io::stdout();
    out.write_all(seq.as_bytes())
        .map_err(|e| format!("Failed to write OSC52 sequence: {e}"))?;
    out.flush()
        .map_err(|e| format!("Failed to flush OSC52 sequence: {e}"))?;

    Ok(if in_tmux {
        CopyMethod::Osc52Tmux
    } else {
        CopyMethod::Osc52
    })
}

fn write_temp_copy_file(text: &str) -> Result<std::path::PathBuf, String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis();
    let path = std::env::temp_dir().join(format!("xylitol-copy-{}-{}.md", std::process::id(), ts));
    std::fs::write(&path, text).map_err(|e| format!("Failed to write {path:?}: {e}"))?;
    Ok(path)
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);

    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        out.push(TABLE[(n & 0x3f) as usize] as char);
        i += 3;
    }

    match data.len().saturating_sub(i) {
        0 => {}
        1 => {
            let n = (data[i] as u32) << 16;
            out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
            out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => unreachable!(),
    }

    out
}
