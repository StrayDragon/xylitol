//! c485 synthetic vertical-slice harness — ScriptedDriver + host/driver pump.
//!
//! Only compiled in unit tests (`cfg(test)`). Stays inside `app/tui` and talks
//! to the core solely via [`crate::app::core::driver::Driver`] (arch_guard).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::{
    CommandInfo, Driver, EventStream, ModelInfo, QueueStats, SessionStats, XyEvent,
};
use crate::domain::session_types::SessionEntry;
use crate::domain::types::ThinkingLevel;
use crate::protocol::Command;
use crate::runtime_protocol::XyBashResult;

use super::host::{HostEvent, HostSession, PendingSlash};

/// Test double: canned `run` streams + call recording for steer/abort/queues/bash.
pub struct ScriptedDriver {
    pub runs: Vec<String>,
    pub steer_calls: Vec<String>,
    pub follow_up_calls: Vec<String>,
    pub clear_calls: Vec<(bool, bool)>,
    pub bash_calls: Vec<(String, bool)>,
    abort_count: AtomicUsize,
    scripts: VecDeque<Vec<XyEvent>>,
    default_script: Vec<XyEvent>,
    bash_results: VecDeque<XyBashResult>,
    default_bash: XyBashResult,
    steer_queued: usize,
    follow_up_queued: usize,
    model: ModelInfo,
}

impl ScriptedDriver {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            steer_calls: Vec::new(),
            follow_up_calls: Vec::new(),
            clear_calls: Vec::new(),
            bash_calls: Vec::new(),
            abort_count: AtomicUsize::new(0),
            scripts: VecDeque::new(),
            default_script: vec![
                XyEvent::AgentStart {
                    session_id: "s".into(),
                    model: "fake".into(),
                },
                XyEvent::TextDelta("hello".into()),
                XyEvent::AgentEnd {
                    messages: Vec::new(),
                },
            ],
            bash_results: VecDeque::new(),
            default_bash: XyBashResult {
                output: "ok".into(),
                exit_code: Some(0),
                cancelled: false,
                truncated: false,
                full_output_path: None,
            },
            steer_queued: 0,
            follow_up_queued: 0,
            model: ModelInfo {
                id: "fake".into(),
                display_name: "Fake".into(),
                thinking: false,
                context_window: 8_000,
            },
        }
    }

    pub fn push_script(&mut self, events: Vec<XyEvent>) {
        self.scripts.push_back(events);
    }

    pub fn set_default_script(&mut self, events: Vec<XyEvent>) {
        self.default_script = events;
    }

    pub fn push_bash_result(&mut self, result: XyBashResult) {
        self.bash_results.push_back(result);
    }

    pub fn abort_count(&self) -> usize {
        self.abort_count.load(Ordering::SeqCst)
    }
}

