//! c485 synthetic vertical-slice harness — ScriptedDriver + host/driver pump.
//!
//! Only compiled in unit tests (`cfg(test)`). Stays inside `app/tui` and talks
//! to the core solely via [`crate::app::core::driver::Driver`] (arch_guard).

use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use xylitol_tui::Terminal;

use crate::app::core::driver::{
    CommandInfo, Driver, EventStream, ModelInfo, QueueStats, SessionStats, XyEvent,
};
use crate::domain::session_types::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};
use crate::domain::types::ThinkingLevel;
use crate::runtime_protocol::XyBashResult;

use super::effects::{drain_pending, run_pending_bash};
use super::host::{HostEvent, HostSession};

/// Test double: canned `run` streams + call recording for steer/abort/queues/bash.
pub struct ScriptedDriver {
    pub runs: Vec<String>,
    pub steer_calls: Vec<String>,
    pub follow_up_calls: Vec<String>,
    pub clear_calls: Vec<(bool, bool)>,
    bash_calls: Mutex<Vec<(String, bool)>>,
    abort_count: AtomicUsize,
    /// When true, [`Self::execute_bash`] waits until [`Self::abort`] (c665).
    hang_bash_until_abort: AtomicBool,
    aborted: AtomicBool,
    scripts: VecDeque<Vec<XyEvent>>,
    default_script: Vec<XyEvent>,
    bash_results: Mutex<VecDeque<XyBashResult>>,
    default_bash: XyBashResult,
    steer_queued: usize,
    follow_up_queued: usize,
    model: ModelInfo,
    message_history_tree: Vec<SessionTreeNode>,
    session_messages: Vec<SessionEntry>,
    travel_overrides: HashMap<String, SessionTreeTravel>,
    session_tree_calls: AtomicUsize,
    travel_calls: Mutex<Vec<String>>,
}

