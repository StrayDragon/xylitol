//! Root app component and event loop for the TUI.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use adk_session::{ListRequest, SessionService};
use syntect::highlighting::ThemeSet;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use crate::agent::profile::ResolvedProfile;
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

pub(crate) struct App {
    chat: ChatComponent,
    tool_panel: ToolPanelComponent,
    input: InputComponent,
    status_bar: StatusBar,
    footer: FooterState,
    overlays: OverlayStack,

    keymap: RuntimeKeymap,

    app_config: AppConfig,
    session_service: Arc<dyn SessionService>,
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
        session_service: Arc<dyn SessionService>,
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

        let mut session_choices: Vec<String> = match session_service
            .list(ListRequest {
                app_name: "xylitol".into(),
                user_id: "default-user".into(),
                limit: Some(200),
                offset: None,
            })
            .await
        {
            Ok(sessions) => sessions.into_iter().map(|s| s.id().to_string()).collect(),
            Err(err) => {
                tracing::warn!(error = %err, "Failed to list sessions.");
                Vec::new()
            }
        };
        session_choices.sort();

        let mut theme_choices: Vec<String> =
            ThemeSet::load_defaults().themes.keys().cloned().collect();
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
        self.footer.message = "Cleared.".to_string();
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
                self.footer.message = format!("Profile: {name}");
            }
            Err(err) => {
                self.status_bar.set_model(name.clone());
                self.footer.model_label = name.clone();
                self.footer.message = format!("Profile '{name}' not ready: {err}");
            }
        }
    }

    fn apply_session_selection(&mut self, session_id: String) {
        self.session_id = session_id.clone();
        self.status_bar.set_session(session_id);
        self.footer.session_label = self.session_id.clone();
        self.footer.message = "Session switched.".to_string();
    }

    fn apply_theme_selection(&mut self, theme: String) {
        if self.chat.set_theme(&theme) {
            self.footer.message = format!("Theme: {theme}");
        } else {
            self.footer.message = format!("Theme not found: {theme}");
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
                    self.footer.message = "No diffs to preview.".to_string();
                }

                None
            }

            AppKeyAction::OpenTranscript => Some(AppAction::OpenTranscript),
            AppKeyAction::CopyLastResponse => Some(AppAction::CopyLastResponse),
            AppKeyAction::ToggleRawOutput => Some(AppAction::ToggleRawOutput),
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
                    self.footer.message = format!("Approval required: {name}");
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
                                if let Some(text) = self.chat.last_assistant_message() {
                                    if let Err(err) = copy_to_clipboard(&text) {
                                        self.footer.message = err;
                                    } else {
                                        self.footer.message = "Copied last response.".to_string();
                                    }
                                } else {
                                    self.footer.message = "No assistant response yet.".to_string();
                                }
                                None
                            }
                            Some(AppAction::ToggleRawOutput) => {
                                self.raw_output = !self.raw_output;
                                self.chat.set_raw_output(self.raw_output);
                                self.input.set_raw_output(self.raw_output);
                                let msg = if self.raw_output {
                                    "Raw output: ON"
                                } else {
                                    "Raw output: OFF"
                                };
                                self.footer.message = msg.to_string();
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
                                self.footer.message =
                                    "Backtrack: press Esc again to edit last message.".to_string();
                                None
                            }
                            Some(AppAction::BacktrackEditLast) => {
                                if self.running {
                                    return None;
                                }
                                self.backtrack_primed = false;
                                if let Some(last) = self.last_user_message() {
                                    self.input.load_text(&last);
                                    self.footer.message = "Editing last message.".to_string();
                                } else {
                                    self.footer.message = "No previous user message.".to_string();
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
            TuiEvent::Mouse(mouse) => {
                use crossterm::event::{MouseButton, MouseEventKind};
                use ratatui::layout::Position;

                let pos = Position {
                    x: mouse.column,
                    y: mouse.row,
                };

                match mouse.kind {
                    MouseEventKind::ScrollUp if self.chat_area.contains(pos) => {
                        self.chat.scroll_wheel_up(3);
                    }
                    MouseEventKind::ScrollDown if self.chat_area.contains(pos) => {
                        self.chat.scroll_wheel_down(3);
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        if self.input_area.contains(pos) {
                            self.focus = Focus::Input;
                        } else if self.chat_area.contains(pos) || self.tool_area.contains(pos) {
                            self.focus = Focus::Chat;
                        }
                    }
                    _ => {}
                }

                None
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
        // Codex-style layout: transcript + bottom pane (composer + footer).
        // We temporarily keep the legacy tool panel disabled by default (height=0) so the layout
        // matches codex visually. Future work can re-introduce tool output as inline transcript
        // items instead of a separate panel.
        let footer_h = 1u16;
        let input_h = self.input.desired_height();
        let bottom_h = input_h.saturating_add(footer_h).max(2);

        let size_changed = self.last_area != area;
        let input_changed = self.last_input_height != bottom_h;
        if size_changed || input_changed {
            self.last_area = area;
            self.last_input_height = bottom_h;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(bottom_h)].as_ref())
            .split(area);

        self.chat_area = chunks[0];
        self.tool_area = Rect::new(0, 0, 0, 0);
        self.input_area = Rect::new(
            chunks[1].x,
            chunks[1].y,
            chunks[1].width,
            chunks[1].height.saturating_sub(footer_h).max(1),
        );
        self.status_area = Rect::new(
            chunks[1].x,
            chunks[1]
                .y
                .saturating_add(chunks[1].height.saturating_sub(footer_h)),
            chunks[1].width,
            footer_h,
        );

        self.chat.set_focused(self.focus == Focus::Chat);
        self.input.set_focused(self.focus == Focus::Input);

        // Ratatui uses immediate-mode rendering: every `Terminal::draw` starts from an empty buffer.
        // We must render all visible components every frame; "dirty" flags should only be used to
        // decide whether a draw is needed at all, not to skip rendering within a draw.
        self.chat.render(frame, self.chat_area);
        self.input.render(frame, self.input_area);

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
            )) as Arc<dyn adk_core::Tool>
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
    let input_paused = Arc::new(AtomicBool::new(false));
    let input_shutdown = Arc::new(AtomicBool::new(false));

    // Spawn crossterm keyboard reader on a dedicated OS thread.
    //
    // NOTE: avoid `tokio::task::spawn_blocking` for this long-lived loop. Tokio waits for all
    // blocking tasks to finish when shutting down the runtime, which can cause quit to hang.
    let input_thread = std::thread::Builder::new()
        .name("xylitol-tui-input".into())
        .spawn({
            let input_paused = input_paused.clone();
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

                    match crossterm::event::poll(Duration::from_millis(50)) {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(_) => break,
                    }

                    if input_shutdown.load(Ordering::Relaxed) {
                        break;
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
                        &input_paused,
                    );
                }
            }
            Some(evt) = evt_rx.recv() => {
                let event = match evt {
                    crossterm::event::Event::Key(key) => TuiEvent::Key(key),
                    crossterm::event::Event::Mouse(mouse) => TuiEvent::Mouse(mouse),
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
                        &input_paused,
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
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input_paused: &Arc<AtomicBool>,
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
        AppAction::SelectProfile(name) => {
            app.apply_profile_selection(name);
        }
        AppAction::SelectSession(session_id) => {
            app.apply_session_selection(session_id);
        }
        AppAction::SelectTheme(theme) => {
            app.apply_theme_selection(theme);
        }
        AppAction::OpenEditor(text) => match open_editor(terminal, input_paused, &text) {
            Ok(edited) => {
                app.input.load_text(&edited);
                app.footer.message = "Edited in $EDITOR.".to_string();
            }
            Err(err) => {
                app.footer.message = err;
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
            app.footer.message = "Loaded from history.".to_string();
        }
    }
}

fn open_editor(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input_paused: &Arc<AtomicBool>,
    initial: &str,
) -> Result<String, String> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };
    use std::io::Write;
    use std::process::Command;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis();
    let path = std::env::temp_dir().join(format!("xylitol-input-{}-{}.md", std::process::id(), ts));

    if let Err(err) = std::fs::write(&path, initial) {
        return Err(format!("Failed to write temp file: {err}"));
    }

    input_paused.store(true, Ordering::Relaxed);

    // Suspend TUI.
    if let Err(err) = disable_raw_mode() {
        input_paused.store(false, Ordering::Relaxed);
        return Err(format!("disable_raw_mode failed: {err}"));
    }
    if let Err(err) = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ) {
        let _ = enable_raw_mode();
        input_paused.store(false, Ordering::Relaxed);
        return Err(format!("LeaveAlternateScreen failed: {err}"));
    }
    let _ = terminal.show_cursor();
    let _ = std::io::stdout().flush();

    // Launch editor.
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or("vim");
    let args: Vec<&str> = parts.collect();

    let status = Command::new(bin)
        .args(args)
        .arg(&path)
        .status()
        .map_err(|e| format!("Failed to run editor '{bin}': {e}"))?;
    if !status.success() {
        // Still attempt to restore and read back.
        tracing::warn!("Editor exited with status: {status}");
    }

    // Restore TUI.
    if let Err(err) = execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    ) {
        input_paused.store(false, Ordering::Relaxed);
        return Err(format!("EnterAlternateScreen failed: {err}"));
    }
    if let Err(err) = enable_raw_mode() {
        input_paused.store(false, Ordering::Relaxed);
        return Err(format!("enable_raw_mode failed: {err}"));
    }
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();

    input_paused.store(false, Ordering::Relaxed);

    let edited = std::fs::read_to_string(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Ok(edited)
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
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
            return Ok(());
        }
        Err(format!("pbcopy exited with {status}"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_millis();
        let path =
            std::env::temp_dir().join(format!("xylitol-copy-{}-{}.md", std::process::id(), ts));
        std::fs::write(&path, text)
            .map_err(|e| format!("Clipboard not supported; wrote to {path:?}: {e}"))?;
        Err(format!(
            "Clipboard not supported; wrote to {}",
            path.display()
        ))
    }
}