impl Default for ScriptedDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Driver for ScriptedDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.runs.push(prompt.to_string());
        let events = self
            .scripts
            .pop_front()
            .unwrap_or_else(|| self.default_script.clone());
        Box::pin(futures::stream::iter(events))
    }

    fn abort(&self) {
        self.abort_count.fetch_add(1, Ordering::SeqCst);
    }

    fn current_model(&self) -> Option<ModelInfo> {
        Some(self.model.clone())
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        vec![self.model.clone()]
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, String> {
        self.model = ModelInfo {
            id: model_id.into(),
            display_name: model_id.into(),
            thinking: false,
            context_window: 8_000,
        };
        Ok(self.model.clone())
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, String> {
        Ok(self.model.clone())
    }

    fn set_thinking_level(&mut self, _level: ThinkingLevel) {}

    fn thinking_level(&self) -> ThinkingLevel {
        ThinkingLevel::Off
    }

    fn session_id(&self) -> Option<String> {
        Some("scripted".into())
    }

    async fn execute_bash(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, String> {
        self.bash_calls
            .push((command.to_string(), exclude_from_context));
        Ok(self
            .bash_results
            .pop_front()
            .unwrap_or_else(|| self.default_bash.clone()))
    }

    async fn compact(&mut self) -> Result<bool, String> {
        Ok(false)
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, String> {
        Ok(path.display().to_string())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, String> {
        Ok(path.display().to_string())
    }

    async fn import_jsonl(&mut self, _path: &Path) -> Result<String, String> {
        Ok("imported".into())
    }

    async fn fork_session(&mut self, _entry_id: &str) -> Result<String, String> {
        Ok("forked".into())
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, String> {
        Ok(session_id.into())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, String> {
        Ok(Vec::new())
    }

    async fn get_session_stats(&self) -> Result<SessionStats, String> {
        Err("scripted: no stats".into())
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        Vec::new()
    }

    fn steer(&mut self, message: &str) -> Result<(), String> {
        self.steer_calls.push(message.to_string());
        self.steer_queued += 1;
        Ok(())
    }

    fn follow_up(&mut self, message: &str) -> Result<(), String> {
        self.follow_up_calls.push(message.to_string());
        self.follow_up_queued += 1;
        Ok(())
    }

    fn clear_queue(&mut self, clear_steer: bool, clear_follow_up: bool) -> Result<(), String> {
        self.clear_calls.push((clear_steer, clear_follow_up));
        if clear_steer {
            self.steer_queued = 0;
        }
        if clear_follow_up {
            self.follow_up_queued = 0;
        }
        Ok(())
    }

    fn queue_stats(&self) -> QueueStats {
        QueueStats {
            steer_count: self.steer_queued,
            follow_up_count: self.follow_up_queued,
        }
    }
}

/// One pump of host pending ops + optional full drain of the active agent stream.
/// Mirrors `run_host_loop` ordering (c485).
pub async fn pump_host_driver<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), String> {
    if session.take_abort() {
        driver.abort();
        let _ = driver.clear_queue(true, false);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if session.take_dequeue() {
        let _ = driver.clear_queue(true, true);
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_steer() {
        if let Err(e) = driver.steer(&msg) {
            session.push_system_note(format!("steer failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(msg) = session.take_follow_up() {
        if let Err(e) = driver.follow_up(&msg) {
            session.push_system_note(format!("follow-up failed: {e}"));
        }
        let stats = driver.queue_stats();
        session.set_queue_badge(stats.steer_count, stats.follow_up_count);
        let _ = session.render_now();
    }
    if let Some(slash) = session.take_slash() {
        match slash {
            PendingSlash::Exit => {
                session.request_quit();
            }
            PendingSlash::CycleModel => {
                match dispatch(driver, Command::CycleModel { id: None }).await {
                    Ok(DispatchOutcome::Model(m)) => {
                        let label = if m.display_name.is_empty() {
                            m.id
                        } else {
                            m.display_name
                        };
                        session.set_footer_model(label.clone());
                        session.push_system_note(format!("model → {label}"));
                    }
                    Ok(_) => session.push_system_note("model cycled"),
                    Err(e) => session.push_system_note(format!("/model failed: {e}")),
                }
                let _ = session.render_now();
            }
            PendingSlash::SetModel(model_id) => {
                match dispatch(
                    driver,
                    Command::SetModel {
                        id: None,
                        provider: String::new(),
                        model_id,
                    },
                )
                .await
                {
                    Ok(DispatchOutcome::Model(m)) => {
                        let label = if m.display_name.is_empty() {
                            m.id
                        } else {
                            m.display_name
                        };
                        session.set_footer_model(label.clone());
                        session.push_system_note(format!("model → {label}"));
                    }
                    Ok(_) => session.push_system_note("model set"),
                    Err(e) => session.push_system_note(format!("/model failed: {e}")),
                }
                let _ = session.render_now();
            }
        }
    }

    if let Some(bash) = session.take_bash() {
        match dispatch(
            driver,
            Command::Bash {
                id: None,
                command: bash.command.clone(),
                exclude_from_context: bash.exclude_from_context,
            },
        )
        .await
        {
            Ok(DispatchOutcome::Bash(result)) => {
                session.push_bash_result(&bash.command, &result);
            }
            Ok(_) => session.push_system_note("bash: unexpected dispatch outcome"),
            Err(e) => session.push_system_note(format!("bash failed: {e}")),
        }
        let _ = session.render_now();
    }

    if agent_stream.is_none()
        && let Some(prompt) = session.take_submit()
    {
        session.on_run_started(&prompt);
        let _ = session.render_now();
        *agent_stream = Some(driver.run(&prompt).await);
    }

    if let Some(stream) = agent_stream.as_mut() {
        while let Some(xy) = stream.next().await {
            session.step(HostEvent::Xy(Box::new(xy)))?;
        }
        *agent_stream = None;
        session.on_run_stream_closed();
        let _ = session.render_now();
    }

    if session.should_quit() {
        session.tui.finish_inline();
    }
    Ok(())
}

#[cfg(test)]
mod slice_tests {
    use super::*;
    use crate::app::tui::bridge::{UiEntry, UiPhase};
    use crate::app::tui::host::{HostEvent, HostSession};
    use xylitol_tui::{Component, InputEvent, Terminal};

    struct TestTerminal {
        cols: u16,
        rows: u16,
        frames: Vec<String>,
        started: bool,
        stopped: bool,
    }

    impl TestTerminal {
        fn new(cols: u16, rows: u16) -> Self {
            Self {
                cols,
                rows,
                frames: Vec::new(),
                started: false,
                stopped: false,
            }
        }
    }

    impl Terminal for TestTerminal {
        fn write(&mut self, data: &str) {
            self.frames.push(data.to_string());
        }
        fn columns(&self) -> u16 {
            self.cols
        }
        fn rows(&self) -> u16 {
            self.rows
        }
        fn hide_cursor(&mut self) {}
        fn show_cursor(&mut self) {}
        fn clear_line(&mut self) {}
        fn clear_from_cursor(&mut self) {}
        fn clear_screen(&mut self) {}
        fn flush(&mut self) {}
        fn set_size_hint(&mut self, cols: u16, rows: u16) {
            self.cols = cols;
            self.rows = rows;
        }
        fn start(&mut self) {
            self.started = true;
        }
        fn stop(&mut self) {
            self.stopped = true;
        }
    }

    fn enter_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn alt_enter_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn esc_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn alt_up_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctrl_g_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[tokio::test]
    async fn h1_idle_enter_runs_driver() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("hi");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.runs, vec!["hi".to_string()]);
        assert_eq!(session.ui_model().phase, UiPhase::Idle);
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Assistant { text } if text.contains("hello")))
        );
    }

    #[tokio::test]
    async fn h2_stream_then_idle() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        session.on_run_started("prompt");
        session
            .step(HostEvent::Xy(Box::new(XyEvent::TextDelta("A".into()))))
            .unwrap();
        session
            .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
                messages: Vec::new(),
            })))
            .unwrap();
        assert_eq!(session.ui_model().phase, UiPhase::Idle);
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Assistant { text } if text == "A"))
        );
    }

    #[tokio::test]
    async fn h3_tool_entry_visible() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        session.on_run_started("tool please");
        session
            .step(HostEvent::Xy(Box::new(XyEvent::ToolExecutionStart {
                id: "t1".into(),
                name: "bash".into(),
                args: serde_json::json!({"cmd": "ls"}),
            })))
            .unwrap();
        session
            .step(HostEvent::Xy(Box::new(XyEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "bash".into(),
                result: "ok".into(),
                is_error: false,
            })))
            .unwrap();
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Tool { name, .. } if name == "bash"))
        );
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("bash")),
            "missing tool in frame: {frame:?}"
        );
    }

    #[tokio::test]
    async fn h4_busy_steer_calls_driver() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("hello");
        root.borrow_mut().set_editor_text("nudge");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.steer_calls, vec!["nudge".to_string()]);
        let frame = root.borrow_mut().render(80);
        assert!(frame.iter().any(|l| l.contains("Steering: nudge")));
        assert!(
            !session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("[steer]")))
        );
    }

    #[tokio::test]
    async fn h5_follow_up_calls_driver() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("hello");
        root.borrow_mut().set_editor_text("later");
        session.step(HostEvent::Input(alt_enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.follow_up_calls, vec!["later".to_string()]);
        let frame = root.borrow_mut().render(80);
        assert!(frame.iter().any(|l| l.contains("Follow-up: later")));
    }

    #[tokio::test]
    async fn h6_alt_up_clears_both_queues() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("hello");
        root.borrow_mut().set_editor_text("nudge");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        root.borrow_mut().set_editor_text("draft");
        session.step(HostEvent::Input(alt_up_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.clear_calls.iter().any(|&(s, f)| s && f),
            "expected clear both: {:?}",
            driver.clear_calls
        );
        assert_eq!(root.borrow().editor_text(), "nudge\n\ndraft");
    }

    #[tokio::test]
    async fn h7_abort_then_second_run() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("first");
        session.step(HostEvent::Input(esc_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.abort_count(), 1);
        assert!(
            driver.clear_calls.iter().any(|&(s, f)| s && !f),
            "abort clears steer only: {:?}",
            driver.clear_calls
        );
        session.on_run_stream_closed();
        assert!(!session.run_active());
        root.borrow_mut().set_editor_text("second");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.runs, vec!["second".to_string()]);
    }

    #[tokio::test]
    async fn h8_exit_finish_inline() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("/exit");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(session.should_quit());
        assert!(
            session.tui.terminal.stopped,
            "finish_inline must stop TestTerminal"
        );
    }

    #[tokio::test]
    async fn h9_model_slash_cycles() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("model"))),
            "expected model note: {:?}",
            session.ui_model().entries
        );
    }

    // ── c492 bang-bash (B1–B7) ─────────────────────────────────────

    #[test]
    fn b1_b2_bang_border_toggles() {
        let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("!ls");
        assert!(root.borrow().bash_mode(), "B1: !ls enables bash border");
        root.borrow_mut().set_editor_text("hello");
        assert!(!root.borrow().bash_mode(), "B2: clear restores muted");
    }

    #[tokio::test]
    async fn b3_idle_bang_execute_bash_not_run() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("!echo hi");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.runs.is_empty(),
            "MUST NOT call run: {:?}",
            driver.runs
        );
        assert_eq!(
            driver.bash_calls,
            vec![("echo hi".to_string(), false)],
            "B3: execute_bash once"
        );
    }

    #[tokio::test]
    async fn b4_bangbang_exclude_from_context() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("!!echo x");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.bash_calls, vec![("echo x".to_string(), true)]);
    }

    #[tokio::test]
    async fn b5_bash_ok_in_scrollback() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.push_bash_result(XyBashResult {
            output: "hello-out".into(),
            exit_code: Some(0),
            ..Default::default()
        });
        let mut stream = None;
        root.borrow_mut().set_editor_text("!echo hi");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let entries = &session.ui_model().entries;
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("$ echo hi"))),
            "summary missing: {entries:?}"
        );
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("hello-out"))),
            "output missing: {entries:?}"
        );
    }

    #[tokio::test]
    async fn b6_bash_nonzero_error_entry() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.push_bash_result(XyBashResult {
            output: "boom".into(),
            exit_code: Some(1),
            ..Default::default()
        });
        let mut stream = None;
        root.borrow_mut().set_editor_text("!false");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Error { text } if text.contains("exit 1"))),
            "B6 expected Error tint: {:?}",
            session.ui_model().entries
        );
    }

    #[test]
    fn b7_ctrl_g_stub() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("draft");
        session.step(HostEvent::Input(ctrl_g_event())).unwrap();
        assert_eq!(root.borrow().external_editor_invocations(), 1);
        assert!(
            root.borrow().editor_text().contains("$EDITOR stub"),
            "stub marker: {}",
            root.borrow().editor_text()
        );
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("Ctrl+G"))),
            "system note: {:?}",
            session.ui_model().entries
        );
    }

    #[test]
    fn bang_parse_helpers() {
        use crate::app::tui::host::{BangParse, parse_bang_command};
        assert_eq!(parse_bang_command("hi"), BangParse::NotBang);
        assert_eq!(
            parse_bang_command("!"),
            BangParse::Empty {
                exclude_from_context: false
            }
        );
        assert_eq!(
            parse_bang_command("!!"),
            BangParse::Empty {
                exclude_from_context: true
            }
        );
        assert_eq!(
            parse_bang_command("  ! ls -la "),
            BangParse::Cmd {
                command: "ls -la".into(),
                exclude_from_context: false
            }
        );
    }
}