impl ScriptedDriver {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            steer_calls: Vec::new(),
            follow_up_calls: Vec::new(),
            clear_calls: Vec::new(),
            bash_calls: Mutex::new(Vec::new()),
            abort_count: AtomicUsize::new(0),
            hang_bash_until_abort: AtomicBool::new(false),
            aborted: AtomicBool::new(false),
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
            bash_results: Mutex::new(VecDeque::new()),
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
            message_history_tree: Vec::new(),
            session_messages: Vec::new(),
            travel_overrides: HashMap::new(),
            session_tree_calls: AtomicUsize::new(0),
            travel_calls: Mutex::new(Vec::new()),
        }
    }

    pub fn set_message_history_tree(&mut self, tree: Vec<SessionTreeNode>) {
        self.message_history_tree = tree;
    }

    pub fn set_session_messages(&mut self, entries: Vec<SessionEntry>) {
        self.session_messages = entries;
    }

    pub fn set_travel_override(&mut self, entry_id: impl Into<String>, travel: SessionTreeTravel) {
        self.travel_overrides.insert(entry_id.into(), travel);
    }

    pub fn session_tree_calls(&self) -> usize {
        self.session_tree_calls.load(Ordering::SeqCst)
    }

    pub fn travel_calls(&self) -> Vec<String> {
        self.travel_calls.lock().expect("travel_calls").clone()
    }

    pub fn push_script(&mut self, events: Vec<XyEvent>) {
        self.scripts.push_back(events);
    }

    pub fn set_default_script(&mut self, events: Vec<XyEvent>) {
        self.default_script = events;
    }

    pub fn push_bash_result(&mut self, result: XyBashResult) {
        self.bash_results
            .lock()
            .expect("bash_results")
            .push_back(result);
    }

    pub fn set_hang_bash_until_abort(&self, hang: bool) {
        self.hang_bash_until_abort.store(hang, Ordering::SeqCst);
        self.aborted.store(false, Ordering::SeqCst);
    }

    pub fn bash_calls(&self) -> Vec<(String, bool)> {
        self.bash_calls.lock().expect("bash_calls").clone()
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
        self.aborted.store(true, Ordering::SeqCst);
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
        &self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, String> {
        self.bash_calls
            .lock()
            .expect("bash_calls")
            .push((command.to_string(), exclude_from_context));
        if self.hang_bash_until_abort.load(Ordering::SeqCst) {
            while !self.aborted.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            return Ok(XyBashResult {
                output: String::new(),
                exit_code: None,
                cancelled: true,
                truncated: false,
                full_output_path: None,
            });
        }
        Ok(self
            .bash_results
            .lock()
            .expect("bash_results")
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
        Ok(self.session_messages.clone())
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

    async fn session_tree(&self, kind: SessionTreeKind) -> Result<Vec<SessionTreeNode>, String> {
        match kind {
            SessionTreeKind::MessageHistory => {
                self.session_tree_calls.fetch_add(1, Ordering::SeqCst);
                Ok(self.message_history_tree.clone())
            }
            SessionTreeKind::FileBrowser => {
                Err("scripted: file_browser tree not implemented".into())
            }
        }
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, String> {
        match kind {
            SessionTreeKind::MessageHistory => {
                self.travel_calls
                    .lock()
                    .expect("travel_calls")
                    .push(entry_id.to_string());
                if let Some(travel) = self.travel_overrides.get(entry_id) {
                    return Ok(travel.clone());
                }
                plan_message_history_travel(&self.session_messages, entry_id)
            }
            SessionTreeKind::FileBrowser => {
                Err("scripted: file_browser travel not implemented".into())
            }
        }
    }
}

/// One pump of host pending ops + optional full drain of the active agent stream.
/// Mirrors `run_host_loop` ordering via shared [`drain_pending`] (c485 / ath6).
pub async fn pump_host_driver<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn Driver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), String> {
    drain_pending(session, driver, agent_stream).await?;

    if let Some(bash) = session.take_bash() {
        run_pending_bash(session, driver, bash).await?;
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
pub fn harness_sample_message_history_tree() -> Vec<SessionTreeNode> {
    use crate::domain::session_types::{EntryBase, MessageEntry};

    fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionTreeNode {
        SessionTreeNode {
            entry: SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: parent.map(str::to_string),
                    timestamp: format!("t-{id}"),
                },
                message: crate::domain::session_types::fixture_message_json(role, text),
            }),
            children: Vec::new(),
            label: None,
        }
    }

    let mut u1 = msg("u1", None, "user", "hello");
    let mut a1 = msg("a1", Some("u1"), "assistant", "plan");
    let t1 = msg("t1", Some("a1"), "tool", "read");
    let mut a2 = msg("a2", Some("a1"), "assistant", "done");
    let u2 = msg("u2", Some("a2"), "user", "next");
    a2.children.push(u2);
    a1.children.extend([t1, a2]);
    u1.children.push(a1);
    vec![u1]
}

#[cfg(test)]
pub fn harness_sample_session_messages() -> Vec<SessionEntry> {
    harness_sample_message_history_tree()
        .into_iter()
        .flat_map(flatten_session_tree_entries)
        .collect()
}

fn flatten_session_tree_entries(node: SessionTreeNode) -> Vec<SessionEntry> {
    let mut out = vec![node.entry];
    for child in node.children {
        out.extend(flatten_session_tree_entries(child));
    }
    out
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
        driver.set_default_script(vec![
            XyEvent::AgentStart {
                session_id: "s".into(),
                model: "fake".into(),
            },
            XyEvent::TextDelta("hello".into()),
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        ]);
        driver.push_script(vec![
            XyEvent::AgentStart {
                session_id: "s".into(),
                model: "fake".into(),
            },
            XyEvent::TextDelta("hello".into()),
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        ]);
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
            driver.bash_calls(),
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
        assert_eq!(driver.bash_calls(), vec![("echo x".to_string(), true)]);
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

    #[tokio::test]
    async fn c665_busy_esc_shows_aborted_and_idles() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("first");
        session.step(HostEvent::Input(esc_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.abort_count(), 1);
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted")),
            "expected Aborted note: {:?}",
            session.ui_model().entries
        );
        assert!(
            session.ui_model().status.is_none(),
            "status must idle: {:?}",
            session.ui_model().status
        );
        assert!(!session.is_busy());
    }

    #[tokio::test]
    async fn c665_bang_esc_cancels_hanging_bash() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_hang_bash_until_abort(true);
        let mut stream = None;
        root.borrow_mut().set_editor_text("!sleep 99");
        session.step(HostEvent::Input(enter_event())).unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let bash = session.take_bash().expect("pending bang");
        session.begin_bash_exec();
        let bash_fut = driver.execute_bash(&bash.command, bash.exclude_from_context);
        tokio::pin!(bash_fut);
        tokio::select! {
            result = &mut bash_fut => {
                let r = result.expect("bash result");
                assert!(r.cancelled, "bash must be cancelled: {r:?}");
                session.push_bash_result(&bash.command, &r);
                session.end_bash_exec();
            }
            _ = async {
                tokio::time::sleep(Duration::from_millis(40)).await;
                session.step(HostEvent::Input(esc_event())).unwrap();
                assert!(session.take_abort(), "busy Esc must request abort");
                driver.abort();
                session.note_user_abort();
                std::future::pending::<()>().await
            } => {}
        }
        assert!(driver.abort_count() >= 1);
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted")),
            "Aborted note: {:?}",
            session.ui_model().entries
        );
        assert!(!session.bash_active());
        assert!(!session.is_busy());
    }

    #[tokio::test]
    async fn c665_bang_after_esc_abort_runs_again() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_hang_bash_until_abort(true);
        let mut stream = None;

        root.borrow_mut().set_editor_text("!sleep 99");
        session.step(HostEvent::Input(enter_event())).unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let bash = session.take_bash().expect("pending bang");
        session.begin_bash_exec();
        {
            let bash_fut = driver.execute_bash(&bash.command, bash.exclude_from_context);
            tokio::pin!(bash_fut);
            tokio::select! {
                result = &mut bash_fut => {
                    let r = result.expect("bash result");
                    assert!(r.cancelled);
                    session.end_bash_exec();
                }
                _ = async {
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    session.step(HostEvent::Input(esc_event())).unwrap();
                    assert!(session.take_abort());
                    driver.abort();
                    session.note_user_abort();
                    // Simulate Esc backlog that used to sticky-cancel the next bang.
                    session.step(HostEvent::Input(esc_event())).unwrap();
                    session.step(HostEvent::Input(esc_event())).unwrap();
                    std::future::pending::<()>().await
                } => {}
            }
        }

        driver.set_hang_bash_until_abort(false);
        root.borrow_mut().set_editor_text("!echo ok");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let calls = driver.bash_calls();
        assert!(
            calls.iter().any(|(c, _)| c == "echo ok"),
            "second bang must run after Esc abort: {calls:?}"
        );
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("ok"))),
            "second bang output missing: {:?}",
            session.ui_model().entries
        );
    }

    #[tokio::test]
    async fn h10_double_esc_fetches_live_tree() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_message_history_tree(harness_sample_message_history_tree());
        let mut stream = None;
        session.step(HostEvent::Input(esc_event())).unwrap();
        session.step(HostEvent::Input(esc_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.session_tree_calls(), 1);
        assert!(root.borrow().tree_open());
        let frame = root.borrow_mut().render(80);
        assert!(
            frame
                .iter()
                .any(|l| l.contains("user:") && l.contains("hello")),
            "live tree frame: {frame:?}"
        );
    }

    #[tokio::test]
    async fn h11_tree_enter_user_travel_prefills() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u1",
        );
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.travel_calls(), vec!["u1".to_string()]);
        assert_eq!(root.borrow().editor_text(), "hello");
        assert!(!root.borrow().tree_open());
    }

    #[test]
    fn bang_parse_helpers() {
        use crate::app::tui::commands::{BangParse, parse_bang_command};
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
