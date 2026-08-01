//! c485 synthetic vertical-slice harness — ScriptedDriver + host/driver pump.
//!
//! Only compiled in unit tests (`cfg(test)`). Stays inside `app/tui` and talks
//! to the core solely via [`crate::app::core::driver::XyDriver`] (layering seam).

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
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, ModelInfo, QueueStats,
    ReloadStepReport, RuntimeReloadReport, SessionListEntry, SessionStats, XyDriver, XyDriverError,
    XyEvent,
};
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};
use crate::protocol::types::ThinkingLevel;

use super::effects::{drain_pending, refresh_footer_tokens, run_interactive_bang};
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
    available_models: Vec<ModelInfo>,
    message_history_tree: Vec<SessionTreeNode>,
    session_messages: Vec<SessionEntry>,
    travel_overrides: HashMap<String, SessionTreeTravel>,
    session_tree_calls: AtomicUsize,
    travel_calls: Mutex<Vec<String>>,
    fork_calls: Mutex<Vec<(String, crate::protocol::session::ForkPosition)>>,
    switch_calls: Mutex<Vec<String>>,
    label_calls: Mutex<Vec<(String, Option<String>)>>,
    debug_scene_calls: Mutex<Vec<String>>,
    active_session_id: Mutex<String>,
    /// Scripted leaf for `/session-fork` (c700/c1005).
    leaf_entry_id: Mutex<Option<String>>,
    compact_calls: Mutex<Vec<Option<String>>>,
    export_html_calls: Mutex<Vec<String>>,
    export_jsonl_calls: Mutex<Vec<String>>,
    import_jsonl_calls: Mutex<Vec<String>>,
    session_list: Mutex<Vec<SessionListEntry>>,
    session_stats: Option<SessionStats>,
    list_sessions_calls: AtomicUsize,
    new_session_calls: AtomicUsize,
    session_name: Mutex<Option<String>>,
    set_session_name_calls: Mutex<Vec<String>>,
    set_session_name_for_calls: Mutex<Vec<(String, String)>>,
    delete_session_calls: Mutex<Vec<String>>,
    /// Optional fixed estimate for footer harness (c1035).
    estimate_override: Option<crate::protocol::types::ContextTokenEstimate>,
    reload_runtime_calls: AtomicUsize,
    persist_project_trust_calls: Mutex<Vec<crate::app::core::driver::ProjectTrustMode>>,
    copy_text_calls: Mutex<Vec<String>>,
    /// When set, next `copy_text_to_clipboard` returns this OSC 52 for host emit.
    copy_pending_osc52: Mutex<Option<String>>,
    dollar_skill_catalog: Mutex<Vec<(String, String)>>,
    /// Injectable loaded-resources header snapshot (c1135).
    loaded_resources: Mutex<LoadedResourcesSnapshot>,
    /// Count of [`XyDriver::loaded_resources_snapshot`] awaits (c1215 cache seam).
    loaded_resources_snapshot_calls: AtomicUsize,
    /// Current thinking level (c1150); mutable via set/cycle.
    thinking_level: ThinkingLevel,
    /// Support list for cycle (default STANDARD; tests may narrow e.g. `[Off, High]`).
    thinking_levels: Vec<ThinkingLevel>,
    /// Injectable clipboard image bytes for paste staging (c1155); `None` = no image.
    clipboard_image: Mutex<Option<(Vec<u8>, String)>>,
    /// Paths written by [`XyDriver::stage_clipboard_image`].
    staged_paste_paths: Mutex<Vec<String>>,
    /// Force `stage_clipboard_image` to Err.
    clipboard_image_error: Mutex<Option<String>>,
    /// Injectable clipboard text for Ctrl+V fallback (c1156).
    clipboard_text: Mutex<Option<String>>,
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
                timed_out: false,
                truncated: false,
                full_output_path: None,
            },
            steer_queued: 0,
            follow_up_queued: 0,
            model: ModelInfo {
                id: "fake".into(),
                display_name: "Fake".into(),
                thinking: false,
                thinking_levels: Vec::new(),
                context_window: 8_000,
            },
            available_models: vec![
                ModelInfo {
                    id: "fake".into(),
                    display_name: "Fake".into(),
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                },
                ModelInfo {
                    id: "ornith-fast".into(),
                    display_name: "Ornith Fast".into(),
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                },
                ModelInfo {
                    id: "ornith-think".into(),
                    display_name: "Ornith Think".into(),
                    thinking: true,
                    thinking_levels: Vec::new(),
                    context_window: 32_000,
                },
            ],
            message_history_tree: Vec::new(),
            session_messages: Vec::new(),
            travel_overrides: HashMap::new(),
            session_tree_calls: AtomicUsize::new(0),
            travel_calls: Mutex::new(Vec::new()),
            fork_calls: Mutex::new(Vec::new()),
            switch_calls: Mutex::new(Vec::new()),
            label_calls: Mutex::new(Vec::new()),
            debug_scene_calls: Mutex::new(Vec::new()),
            active_session_id: Mutex::new("scripted".into()),
            leaf_entry_id: Mutex::new(None),
            compact_calls: Mutex::new(Vec::new()),
            export_html_calls: Mutex::new(Vec::new()),
            export_jsonl_calls: Mutex::new(Vec::new()),
            import_jsonl_calls: Mutex::new(Vec::new()),
            session_list: Mutex::new(Vec::new()),
            session_stats: None,
            list_sessions_calls: AtomicUsize::new(0),
            new_session_calls: AtomicUsize::new(0),
            session_name: Mutex::new(None),
            set_session_name_calls: Mutex::new(Vec::new()),
            set_session_name_for_calls: Mutex::new(Vec::new()),
            delete_session_calls: Mutex::new(Vec::new()),
            estimate_override: None,
            reload_runtime_calls: AtomicUsize::new(0),
            persist_project_trust_calls: Mutex::new(Vec::new()),
            copy_text_calls: Mutex::new(Vec::new()),
            copy_pending_osc52: Mutex::new(None),
            dollar_skill_catalog: Mutex::new(Vec::new()),
            loaded_resources: Mutex::new(LoadedResourcesSnapshot::default()),
            loaded_resources_snapshot_calls: AtomicUsize::new(0),
            thinking_level: ThinkingLevel::Off,
            thinking_levels: ThinkingLevel::STANDARD.to_vec(),
            clipboard_image: Mutex::new(None),
            staged_paste_paths: Mutex::new(Vec::new()),
            clipboard_image_error: Mutex::new(None),
            clipboard_text: Mutex::new(None),
        }
    }

    /// Queue a fake clipboard image for the next [`XyDriver::stage_clipboard_image`] (c1155).
    pub fn set_clipboard_image(&self, bytes: Vec<u8>, mime: impl Into<String>) {
        *self.clipboard_image.lock().expect("clipboard_image") = Some((bytes, mime.into()));
    }

    /// Next stage call returns this error.
    pub fn set_clipboard_image_error(&self, err: impl Into<String>) {
        *self
            .clipboard_image_error
            .lock()
            .expect("clipboard_image_error") = Some(err.into());
    }

    /// Queue clipboard text for [`XyDriver::read_clipboard_text`] (c1156).
    pub fn set_clipboard_text(&self, text: impl Into<String>) {
        *self.clipboard_text.lock().expect("clipboard_text") = Some(text.into());
    }

    pub fn staged_paste_paths(&self) -> Vec<String> {
        self.staged_paste_paths
            .lock()
            .expect("staged_paste_paths")
            .clone()
    }

    /// Replace the thinking support list used by [`XyDriver::cycle_thinking_level`].
    pub fn set_thinking_levels(&mut self, levels: Vec<ThinkingLevel>) {
        self.thinking_levels = levels;
        if !self.thinking_levels.is_empty() && !self.thinking_levels.contains(&self.thinking_level)
        {
            self.thinking_level = self.thinking_levels[0];
        }
    }

    /// Queue a pending OSC 52 sequence for the next clipboard copy (TUI host path).
    pub fn set_next_copy_pending_osc52(&self, sequence: Option<String>) {
        *self.copy_pending_osc52.lock().expect("copy_pending_osc52") = sequence;
    }

    pub fn reload_runtime_calls(&self) -> usize {
        self.reload_runtime_calls.load(Ordering::SeqCst)
    }

    pub fn persist_project_trust_calls(&self) -> Vec<crate::app::core::driver::ProjectTrustMode> {
        self.persist_project_trust_calls
            .lock()
            .expect("persist_project_trust_calls")
            .clone()
    }

    pub fn copy_text_calls(&self) -> Vec<String> {
        self.copy_text_calls
            .lock()
            .expect("copy_text_calls")
            .clone()
    }

    pub fn set_dollar_skill_catalog_for_driver(&self, catalog: Vec<(String, String)>) {
        *self.dollar_skill_catalog.lock().expect("catalog") = catalog;
    }

    /// Inject loaded-resources snapshot for header harness (c1135).
    pub fn set_loaded_resources_for_driver(&self, snap: LoadedResourcesSnapshot) {
        *self.loaded_resources.lock().expect("loaded_resources") = snap;
    }

    /// How many times [`XyDriver::loaded_resources_snapshot`] was awaited (c1215).
    pub fn loaded_resources_snapshot_calls(&self) -> usize {
        self.loaded_resources_snapshot_calls.load(Ordering::SeqCst)
    }

    pub fn new_session_calls(&self) -> usize {
        self.new_session_calls.load(Ordering::SeqCst)
    }

    pub fn set_session_name_calls(&self) -> Vec<String> {
        self.set_session_name_calls
            .lock()
            .expect("set_session_name_calls")
            .clone()
    }

    pub fn set_session_name_for_calls(&self) -> Vec<(String, String)> {
        self.set_session_name_for_calls
            .lock()
            .expect("set_session_name_for_calls")
            .clone()
    }

    pub fn delete_session_calls(&self) -> Vec<String> {
        self.delete_session_calls
            .lock()
            .expect("delete_session_calls")
            .clone()
    }

    pub fn set_message_history_tree(&mut self, tree: Vec<SessionTreeNode>) {
        self.message_history_tree = tree;
    }

    pub fn set_session_messages(&mut self, entries: Vec<SessionEntry>) {
        self.session_messages = entries;
    }

    /// Fixed [`XyDriver::estimate_context_tokens`] result for footer harness (c1035).
    pub fn set_estimate_override(
        &mut self,
        estimate: Option<crate::protocol::types::ContextTokenEstimate>,
    ) {
        self.estimate_override = estimate;
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

    pub fn fork_calls(&self) -> Vec<(String, crate::protocol::session::ForkPosition)> {
        self.fork_calls.lock().expect("fork_calls").clone()
    }

    pub fn label_calls(&self) -> Vec<(String, Option<String>)> {
        self.label_calls.lock().expect("label_calls").clone()
    }

    pub fn set_leaf_entry_id(&self, id: Option<String>) {
        *self.leaf_entry_id.lock().expect("leaf") = id;
    }

    pub fn debug_scene_calls(&self) -> Vec<String> {
        self.debug_scene_calls
            .lock()
            .expect("debug_scene_calls")
            .clone()
    }

    pub fn switch_calls(&self) -> Vec<String> {
        self.switch_calls.lock().expect("switch_calls").clone()
    }

    pub fn compact_calls(&self) -> usize {
        self.compact_calls.lock().expect("compact_calls").len()
    }

    pub fn compact_instructions(&self) -> Vec<Option<String>> {
        self.compact_calls.lock().expect("compact_calls").clone()
    }

    pub fn export_html_calls(&self) -> Vec<String> {
        self.export_html_calls
            .lock()
            .expect("export_html_calls")
            .clone()
    }

    pub fn export_jsonl_calls(&self) -> Vec<String> {
        self.export_jsonl_calls
            .lock()
            .expect("export_jsonl_calls")
            .clone()
    }

    pub fn import_jsonl_calls(&self) -> Vec<String> {
        self.import_jsonl_calls
            .lock()
            .expect("import_jsonl_calls")
            .clone()
    }

    pub fn set_session_list(&mut self, entries: Vec<SessionListEntry>) {
        *self.session_list.lock().expect("session_list") = entries;
    }

    pub fn set_session_stats(&mut self, stats: SessionStats) {
        self.session_stats = Some(stats);
    }

    pub fn list_sessions_calls(&self) -> usize {
        self.list_sessions_calls.load(Ordering::SeqCst)
    }

    pub fn set_current_model(&mut self, model: ModelInfo) {
        self.model = model;
    }

    pub fn set_available_models(&mut self, models: Vec<ModelInfo>) {
        self.available_models = models;
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
impl XyDriver for ScriptedDriver {
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
        if self.available_models.is_empty() {
            vec![self.model.clone()]
        } else {
            self.available_models.clone()
        }
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        self.model = ModelInfo {
            id: model_id.into(),
            display_name: model_id.into(),
            thinking: false,
            thinking_levels: Vec::new(),
            context_window: 8_000,
        };
        Ok(self.model.clone())
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        Ok(self.model.clone())
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyDriverError> {
        if !self.thinking_levels.is_empty() && !self.thinking_levels.contains(&level) {
            return Err(format!(
                "thinking level `{}` is not supported by the current model",
                level.as_str()
            )
            .into());
        }
        self.thinking_level = level;
        Ok(())
    }

    fn thinking_level(&self) -> ThinkingLevel {
        self.thinking_level
    }

    fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyDriverError> {
        if self.thinking_levels.is_empty() {
            return Err("current model has no thinking levels".into());
        }
        let idx = self
            .thinking_levels
            .iter()
            .position(|l| *l == self.thinking_level)
            .unwrap_or(0);
        let next = self.thinking_levels[(idx + 1) % self.thinking_levels.len()];
        self.thinking_level = next;
        Ok(next)
    }

    fn session_id(&self) -> Option<String> {
        Some(self.active_session_id.lock().expect("sid").clone())
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        // Fresh run: do not inherit a prior abort latch (pi: new AbortController each bang).
        self.aborted.store(false, Ordering::SeqCst);
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
                timed_out: false,
                truncated: false,
                full_output_path: None,
            });
        }
        let result = self
            .bash_results
            .lock()
            .expect("bash_results")
            .pop_front()
            .unwrap_or_else(|| self.default_bash.clone());
        if let Some(tx) = chunk_tx {
            // Stream output in small frames so harness can assert pending tint mid-flight.
            let bytes = result.output.as_bytes();
            if !bytes.is_empty() {
                let mid = bytes.len().saturating_add(1) / 2;
                let _ = tx.send(bytes[..mid].to_vec()).await;
                if mid < bytes.len() {
                    let _ = tx.send(bytes[mid..].to_vec()).await;
                }
            }
        }
        Ok(result)
    }

    async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        self.compact_calls
            .lock()
            .expect("compact_calls")
            .push(instructions);
        Ok(false)
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let s = path.display().to_string();
        self.export_html_calls
            .lock()
            .expect("export_html_calls")
            .push(s.clone());
        Ok(s)
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let s = path.display().to_string();
        self.export_jsonl_calls
            .lock()
            .expect("export_jsonl_calls")
            .push(s.clone());
        Ok(s)
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.import_jsonl_calls
            .lock()
            .expect("import_jsonl_calls")
            .push(path.display().to_string());
        Ok("imported".into())
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        self.fork_calls
            .lock()
            .expect("fork_calls")
            .push((entry_id.to_string(), position));
        Ok("forked-child".into())
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        self.switch_calls
            .lock()
            .expect("switch_calls")
            .push(session_id.to_string());
        *self.active_session_id.lock().expect("sid") = session_id.to_string();
        Ok(session_id.into())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        Ok(self.session_messages.clone())
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        self.session_stats
            .clone()
            .ok_or_else(|| "scripted: no stats".into())
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::types::ContextTokenEstimate, XyDriverError> {
        if let Some(est) = self.estimate_override.clone() {
            return Ok(est);
        }
        Ok(crate::app::core::driver::estimate_from_session_entries(
            &self.session_messages,
            self.current_model().map(|m| m.id),
            None,
        ))
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        Vec::new()
    }

    fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.steer_calls.push(message.to_string());
        self.steer_queued += 1;
        Ok(())
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.follow_up_calls.push(message.to_string());
        self.follow_up_queued += 1;
        Ok(())
    }

    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
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

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
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
    ) -> Result<SessionTreeTravel, XyDriverError> {
        match kind {
            SessionTreeKind::MessageHistory => {
                self.travel_calls
                    .lock()
                    .expect("travel_calls")
                    .push(entry_id.to_string());
                if let Some(travel) = self.travel_overrides.get(entry_id) {
                    return Ok(travel.clone());
                }
                plan_message_history_travel(&self.session_messages, entry_id).map_err(Into::into)
            }
            SessionTreeKind::FileBrowser => {
                Err("scripted: file_browser travel not implemented".into())
            }
        }
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        let cleaned = label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        self.label_calls
            .lock()
            .expect("label_calls")
            .push((target_id.to_string(), cleaned));
        Ok(())
    }

    fn leaf_entry_id(&self) -> Option<String> {
        self.leaf_entry_id.lock().expect("leaf").clone()
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        self.debug_scene_calls
            .lock()
            .expect("debug_scene_calls")
            .push(scene.to_string());
        let canonical = crate::app::debug_fixtures::resolve_scene_id(scene).unwrap_or(scene);
        let sid = format!("debug-{canonical}-scripted");
        *self.active_session_id.lock().expect("active_session_id") = sid.clone();
        let entries = self.session_messages.clone();
        Ok(DebugSceneLoad {
            session_id: sid,
            entries,
            note: format!("debug scene `{canonical}` (scripted)"),
            model: Some(self.model.clone()),
        })
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        self.list_sessions_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.session_list.lock().expect("session_list").clone())
    }

    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        let _ = session_id;
        Ok(self.session_messages.clone())
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        self.new_session_calls.fetch_add(1, Ordering::SeqCst);
        let sid = format!("new-{}", self.new_session_calls());
        *self.active_session_id.lock().expect("sid") = sid.clone();
        self.session_messages.clear();
        *self.leaf_entry_id.lock().expect("leaf") = None;
        *self.session_name.lock().expect("session_name") = None;
        Ok(sid)
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        Ok(self.session_name.lock().expect("session_name").clone())
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let stored = crate::protocol::ports::sanitize_session_display_name(name);
        self.set_session_name_calls
            .lock()
            .expect("set_session_name_calls")
            .push(name.to_string());
        *self.session_name.lock().expect("session_name") = Some(stored.clone());
        Ok(stored)
    }

    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        let stored = crate::protocol::ports::sanitize_session_display_name(name);
        self.set_session_name_for_calls
            .lock()
            .expect("set_session_name_for_calls")
            .push((session_id.to_string(), name.to_string()));
        if let Some(entry) = self
            .session_list
            .lock()
            .expect("session_list")
            .iter_mut()
            .find(|e| e.id == session_id)
        {
            entry.name = Some(stored.clone());
        }
        Ok(stored)
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.delete_session_calls
            .lock()
            .expect("delete_session_calls")
            .push(session_id.to_string());
        self.session_list
            .lock()
            .expect("session_list")
            .retain(|e| e.id != session_id);
        Ok(())
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.dollar_skill_catalog.lock().expect("catalog").clone()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        self.loaded_resources_snapshot_calls
            .fetch_add(1, Ordering::SeqCst);
        self.loaded_resources
            .lock()
            .expect("loaded_resources")
            .clone()
    }

    async fn reload_runtime(&mut self) -> Result<RuntimeReloadReport, XyDriverError> {
        self.reload_runtime_calls.fetch_add(1, Ordering::SeqCst);
        // Mirror catalog → skill_names so /reload harness sees header refresh (c1135).
        let names: Vec<String> = self
            .dollar_skill_catalog
            .lock()
            .expect("catalog")
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        self.loaded_resources
            .lock()
            .expect("loaded_resources")
            .skill_names = names;
        Ok(RuntimeReloadReport {
            steps: vec![ReloadStepReport {
                step: "skills",
                ok: true,
                message: "scripted noop".into(),
            }],
        })
    }

    fn persist_project_trust(
        &mut self,
        mode: crate::app::core::driver::ProjectTrustMode,
    ) -> Result<crate::app::core::driver::ProjectTrustPersistReport, XyDriverError> {
        use crate::app::core::driver::{ProjectTrustMode, ProjectTrustPersistReport};
        self.persist_project_trust_calls
            .lock()
            .expect("persist_project_trust_calls")
            .push(mode);
        let (trusted, path) = match mode {
            ProjectTrustMode::TrustCwd => (true, "/scripted/cwd"),
            ProjectTrustMode::TrustParent => (true, "/scripted"),
            ProjectTrustMode::Deny => (false, "/scripted/cwd"),
        };
        Ok(ProjectTrustPersistReport {
            trusted,
            saved_path: Some(path.into()),
            message: format!(
                "Project {} at {path}. {}",
                if trusted { "trusted" } else { "denied" },
                ProjectTrustPersistReport::RELOAD_HINT
            ),
        })
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<crate::app::core::driver::ClipboardCopyOutcome, XyDriverError> {
        self.copy_text_calls
            .lock()
            .expect("copy_text_calls")
            .push(text.to_string());
        let pending_osc52 = self
            .copy_pending_osc52
            .lock()
            .expect("copy_pending_osc52")
            .take();
        Ok(crate::app::core::driver::ClipboardCopyOutcome { pending_osc52 })
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        if let Some(err) = self
            .clipboard_image_error
            .lock()
            .expect("clipboard_image_error")
            .take()
        {
            return Err(err.into());
        }
        let Some((bytes, mime)) = self.clipboard_image.lock().expect("clipboard_image").take()
        else {
            return Ok(None);
        };
        let ext = match mime.as_str() {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "png",
        };
        let path =
            std::env::temp_dir().join(format!("xylitol-paste-{}.{}", uuid::Uuid::new_v4(), ext));
        std::fs::write(&path, &bytes).map_err(|e| format!("write paste image failed: {e}"))?;
        self.staged_paste_paths
            .lock()
            .expect("staged_paste_paths")
            .push(path.display().to_string());
        Ok(Some(path))
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        Ok(self.clipboard_text.lock().expect("clipboard_text").take())
    }
}

/// One pump of host pending ops + optional full drain of the active agent stream.
/// Mirrors `run_host_loop` ordering via shared [`drain_pending`] (c485 / ath6).
pub async fn pump_host_driver<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    agent_stream: &mut Option<EventStream>,
) -> Result<(), XyDriverError> {
    drain_pending(session, driver, agent_stream).await?;

    if let Some(bash) = session.take_bash() {
        // No keyboard: pending stream (not empty — empty would EOF-quit the bang loop).
        let input = futures::stream::pending::<Result<HostEvent, XyDriverError>>();
        run_interactive_bang(session, driver, bash, agent_stream, input).await?;
    }

    if let Some(stream) = agent_stream.as_mut() {
        while let Some(xy) = stream.next().await {
            session.step(HostEvent::Xy(Box::new(xy)))?;
        }
        *agent_stream = None;
        session.on_run_stream_closed();
        refresh_footer_tokens(session, driver).await;
        let _ = session.render_now();
    }

    if session.should_quit() {
        session.tui.finish_inline();
    }
    Ok(())
}

#[cfg(test)]
pub fn harness_sample_message_history_tree() -> Vec<SessionTreeNode> {
    use crate::protocol::session::{EntryBase, MessageEntry};

    fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionTreeNode {
        SessionTreeNode {
            entry: SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: parent.map(str::to_string),
                    timestamp: format!("t-{id}"),
                },
                message: crate::protocol::session::fixture_message_json(role, text),
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
    use futures::Stream;
    use xylitol_tui::{Component, InputEvent, Terminal};

    /// HostEvent stream for hanging-bang Esc: delay → Esc(+backlog) → park (no EOF).
    fn bang_esc_input_stream(
        backlog_after_abort: usize,
    ) -> impl Stream<Item = Result<HostEvent, XyDriverError>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            let _ = tx.send(Ok(HostEvent::Input(esc_event())));
            for _ in 0..backlog_after_abort {
                let _ = tx.send(Ok(HostEvent::Input(esc_event())));
            }
            std::future::pending::<()>().await;
        });
        futures::stream::unfold(
            rx,
            |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
        )
    }

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

    fn down_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn up_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn tab_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn shift_tab_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::SHIFT,
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

    fn ctrl_key_event(ch: char) -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctrl_shift_key_event(ch: char) -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctrl_left_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn ctrl_right_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn bare_left_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn shift_f_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn shift_l_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn shift_t_event() -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char('t'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn open_sample_tree(
        session: &mut HostSession<TestTerminal>,
    ) -> std::rc::Rc<std::cell::RefCell<crate::app::tui::layout::UiRoot>> {
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().open_session_tree_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            None,
        );
        root
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
            frame.iter().any(|l| l.contains("Bash")),
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
                .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("[steer]")))
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
    async fn h9_model_slash_opens_picker() {
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
            root.borrow().models_open(),
            "bare /model must open models slot"
        );
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("Ornith Think")),
            "expected models in frame: {frame:?}"
        );
    }

    fn char_event(ch: char) -> InputEvent {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        InputEvent::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[tokio::test]
    async fn c630_model_filter_select_updates_footer() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_current_model(ModelInfo {
            id: "fake".into(),
            display_name: "Fake".into(),
            thinking: false,
            thinking_levels: Vec::new(),
            context_window: 8_000,
        });
        let mut stream = None;
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().models_open());
        for ch in "think".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(!root.borrow().models_open());
        let frame = root.borrow_mut().render(80);
        assert!(
            frame
                .iter()
                .any(|l| l.contains("ornith-think") || l.contains("Ornith Think")),
            "footer should show selected model: {frame:?}"
        );
    }

    #[tokio::test]
    async fn c630_model_esc_keeps_model() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_current_model(ModelInfo {
            id: "fake".into(),
            display_name: "Fake".into(),
            thinking: false,
            thinking_levels: Vec::new(),
            context_window: 8_000,
        });
        let mut stream = None;
        root.borrow_mut().set_layout_meta(".", "Fake");
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().models_open());
        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(!root.borrow().models_open());
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("Fake")),
            "Esc must not change footer model: {frame:?}"
        );
        assert_eq!(driver.model.id, "fake");
    }

    #[tokio::test]
    async fn c630_model_id_direct_set() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().set_editor_text("/model ornith-think");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(!root.borrow().models_open());
        assert_eq!(driver.model.id, "ornith-think");
    }

    #[tokio::test]
    async fn c630_bare_model_not_cycle() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        assert_eq!(
            session.take_slash(),
            Some(crate::app::tui::commands::PendingSlash::OpenModels)
        );
    }

    #[tokio::test]
    async fn c1130_dollar_skill_tab_applies_name() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        session.set_dollar_skill_catalog(vec![
            ("demo".into(), "Demo skill".into()),
            ("other".into(), String::new()),
        ]);
        for ch in "use $de".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("demo")),
            "expected $skill popup; got: {frame:?}"
        );
        session.step(HostEvent::Input(tab_event())).unwrap();
        let text = root.borrow().editor_text();
        assert!(
            text.contains("$demo"),
            "Tab must apply $skill name; got {text:?}"
        );
    }

    #[tokio::test]
    async fn c999_model_arg_tab_applies_id() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        session.set_model_arg_catalog_from_models(&driver.available_models());
        for ch in "/model orn".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        let frame = root.borrow_mut().render(80);
        assert!(
            frame
                .iter()
                .any(|l| l.contains("ornith-fast") || l.contains("ornith-think")),
            "expected model-id popup; got: {frame:?}"
        );
        session.step(HostEvent::Input(tab_event())).unwrap();
        let text = root.borrow().editor_text();
        assert!(
            text.starts_with("/model ornith-"),
            "Tab must apply model id; got {text:?}"
        );
        assert_eq!(
            crate::app::tui::commands::parse_slash_command(&text),
            Some(crate::app::tui::commands::PendingSlash::SetModel(
                text.trim_start_matches("/model ").trim().to_string()
            ))
        );
    }

    #[tokio::test]
    async fn c1105_trust_arg_space_shows_self_parent_deny() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        for ch in "/trust ".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("self"))
                && frame.iter().any(|l| l.contains("parent"))
                && frame.iter().any(|l| l.contains("deny")),
            "expected /trust arg popup with self|parent|deny; got: {frame:?}"
        );
        session.step(HostEvent::Input(tab_event())).unwrap();
        let text = root.borrow().editor_text();
        let mode = text.trim_start_matches("/trust ").trim();
        assert_eq!(mode, "self", "default Tab must apply self; got {text:?}");
        assert_eq!(
            crate::app::tui::commands::parse_slash_command(text.trim()),
            Some(crate::app::tui::commands::PendingSlash::Trust {
                mode: crate::app::core::driver::ProjectTrustMode::TrustCwd
            })
        );
    }

    #[tokio::test]
    async fn c999_model_arg_esc_keeps_model() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        session.set_model_arg_catalog_from_models(&driver.available_models());
        root.borrow_mut().set_layout_meta(".", "Fake");
        for ch in "/model ".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(
            !root.borrow().models_open(),
            "Esc on arg popup must not open models slot"
        );
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("Fake")),
            "footer model must stay Fake after Esc dismiss: {frame:?}"
        );
    }

    #[tokio::test]
    async fn c1125_at_path_popup_and_tab_insert() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("hello.rs"), b"fn main() {}\n").expect("write");
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_at_path_base(dir.path());
        for ch in "@hel".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("hello.rs")),
            "expected @ path popup with hello.rs; got: {frame:?}"
        );
        session.step(HostEvent::Input(tab_event())).unwrap();
        let text = root.borrow().editor_text();
        assert!(
            text.contains("hello.rs") && text.contains('@'),
            "Tab must insert @path reference; got {text:?}"
        );
    }

    #[tokio::test]
    async fn c1125_at_path_esc_keeps_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("hello.rs"), b"").expect("write");
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_at_path_base(dir.path());
        for ch in "@hel".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        session.step(HostEvent::Input(esc_event())).unwrap();
        let text = root.borrow().editor_text();
        assert_eq!(text, "@hel", "Esc closes popup without rewriting prefix");
    }

    // ── c492 bang-bash (B1–B7) ─────────────────────────────────────

    #[test]
    fn b1_b2_bang_border_toggles() {
        let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        // Seed a non-off thinking level so restore is distinguishable from muted.
        root.borrow_mut().set_thinking_level_ui(ThinkingLevel::Low);
        root.borrow_mut().set_editor_text("!ls");
        assert!(root.borrow().bash_mode(), "B1: !ls enables bash border");
        let bash = root.borrow_mut().editor_render_for_test(40).join("\n");
        let success = xylitol_tui::Palette::dark().success;
        assert!(
            bash.contains(&format!("38;2;{};{};{}", success.r, success.g, success.b)),
            "bash should use success border; got:\n{bash}"
        );
        root.borrow_mut().set_editor_text("hello");
        assert!(
            !root.borrow().bash_mode(),
            "B2: clear restores thinking border"
        );
        let restored = root.borrow_mut().editor_render_for_test(40).join("\n");
        // pi dark thinkingLow #5f87af
        assert!(
            restored.contains("38;2;95;135;175"),
            "clear must restore low thinking border, not only muted; got:\n{restored}"
        );
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
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Bash { command, .. } if command.contains("echo hi")
            )),
            "summary missing: {:?}",
            session.ui_model().entries
        );
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Bash { output, .. } if output.contains("hello-out")
            )),
            "output missing: {:?}",
            session.ui_model().entries
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
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Bash {
                    status: crate::app::tui::bridge::BashBlockStatus::Error,
                    output,
                    ..
                } if output.contains("exit 1")
            )),
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
                .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("Ctrl+G"))),
            "scroll notice: {:?}",
            session.ui_model().entries
        );
    }

    #[test]
    fn c650_ctrl_g_missing_editor_is_error() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("keep-me");
        session.set_force_real_external_editor(true);
        session.set_external_editor_cmd_override(Some(Err(
            "set $VISUAL or $EDITOR to use external editor (Ctrl+G)".into(),
        )));
        session.step(HostEvent::Input(ctrl_g_event())).unwrap();
        assert_eq!(root.borrow().editor_text(), "keep-me");
        assert!(
            !root.borrow().editor_text().contains("$EDITOR stub"),
            "must not stub-mark on missing config"
        );
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Error { text } if text.contains("$VISUAL") || text.contains("$EDITOR")
            )),
            "expected UiEntry::Error: {:?}",
            session.ui_model().entries
        );
    }

    #[test]
    fn c650_ctrl_g_spawn_fail_is_error() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().set_editor_text("keep-me");
        session.set_force_real_external_editor(true);
        session
            .set_external_editor_cmd_override(Some(Ok("/nonexistent/xylitol-c650-editor".into())));
        session.step(HostEvent::Input(ctrl_g_event())).unwrap();
        assert_eq!(root.borrow().editor_text(), "keep-me");
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Error { text } if text.contains("external editor failed")
            )),
            "expected spawn Error: {:?}",
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
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Operation aborted" || text == "Aborted"
            )),
            "expected abort note: {:?}",
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
    async fn c720_esc_suppresses_xy_before_drain() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("first");
        // Mid-stream draft then Esc — suppress must arm before drain.
        session
            .step(HostEvent::Xy(Box::new(XyEvent::TextDelta("draft".into()))))
            .unwrap();
        session.step(HostEvent::Input(esc_event())).unwrap();
        session
            .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
                "SHOULD_NOT_APPEAR".into(),
            ))))
            .unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.abort_count(), 1, "XyDriver::abort must still run");
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Assistant { text } if text.contains("draft")
            )),
            "c1595: partial must remain after abort: {:?}",
            session.ui_model().entries
        );
        assert!(
            !session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Assistant { text } if text.contains("SHOULD_NOT_APPEAR")
            )),
            "late delta before drain must not appear: {:?}",
            session.ui_model().entries
        );
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Operation aborted" || text == "Aborted"
            )),
            "expected abort note after drain: {:?}",
            session.ui_model().entries
        );
    }

    #[tokio::test]
    async fn c670_abort_drops_late_deltas() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.push_script(vec![
            XyEvent::ThinkingDelta("plan…".into()),
            XyEvent::TextDelta("SHOULD_NOT_APPEAR".into()),
            XyEvent::AgentEnd {
                messages: Vec::new(),
            },
        ]);
        let mut stream = None;
        root.borrow_mut().set_editor_text("go");
        session.step(HostEvent::Input(enter_event())).unwrap();
        // Start run but do not drain stream yet.
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(stream.is_some(), "run must open EventStream");
        // Mid-stream Esc abort.
        session.step(HostEvent::Input(esc_event())).unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.abort_count(), 1);
        // Pump remaining scripted events — must be dropped (c670).
        if let Some(s) = stream.as_mut() {
            while let Some(xy) = s.next().await {
                session.step(HostEvent::Xy(Box::new(xy))).unwrap();
            }
            stream = None;
            session.on_run_stream_closed();
        }
        assert!(
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Operation aborted" || text == "Aborted"
            )),
            "expected abort note: {:?}",
            session.ui_model().entries
        );
        assert!(
            !session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Assistant { text } if text.contains("SHOULD_NOT_APPEAR")
            )),
            "late TextDelta must not become assistant: {:?}",
            session.ui_model().entries
        );
        assert!(
            session.ui_model().streaming_assistant.is_empty()
                && session.ui_model().streaming_thinking.is_empty(),
            "streaming buffers must stay empty"
        );
        assert!(!session.is_busy());
        // Next turn still works.
        root.borrow_mut().set_editor_text("again");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.runs.iter().any(|r| r == "again"),
            "second run after abort: {:?}",
            driver.runs
        );
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
        let input = bang_esc_input_stream(0);
        run_interactive_bang(&mut session, &mut driver, bash, &mut stream, input)
            .await
            .unwrap();
        assert!(driver.abort_count() >= 1);
        let entries = &session.ui_model().entries;
        assert!(
            entries.iter().any(|e| matches!(
                e,
                UiEntry::Bash {
                    command,
                    status: crate::app::tui::bridge::BashBlockStatus::Cancelled,
                    output,
                    ..
                } if command == "sleep 99" && output.contains("(cancelled)")
            )),
            "bang Esc must cancel Bash block: {entries:?}"
        );
        assert!(
            !entries.iter().any(|e| matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Aborted" || text == "Operation aborted"
            )),
            "bang Esc must not use agent abort footer: {entries:?}"
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
        let input = bang_esc_input_stream(2);
        run_interactive_bang(&mut session, &mut driver, bash, &mut stream, input)
            .await
            .unwrap();

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
            session.ui_model().entries.iter().any(|e| matches!(
                e,
                UiEntry::Bash { command, output, .. }
                    if command == "echo ok" && output.contains("ok")
            )),
            "second bang output missing: {:?}",
            session.ui_model().entries
        );
    }

    /// After abort + Esc backlog, a second hanging bang must still be Esc-abortable
    /// (regression: `suppress_busy_esc` used to eat busy Esc forever).
    #[tokio::test]
    async fn c665_second_bang_esc_still_aborts() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_hang_bash_until_abort(true);
        let mut stream = None;

        // Bang 1: hang + Esc abort + Esc backlog while idle.
        root.borrow_mut().set_editor_text("!sleep 1");
        session.step(HostEvent::Input(enter_event())).unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let bash1 = session.take_bash().expect("bang1");
        let input1 = bang_esc_input_stream(8);
        run_interactive_bang(&mut session, &mut driver, bash1, &mut stream, input1)
            .await
            .unwrap();
        assert!(!session.is_busy());

        // Bang 2: must still accept Esc abort (pi: fresh AbortController each run).
        root.borrow_mut().set_editor_text("!sleep 2");
        session.step(HostEvent::Input(enter_event())).unwrap();
        drain_pending(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let bash2 = session.take_bash().expect("bang2");
        let abort_before = driver.abort_count();
        let input2 = bang_esc_input_stream(0);
        run_interactive_bang(&mut session, &mut driver, bash2, &mut stream, input2)
            .await
            .unwrap();
        assert!(driver.abort_count() > abort_before);
        let entries = &session.ui_model().entries;
        let cancelled_count = entries
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    UiEntry::Bash {
                        status: crate::app::tui::bridge::BashBlockStatus::Cancelled,
                        ..
                    }
                )
            })
            .count();
        assert!(
            cancelled_count >= 2,
            "each bang Esc abort should cancel a Bash block: {entries:?}"
        );
        assert!(
            !entries.iter().any(|e| matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Aborted" || text == "Operation aborted"
            )),
            "bang Esc must not emit agent abort footer: {entries:?}"
        );
    }

    #[test]
    fn c669_append_bash_keeps_pending_tint() {
        use crate::app::tui::bridge::{BashBlockStatus, UiEntry, UiModel};
        let mut model = UiModel::new();
        model.begin_bash_block("printf hi", false);
        model.append_bash_output(b"hel");
        model.append_bash_output(b"lo\n");
        let Some(UiEntry::Bash { status, output, .. }) = model.entries.last() else {
            panic!("expected Bash entry: {:?}", model.entries);
        };
        assert_eq!(*status, BashBlockStatus::Pending);
        assert!(output.contains("hello"), "output={output:?}");
        model.finish_bash_block(BashBlockStatus::Success, "hello\n(exit 0)".into());
        let Some(UiEntry::Bash { status, .. }) = model.entries.last() else {
            panic!("expected Bash entry");
        };
        assert_eq!(*status, BashBlockStatus::Success);
    }

    #[tokio::test]
    async fn c669_scripted_streams_chunks_before_done() {
        let mut driver = ScriptedDriver::new();
        driver.push_bash_result(XyBashResult {
            output: "abcdef".into(),
            exit_code: Some(0),
            cancelled: false,
            timed_out: false,
            truncated: false,
            full_output_path: None,
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
        let result = driver
            .execute_bash("echo", false, Some(tx))
            .await
            .expect("bash");
        assert_eq!(result.output, "abcdef");
        let mut collected = Vec::new();
        while let Ok(c) = rx.try_recv() {
            collected.extend_from_slice(&c);
        }
        assert_eq!(String::from_utf8_lossy(&collected), "abcdef");
    }

    #[tokio::test]
    async fn c669_second_bang_hard_reject_while_bash_active() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        session.begin_bash_exec("sleep 99", false);
        assert!(session.bash_active());
        root.borrow_mut()
            .set_editor_text(String::from("!echo second"));
        session.step(HostEvent::Input(enter_event())).unwrap();
        assert_eq!(
            driver.bash_calls(),
            Vec::<(String, bool)>::new(),
            "second bang must not call execute_bash"
        );
        assert_eq!(root.borrow().editor_text(), "!echo second");
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("rejected"))),
            "expected hard-reject note: {:?}",
            session.ui_model().entries
        );
    }

    #[tokio::test]
    async fn ati32_agent_busy_bang_prefix_rejected_not_steer() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("!ls");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.steer_calls.is_empty(),
            "bang must not steer: {:?}",
            driver.steer_calls
        );
        assert_eq!(root.borrow().editor_text(), "!ls");
        assert!(
            session
                .ui_model()
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("rejected"))),
            "expected reject note: {:?}",
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

    #[tokio::test]
    async fn h12_tree_filter_no_tools_toggle() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        session.step(HostEvent::Input(ctrl_key_event('t'))).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            !panel.contains("tool:"),
            "no-tools must hide tool rows; got:\n{panel}"
        );
        assert!(
            panel.contains("[no-tools]"),
            "status must show [no-tools]; got:\n{panel}"
        );
        session.step(HostEvent::Input(ctrl_key_event('t'))).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("tool:"),
            "toggle back to default must show tools; got:\n{panel}"
        );
        assert!(
            !panel.contains("[no-tools]"),
            "default must not show [no-tools]; got:\n{panel}"
        );
        assert_eq!(
            root.borrow().tree_filter_for_test(),
            crate::app::tui::layout::FilterMode::Default
        );
    }

    #[tokio::test]
    async fn h13_tree_filter_user_only() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        session.step(HostEvent::Input(ctrl_key_event('u'))).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("user:") && !panel.contains("assistant:") && !panel.contains("tool:"),
            "user-only must hide non-user rows; got:\n{panel}"
        );
        assert!(
            panel.contains("[user]"),
            "status must show [user]; got:\n{panel}"
        );
    }

    #[tokio::test]
    async fn h14_tree_filter_labeled_only() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        session.step(HostEvent::Input(ctrl_key_event('l'))).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("keep") && !panel.contains("alternate"),
            "labeled-only must show annotated node only; got:\n{panel}"
        );
        assert!(
            panel.contains("[labeled]"),
            "status must show [labeled]; got:\n{panel}"
        );
    }

    #[tokio::test]
    async fn h15_tree_filter_cycle_ctrl_o() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        assert_eq!(
            root.borrow().tree_filter_for_test(),
            crate::app::tui::layout::FilterMode::Default
        );
        session.step(HostEvent::Input(ctrl_key_event('o'))).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("[no-tools]"),
            "ctrl+o from default must cycle to no-tools; got:\n{panel}"
        );
        assert_eq!(
            root.borrow().tree_filter_for_test(),
            crate::app::tui::layout::FilterMode::NoTools
        );
    }

    #[tokio::test]
    async fn h16_tree_search_esc_then_close() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        session.step(HostEvent::Input(char_event('r'))).unwrap();
        assert!(
            !root.borrow().tree_search_query_for_test().is_empty(),
            "typing must set tree search query"
        );
        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(
            root.borrow().tree_search_query_for_test().is_empty(),
            "first Esc must clear search"
        );
        assert!(
            root.borrow().tree_open(),
            "tree must stay open after search clear"
        );
        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(!root.borrow().tree_open(), "second Esc must close tree");
    }

    #[tokio::test]
    async fn h17_tree_ctrl_left_folds_hides_descendants() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u1",
        );
        let before = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            before.contains("tool:") || before.contains("read"),
            "precondition: descendant tool row visible:\n{before}"
        );
        session.step(HostEvent::Input(ctrl_left_event())).unwrap();
        assert!(
            root.borrow().tree_is_folded_for_test("u1"),
            "Ctrl+Left must fold selected u1"
        );
        let folded = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            folded.contains('⊞') || folded.contains("⊞"),
            "expected ⊞ fold marker; got:\n{folded}"
        );
        assert!(
            !(folded.contains("tool:") && folded.contains("read")),
            "folded u1 must hide tool descendant; got:\n{folded}"
        );
        session.step(HostEvent::Input(ctrl_right_event())).unwrap();
        assert!(
            !root.borrow().tree_is_folded_for_test("u1"),
            "Ctrl+Right must unfold u1"
        );
        let restored = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            restored.contains("tool:") || restored.contains("read"),
            "unfold must restore descendants:\n{restored}"
        );
    }

    #[tokio::test]
    async fn h18_tree_bare_left_does_not_fold() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u1",
        );
        session.step(HostEvent::Input(bare_left_event())).unwrap();
        assert!(
            !root.borrow().tree_is_folded_for_test("u1"),
            "bare Left must not fold (page chords unbound; fold is Ctrl/Alt+Left)"
        );
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("tool:") || panel.contains("read"),
            "bare Left must keep descendants visible:\n{panel}"
        );
    }

    #[tokio::test]
    async fn h19_tree_shift_f_forks_user_before() {
        use crate::protocol::session::ForkPosition;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u1",
        );
        session.step(HostEvent::Input(shift_f_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.fork_calls(),
            vec![("u1".to_string(), ForkPosition::Before)]
        );
        assert_eq!(driver.switch_calls(), vec!["forked-child".to_string()]);
        assert!(!root.borrow().tree_open(), "tree must close after fork");
        assert_eq!(root.borrow().editor_text(), "hello");
        assert_eq!(
            session
                .ui_model()
                .entries
                .iter()
                .filter(
                    |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("forked →"))
                )
                .count(),
            1,
            "exactly one trailing fork note: {:?}",
            session.ui_model().entries
        );
        let last_fork = session
            .ui_model()
            .entries
            .iter()
            .rev()
            .find_map(|e| match e {
                UiEntry::ScrollNotice { text } if text.contains("forked →") => {
                    Some(text.as_str())
                }
                _ => None,
            });
        assert!(
            last_fork.is_some_and(|t| t.contains("forked-child")),
            "fork note at end: {:?}",
            session.ui_model().entries
        );
    }

    #[tokio::test]
    async fn h20_tree_shift_f_forks_assistant_at() {
        use crate::protocol::session::ForkPosition;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "a1",
        );
        root.borrow_mut().set_editor_text("stale prefill");
        session.step(HostEvent::Input(shift_f_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.fork_calls(),
            vec![("a1".to_string(), ForkPosition::At)]
        );
        assert_eq!(driver.switch_calls(), vec!["forked-child".to_string()]);
        assert!(!root.borrow().tree_open(), "tree must close after fork");
        assert_eq!(
            root.borrow().editor_text(),
            "",
            "At must not prefill user body"
        );
    }

    #[tokio::test]
    async fn h21_tree_slot_search_and_help() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        let slot = root.borrow_mut().tree_slot_text_for_test(80);
        assert!(
            slot.contains("Type to search") || slot.contains("Search:"),
            "expected Search head line; got:\n{slot}"
        );
        assert!(
            slot.contains("filters") && slot.contains("cycle"),
            "expected dynamic TreeHelp filters/cycle; got:\n{slot}"
        );
        assert!(
            !slot.contains("Up/Down  Enter travel (user→input)"),
            "stale hard-coded help must be gone; got:\n{slot}"
        );
        session.step(HostEvent::Input(char_event('f'))).unwrap();
        session.step(HostEvent::Input(char_event('o'))).unwrap();
        session.step(HostEvent::Input(char_event('o'))).unwrap();
        let slot = root.borrow_mut().tree_slot_text_for_test(80);
        assert!(
            slot.contains("Search: foo"),
            "search echo missing; got:\n{slot}"
        );
    }

    #[tokio::test]
    async fn h22_tree_filter_cycle_backward_ctrl_shift_o() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        assert_eq!(
            root.borrow().tree_filter_for_test(),
            crate::app::tui::layout::FilterMode::Default
        );
        session
            .step(HostEvent::Input(ctrl_shift_key_event('o')))
            .unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("[all]"),
            "ctrl+shift+o from default must cycle to all; got:\n{panel}"
        );
        assert_eq!(
            root.borrow().tree_filter_for_test(),
            crate::app::tui::layout::FilterMode::All
        );
    }

    #[tokio::test]
    async fn h23_tree_shift_l_persists_annotation() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "a1",
        );
        session.step(HostEvent::Input(shift_l_event())).unwrap();
        let slot = root.borrow_mut().tree_slot_text_for_test(80);
        assert!(
            slot.contains("Label edit"),
            "Shift+L must open label editor; got:\n{slot}"
        );
        session.step(HostEvent::Input(char_event('k'))).unwrap();
        session.step(HostEvent::Input(char_event('e'))).unwrap();
        session.step(HostEvent::Input(char_event('e'))).unwrap();
        session.step(HostEvent::Input(char_event('p'))).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.label_calls(),
            vec![("a1".to_string(), Some("keep".to_string()))]
        );
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("[keep]"),
            "annotation must show after label save; got:\n{panel}"
        );
    }

    #[tokio::test]
    async fn h24_tree_shift_t_toggles_timestamps() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = open_sample_tree(&mut session);
        // sample has annotation "keep" on u1
        root.borrow_mut()
            .apply_tree_label("u1", Some("keep".into()));
        session.step(HostEvent::Input(shift_t_event())).unwrap();
        let panel = root.borrow_mut().tree_panel_text_for_test(80);
        assert!(
            panel.contains("just now") || panel.contains("[keep]"),
            "Shift+T should reveal timestamp near annotation; got:\n{panel}"
        );
    }

    #[tokio::test]
    async fn h25_debug_list_load_and_arg_completion() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/debug"),
            Some(PendingSlash::DebugScene("list".into()))
        );
        assert_eq!(
            parse_slash_command("/debug session-tree-multiturn"),
            Some(PendingSlash::DebugScene("session-tree-multiturn".into()))
        );
        // Colon form intentionally unsupported (use space).
        assert_eq!(parse_slash_command("/debug:session-tree-multiturn"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/debug");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.debug_scene_calls().is_empty(),
            "list must not call XyDriver::load_debug_scene"
        );
        let note = session
            .ui_model()
            .entries
            .iter()
            .rev()
            .find_map(|e| match e {
                crate::app::tui::UiEntry::ScrollNotice { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or("");
        assert!(
            note.contains("session-tree-multiturn"),
            "list note should name scenes; got: {note}"
        );
        assert!(
            note.contains("Multi-turn") || note.contains("MessageHistory"),
            "list note should include description; got: {note}"
        );

        root.borrow_mut()
            .set_editor_text("/debug session-tree-multiturn");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.debug_scene_calls(),
            vec!["session-tree-multiturn".to_string()]
        );
        assert_eq!(
            driver.session_id().as_deref(),
            Some("debug-session-tree-multiturn-scripted")
        );

        // `/debug ` arg completion (same path as `/model `).
        for ch in "/debug sess".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        let frame = root.borrow_mut().render(80);
        let joined = frame.join("\n");
        assert!(
            joined.contains("session-tree") || joined.contains("multiturn"),
            "debug arg completion should list scene ids; got:\n{joined}"
        );
    }

    #[tokio::test]
    async fn h26_slash_session_tree_opens_session_tree() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-tree"),
            Some(PendingSlash::OpenTree)
        );
        assert_eq!(parse_slash_command("/tree"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_message_history_tree(harness_sample_message_history_tree());
        let mut stream = None;
        root.borrow_mut().set_editor_text("/session-tree");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.session_tree_calls(), 1);
        assert!(root.borrow().tree_open());
        let slot = root.borrow_mut().tree_slot_text_for_test(80);
        assert!(
            slot.contains("Type to search") || slot.contains("Search:"),
            "expected Search row after /session-tree; got:\n{slot}"
        );
    }

    #[tokio::test]
    async fn h27_slash_session_fork_at_leaf() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        use crate::protocol::session::ForkPosition;
        assert_eq!(
            parse_slash_command("/session-fork"),
            Some(PendingSlash::ForkAtLeaf)
        );
        assert_eq!(parse_slash_command("/fork"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_leaf_entry_id(Some("a1".into()));
        let mut stream = None;
        root.borrow_mut().set_editor_text("/session-fork");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.fork_calls(),
            vec![("a1".to_string(), ForkPosition::At)]
        );
        assert_eq!(driver.switch_calls(), vec!["forked-child".to_string()]);
        assert_eq!(
            session
                .ui_model()
                .entries
                .iter()
                .filter(
                    |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("forked →"))
                )
                .count(),
            1,
            "exactly one trailing fork note: {:?}",
            session.ui_model().entries
        );
    }

    #[tokio::test]
    async fn h26b_old_tree_fork_slash_unknown() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;
        for old in ["/tree", "/fork"] {
            root.borrow_mut().set_editor_text(old);
            session.step(HostEvent::Input(enter_event())).unwrap();
            pump_host_driver(&mut session, &mut driver, &mut stream)
                .await
                .unwrap();
        }
        assert_eq!(driver.session_tree_calls(), 0);
        assert_eq!(driver.fork_calls().len(), 0);
        let notes: Vec<_> = session
            .ui_model()
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::ScrollNotice { text } | UiEntry::Error { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            notes
                .iter()
                .any(|t| t.contains("unknown command") && t.contains("/tree")),
            "expected unknown for /tree: {notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|t| t.contains("unknown command") && t.contains("/fork")),
            "expected unknown for /fork: {notes:?}"
        );
    }

    fn system_notes(session: &HostSession<TestTerminal>) -> Vec<String> {
        session
            .ui_model()
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::ScrollNotice { text } | UiEntry::Error { text } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn chrome_toast_body(session: &HostSession<TestTerminal>) -> Option<String> {
        session
            .ui_root()
            .map(|r| r.borrow().chrome_toast_body().map(str::to_string))
            .flatten()
    }

    #[tokio::test]
    async fn h28_slash_session_compact_and_usage() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-compact"),
            Some(PendingSlash::Compact { instructions: None })
        );
        assert_eq!(
            parse_slash_command("/session-compact please focus on errors"),
            Some(PendingSlash::Compact {
                instructions: Some("please focus on errors".into())
            })
        );
        assert_eq!(
            parse_slash_command("/session-compact    "),
            Some(PendingSlash::Compact { instructions: None })
        );
        assert_eq!(parse_slash_command("/compact"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-compact");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.compact_calls(), 1);
        assert_eq!(driver.compact_instructions(), vec![None]);

        root.borrow_mut()
            .set_editor_text("/session-compact please focus on errors");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.compact_calls(),
            2,
            "args must compact with instructions"
        );
        assert_eq!(
            driver.compact_instructions(),
            vec![None, Some("please focus on errors".into())]
        );
    }

    #[tokio::test]
    async fn h29_slash_session_export_html_and_jsonl() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-export"),
            Some(PendingSlash::Export { path: None })
        );
        assert_eq!(
            parse_slash_command("/session-export /tmp/out.jsonl"),
            Some(PendingSlash::Export {
                path: Some("/tmp/out.jsonl".into())
            })
        );
        assert_eq!(parse_slash_command("/export"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-export");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.export_html_calls(), vec!["export.html".to_string()]);
        assert!(driver.export_jsonl_calls().is_empty());
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("exported") && t.contains("export.html")),
            "expected export note: {:?}",
            system_notes(&session)
        );

        root.borrow_mut()
            .set_editor_text("/session-export /tmp/out.jsonl");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.export_jsonl_calls(),
            vec!["/tmp/out.jsonl".to_string()]
        );
    }

    #[tokio::test]
    async fn h30_slash_session_import_confirm_cancel_and_accept() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-import"),
            Some(PendingSlash::Usage("usage: /session-import <path>"))
        );
        assert_eq!(
            parse_slash_command("/session-import /tmp/a.jsonl"),
            Some(PendingSlash::Import {
                path: "/tmp/a.jsonl".into()
            })
        );
        assert_eq!(parse_slash_command("/import"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;

        root.borrow_mut()
            .set_editor_text("/session-import /tmp/a.jsonl");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            root.borrow().import_confirm_open(),
            "import must open confirm slot"
        );
        assert!(driver.import_jsonl_calls().is_empty());

        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(driver.import_jsonl_calls().is_empty());
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("Import cancelled")),
            "expected cancel note: {:?}",
            system_notes(&session)
        );
        assert!(!root.borrow().import_confirm_open());

        root.borrow_mut()
            .set_editor_text("/session-import /tmp/a.jsonl");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().import_confirm_open());
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.import_jsonl_calls(),
            vec!["/tmp/a.jsonl".to_string()]
        );
        assert_eq!(driver.switch_calls(), vec!["imported".to_string()]);
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("imported") && t.contains("imported")),
            "expected import note: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn h31_slash_session_dumps_stats() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session"),
            Some(PendingSlash::SessionDump)
        );
        assert_eq!(
            parse_slash_command("/session info"),
            Some(PendingSlash::Usage("usage: /session (no arguments)"))
        );

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_stats(SessionStats {
            session_id: "sess-abc".into(),
            user_messages: 2,
            assistant_messages: 3,
            total_messages: 5,
            thinking_level: "medium".into(),
            model: Some(("openai".into(), "gpt-4".into())),
        });
        let mut stream = None;
        root.borrow_mut().set_editor_text("/session");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let notes = system_notes(&session);
        let dump = notes
            .iter()
            .find(|t| t.contains("Session Info"))
            .expect("expected stats dump");
        assert!(dump.contains("sess-abc"));
        assert!(dump.contains("User: 2") || dump.contains("User: 2\n"));
        assert!(dump.contains("Assistant: 3"));
        assert!(dump.contains("Total: 5"));
    }

    #[tokio::test]
    async fn h32_slash_session_resume_list_switch_and_esc() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-resume"),
            Some(PendingSlash::OpenSessionResume)
        );
        assert_eq!(parse_slash_command("/resume"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_list(vec![
            SessionListEntry {
                id: "older".into(),
                name: Some("Old chat".into()),
                first_message: Some("hello from older".into()),
                message_count: 4,
                modified_unix: Some(1_700_000_000),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
            SessionListEntry {
                id: "newer".into(),
                name: None,
                first_message: Some("latest dialogue preview".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_100),
                parent_session_id: Some("older".into()),
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
        ]);
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.list_sessions_calls(), 1);
        assert!(root.borrow().session_resume_open());
        let frame = root.borrow_mut().render(80);
        let joined = frame.join("\n");
        assert!(
            joined.contains("older")
                || joined.contains("Old chat")
                || joined.contains("latest dialogue"),
            "expected session list / preview in slot: {joined}"
        );
        assert!(
            joined.contains("latest dialogue preview") || joined.contains("Old chat"),
            "expected first-message preview or name: {joined}"
        );

        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(!root.borrow().session_resume_open());
        assert!(driver.switch_calls().is_empty());

        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.switch_calls(), vec!["newer".to_string()]);
        assert!(!root.borrow().session_resume_open());
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("switched") && t.contains("newer")),
            "expected switch note: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn h37_session_resume_panel_scope_sort_rename_delete_fold() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(100, 30));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        *driver.active_session_id.lock().expect("sid") =
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
        driver.set_session_list(vec![
            SessionListEntry {
                id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
                name: Some("Parent chat".into()),
                first_message: Some("parent preview".into()),
                message_count: 5,
                modified_unix: Some(1_700_000_200),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: Some("/tmp/parent.jsonl".into()),
            },
            SessionListEntry {
                id: "child".into(),
                name: None,
                first_message: Some("child preview line".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_100),
                parent_session_id: Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into()),
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
            SessionListEntry {
                id: "other-cwd".into(),
                name: Some("Elsewhere".into()),
                first_message: Some("other cwd".into()),
                message_count: 1,
                modified_unix: Some(1_700_000_050),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some("/other/project".into()),
                path: None,
            },
        ]);
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        let panel = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            panel.contains("Sort: Threaded") || panel.contains("Current"),
            "expected resume header cues: {panel}"
        );
        assert!(panel.contains("filter"), "expected filter hint: {panel}");
        assert!(panel.contains("ctrl+u"), "expected ctrl+u id hint: {panel}");
        assert!(
            !panel.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            "session id must be hidden by default: {panel}"
        );

        // Ctrl+U shows full session ids in the list rows.
        session.step(HostEvent::Input(ctrl_key_event('u'))).unwrap();
        let with_id = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            with_id.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            "ctrl+u must reveal full session id: {with_id}"
        );
        session.step(HostEvent::Input(ctrl_key_event('u'))).unwrap();
        let id_off = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            !id_off.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            "second ctrl+u must hide session id: {id_off}"
        );

        // Tab to All scope — other-cwd session becomes visible.
        session.step(HostEvent::Input(tab_event())).unwrap();
        let all_scope = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            all_scope.contains("Elsewhere") || all_scope.contains("other-cwd"),
            "expected All scope to show other cwd session: {all_scope}"
        );

        // Named filter hides unnamed child.
        session.step(HostEvent::Input(ctrl_key_event('n'))).unwrap();
        let named_only = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            !named_only.contains("child preview"),
            "Named filter should hide unnamed child: {named_only}"
        );
        session.step(HostEvent::Input(ctrl_key_event('n'))).unwrap();

        // Sort cycle to Recent.
        session.step(HostEvent::Input(ctrl_key_event('s'))).unwrap();
        let recent = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            recent.contains("Sort: Recent"),
            "expected Recent sort label: {recent}"
        );

        // Back to Threaded for fold test.
        session.step(HostEvent::Input(ctrl_key_event('s'))).unwrap();
        session.step(HostEvent::Input(ctrl_key_event('s'))).unwrap();
        let threaded = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            threaded.contains("child preview"),
            "child visible: {threaded}"
        );

        // Fold parent hides child (parent is first row when threaded).
        session.step(HostEvent::Input(ctrl_left_event())).unwrap();
        let folded = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            !folded.contains("child preview"),
            "fold should hide child row: {folded}"
        );
        session.step(HostEvent::Input(ctrl_right_event())).unwrap();

        // Rename non-active child session (unnamed — empty rename buffer).
        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(ctrl_key_event('r'))).unwrap();
        for ch in "child-named".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.set_session_name_for_calls(),
            vec![("child".to_string(), "child-named".to_string())]
        );

        // Back to parent (active) for delete reject test.
        session.step(HostEvent::Input(up_event())).unwrap();
        session.step(HostEvent::Input(ctrl_key_event('d'))).unwrap();
        let reject_panel = root.borrow().session_resume_panel_text_for_test(100);
        assert!(
            reject_panel.contains("Cannot delete the active session"),
            "expected panel reject message: {reject_panel}"
        );
        assert!(driver.delete_session_calls().is_empty());

        // Delete non-active child.
        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(ctrl_key_event('d'))).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.delete_session_calls(), vec!["child".to_string()]);
    }

    #[tokio::test]
    async fn h33_slash_session_new() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-new"),
            Some(PendingSlash::SessionNew)
        );
        assert_eq!(
            parse_slash_command("/session-new x"),
            Some(PendingSlash::Usage("usage: /session-new (no arguments)"))
        );
        assert_eq!(parse_slash_command("/new"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;
        root.borrow_mut().set_editor_text("/session-new");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.new_session_calls(), 1);
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("new session") && t.contains("new-1")),
            "expected new session note: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn h34_slash_session_clone_at_and_no_leaf() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        use crate::protocol::session::ForkPosition;
        assert_eq!(
            parse_slash_command("/session-clone"),
            Some(PendingSlash::SessionClone)
        );
        assert_eq!(parse_slash_command("/clone"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-clone");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(driver.fork_calls().is_empty());
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("Nothing to clone")),
            "expected no-leaf note: {:?}",
            system_notes(&session)
        );

        driver.set_leaf_entry_id(Some("leaf-1".into()));
        driver.set_session_messages(harness_sample_session_messages());
        root.borrow_mut().set_editor_text("/session-clone");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.fork_calls(),
            vec![("leaf-1".to_string(), ForkPosition::At)]
        );
        assert_eq!(driver.switch_calls(), vec!["forked-child".to_string()]);
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("cloned") && t.contains("forked-child")),
            "expected clone note: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn h35_slash_session_name_show_and_set() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};
        assert_eq!(
            parse_slash_command("/session-name"),
            Some(PendingSlash::SessionName { name: None })
        );
        assert_eq!(
            parse_slash_command("/session-name hello"),
            Some(PendingSlash::SessionName {
                name: Some("hello".into())
            })
        );
        assert_eq!(parse_slash_command("/name"), None);

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/session-name");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("usage: /session-name")),
            "expected usage when unnamed: {:?}",
            system_notes(&session)
        );

        root.borrow_mut()
            .set_editor_text("/session-name hello\nworld");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.set_session_name_calls(),
            vec!["hello\nworld".to_string()]
        );
        let notes = system_notes(&session);
        assert!(
            notes.iter().any(|t| t.contains("normalized")),
            "expected normalize warning: {notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|t| t.contains("Session name set:") && t.contains("hello world")),
            "expected set note: {notes:?}"
        );

        root.borrow_mut().set_editor_text("/session-name");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("Session name: hello world")),
            "expected show name: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn h36_slash_fuzzy_enter_applies_before_host_parse() {
        // Regression: host stole Enter while popup showed `session-new` for typed `/new`
        // → unknown command. Must apply selection then parse.
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        for ch in "/new".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        assert!(
            root.borrow().editor_autocomplete_open(),
            "expected slash popup while typing /new; editor={:?}",
            root.borrow().editor_text()
        );
        assert_eq!(root.borrow().editor_text(), "/new");

        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(
            driver.new_session_calls(),
            1,
            "Enter should run /session-new"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t.contains("unknown command")),
            "must not treat /new as unknown when popup selected session-new: {:?}",
            system_notes(&session)
        );
        assert!(
            system_notes(&session)
                .iter()
                .any(|t| t.contains("new session")),
            "expected new-session note: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn c1035_empty_session_omits_footer_token() {
        use crate::app::tui::effects::refresh_footer_tokens;

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let driver = ScriptedDriver::new();
        refresh_footer_tokens(&mut session, &driver).await;
        session.render_now().unwrap();
        let frame = session.ui_root().expect("ui").borrow_mut().render(80);
        let footer = frame.last().expect("footer");
        assert!(
            footer.contains("~/x") && footer.contains("Fake"),
            "footer identity missing: {footer}"
        );
        assert!(
            !footer.contains("used "),
            "empty session must omit used field: {footer}"
        );
        assert!(
            !footer.contains("used 0"),
            "must not forge used 0: {footer}"
        );
    }

    #[tokio::test]
    async fn c1035_heuristic_shows_tilde() {
        use crate::app::tui::effects::refresh_footer_tokens;
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 42,
            provenance: TokenProvenance::Heuristic,
            usage_tokens: 0,
            trailing_tokens: 42,
            last_usage_index: None,
        }));
        refresh_footer_tokens(&mut session, &driver).await;
        let frame = session.ui_root().expect("ui").borrow_mut().render(80);
        let footer = frame.last().expect("footer");
        assert!(
            footer.contains("used ~42 tokens"),
            "Heuristic must show tilde: {footer}"
        );
        assert!(
            footer.contains("~0.5%/8.0k"),
            "Heuristic must derive ~percent with tilde: {footer}"
        );
    }

    #[tokio::test]
    async fn c1680_no_percent_when_window_zero() {
        use crate::app::tui::effects::refresh_footer_tokens;
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_current_model(ModelInfo {
            id: "Fake".into(),
            display_name: "Fake".into(),
            thinking: false,
            thinking_levels: Vec::new(),
            context_window: 0,
        });
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 42,
            provenance: TokenProvenance::Api,
            usage_tokens: 42,
            trailing_tokens: 0,
            last_usage_index: None,
        }));
        refresh_footer_tokens(&mut session, &driver).await;
        let frame = session.ui_root().expect("ui").borrow_mut().render(80);
        let footer = frame.last().expect("footer");
        assert!(
            footer.contains("used 42 tokens"),
            "must keep used field: {footer}"
        );
        assert!(
            !footer.contains("%/"),
            "window=0 must omit percent: {footer}"
        );
    }

    #[tokio::test]
    async fn c1680_api_derived_percent() {
        use crate::app::tui::effects::refresh_footer_tokens;
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_current_model(ModelInfo {
            id: "Fake".into(),
            display_name: "Fake".into(),
            thinking: false,
            thinking_levels: Vec::new(),
            context_window: 128_000,
        });
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 42_000,
            provenance: TokenProvenance::Api,
            usage_tokens: 42_000,
            trailing_tokens: 0,
            last_usage_index: None,
        }));
        refresh_footer_tokens(&mut session, &driver).await;
        let frame = session.ui_root().expect("ui").borrow_mut().render(80);
        let footer = frame.last().expect("footer");
        assert!(
            footer.contains("used 42k tokens") && footer.contains("32.8%/128k"),
            "Api must show derived percent: {footer}"
        );
    }

    #[tokio::test]
    async fn c1035_api_shows_exact_used() {
        use crate::app::tui::effects::refresh_footer_tokens;
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 100,
            provenance: TokenProvenance::Api,
            usage_tokens: 100,
            trailing_tokens: 0,
            last_usage_index: Some(0),
        }));
        refresh_footer_tokens(&mut session, &driver).await;
        let frame = session.ui_root().expect("ui").borrow_mut().render(80);
        let footer = frame.last().expect("footer");
        assert!(
            footer.contains("used 100 tokens") && !footer.contains("used ~"),
            "Api must be exact: {footer}"
        );
        assert!(
            footer.contains("1.3%/8.0k") || footer.contains("1.2%/8.0k"),
            "Api with default window must show derived percent: {footer}"
        );
    }

    #[tokio::test]
    async fn c1035_travel_refreshes_footer_token() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_message_history_tree(harness_sample_message_history_tree());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 11,
            provenance: TokenProvenance::Heuristic,
            usage_tokens: 0,
            trailing_tokens: 11,
            last_usage_index: None,
        }));
        let mut stream = None;
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u1",
        );
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let footer1 = root.borrow_mut().render(80);
        let f1 = footer1.last().expect("footer").clone();
        assert!(
            f1.contains("used ~11 tokens"),
            "first travel estimate: {f1}"
        );

        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 99,
            provenance: TokenProvenance::Heuristic,
            usage_tokens: 0,
            trailing_tokens: 99,
            last_usage_index: None,
        }));
        root.borrow_mut().open_session_tree_at_for_test(
            crate::app::tui::layout::sample_tree_nodes_for_test(),
            "u2",
        );
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let footer2 = root.borrow_mut().render(80);
        let f2 = footer2.last().expect("footer").clone();
        assert!(
            f2.contains("used ~99 tokens"),
            "travel must refresh token field: {f2}"
        );
    }

    #[tokio::test]
    async fn c1730_compaction_end_refreshes_footer_token() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 42,
            provenance: TokenProvenance::Api,
            usage_tokens: 42,
            trailing_tokens: 0,
            last_usage_index: Some(0),
        }));
        session
            .step(HostEvent::Xy(Box::new(XyEvent::CompactionEnd {
                result: Some("ok".into()),
                aborted: false,
                reason: "manual".into(),
                will_retry: false,
                error_message: None,
                summary: Some("done".into()),
                tokens_before: Some(100),
            })))
            .unwrap();
        let mut stream = None;
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let footer = session
            .ui_root()
            .expect("ui")
            .borrow_mut()
            .render(80)
            .last()
            .cloned()
            .unwrap_or_default();
        assert!(
            footer.contains("used 42 tokens"),
            "CompactionEnd must refresh footer tokens: {footer}"
        );
    }

    #[tokio::test]
    async fn c1730_turn_end_refreshes_footer_token() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 11,
            provenance: TokenProvenance::Api,
            usage_tokens: 11,
            trailing_tokens: 0,
            last_usage_index: Some(0),
        }));
        session
            .step(HostEvent::Xy(Box::new(XyEvent::TurnEnd { turn_index: 0 })))
            .unwrap();
        let mut stream = None;
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let footer = session
            .ui_root()
            .expect("ui")
            .borrow_mut()
            .render(80)
            .last()
            .cloned()
            .unwrap_or_default();
        assert!(
            footer.contains("used 11 tokens"),
            "TurnEnd must refresh footer tokens: {footer}"
        );
    }

    #[tokio::test]
    async fn c1035_stream_closed_requests_footer_refresh() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 7,
            provenance: TokenProvenance::LocalTokenizer,
            usage_tokens: 0,
            trailing_tokens: 7,
            last_usage_index: None,
        }));
        // AgentEnd alone must not schedule estimate; stream close is the owner.
        session
            .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
                messages: Vec::new(),
            })))
            .unwrap();
        let mut stream = None;
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let before = session.ui_root().expect("ui").borrow_mut().render(80);
        let before_f = before.last().expect("footer").clone();
        assert!(
            !before_f.contains("used 7 tokens"),
            "AgentEnd must not kick footer estimate alone: {before_f}"
        );

        session.on_run_stream_closed();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let footer = session.ui_root().expect("ui").borrow_mut().render(80);
        let f = footer.last().expect("footer");
        assert!(
            f.contains("used 7 tokens"),
            "stream closed + drain must refresh footer: {f}"
        );
    }

    #[tokio::test]
    async fn c1035_cli_restore_and_resume_refresh_footer_token() {
        use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

        let mut session = HostSession::new_product_ui_with_meta(
            TestTerminal::new(80, 24),
            "~/x".into(),
            "Fake".into(),
        );
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 42_252,
            provenance: TokenProvenance::Api,
            usage_tokens: 42_252,
            trailing_tokens: 0,
            last_usage_index: Some(0),
        }));

        // CLI `--session` restore path: apply then drain (mirrors first host tick).
        session.apply_cli_restored_session("sid-restored", harness_sample_session_messages());
        let before = root.borrow_mut().render(80);
        let before_f = before.last().expect("footer").clone();
        assert!(
            !before_f.contains("used 42k tokens"),
            "restore alone must not sync estimate without drain: {before_f}"
        );
        let mut stream = None;
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let after_restore = root.borrow_mut().render(80);
        let f_restore = after_restore.last().expect("footer").clone();
        assert!(
            f_restore.contains("used 42k tokens"),
            "CLI restore + drain must fill footer tokens: {f_restore}"
        );

        // In-TUI `/session-resume` switch applies mid-drain; same-cycle refresh.
        driver.set_estimate_override(Some(ContextTokenEstimate {
            tokens: 99,
            provenance: TokenProvenance::Heuristic,
            usage_tokens: 0,
            trailing_tokens: 99,
            last_usage_index: None,
        }));
        session.apply_resume_session("sid-other", harness_sample_session_messages());
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        let after_resume = root.borrow_mut().render(80);
        let f_resume = after_resume.last().expect("footer").clone();
        assert!(
            f_resume.contains("used ~99 tokens"),
            "resume switch + drain must refresh footer: {f_resume}"
        );
    }

    #[tokio::test]
    async fn c1120_reload_preserves_session_message_count() {
        use crate::app::tui::commands::{PendingSlash, parse_slash_command};

        assert_eq!(parse_slash_command("/reload"), Some(PendingSlash::Reload));

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_messages(harness_sample_session_messages());
        let before = driver.get_messages().await.unwrap().len();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/reload");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(driver.reload_runtime_calls(), 1);
        assert_eq!(driver.get_messages().await.unwrap().len(), before);
        let notes = system_notes(&session);
        assert!(
            notes.iter().any(|t| t.contains("Reload:")),
            "expected reload report: {notes:?}"
        );
    }

    #[tokio::test]
    async fn c1120_reload_busy_refused() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/reload");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(driver.reload_runtime_calls(), 0);
        let notes = system_notes(&session);
        assert!(
            notes
                .iter()
                .any(|t| t.contains("busy") && t.contains("/reload refused")),
            "expected busy refusal: {notes:?}"
        );
    }

    #[tokio::test]
    async fn c1210_mcp_connecting_allows_prompt_and_reload() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_list(vec![SessionListEntry {
            id: "sid-a".into(),
            name: Some("While connecting".into()),
            first_message: Some("preview".into()),
            message_count: 1,
            modified_unix: Some(1_700_000_000),
            parent_session_id: None,
            tree_prefix: String::new(),
            cwd: Some(".".into()),
            path: None,
        }]);
        let mut stream = None;

        session.set_mcp_blocks_agent(true);

        root.borrow_mut().set_editor_text("hello while connecting");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.runs.len(), 1, "prompt MUST run while MCP connecting");
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t.contains("MCP still connecting")),
            "must not defer prompt: {:?}",
            system_notes(&session)
        );

        root.borrow_mut().set_editor_text("/reload");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(
            driver.reload_runtime_calls(),
            1,
            "/reload MUST run while MCP connecting (idle)"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t.contains("MCP still connecting") && t.contains("/reload")),
            "must not refuse reload: {:?}",
            system_notes(&session)
        );

        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            root.borrow().session_resume_open(),
            "session-resume MUST still open while MCP connecting"
        );
    }

    #[tokio::test]
    async fn c1210_mcp_panel_open_and_short_cue() {
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        let pending_snap = LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_connecting_label: Some("connecting 0/2".into()),
            mcp_servers: vec![
                McpServerSnapshot {
                    id: "fs".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 0,
                },
                McpServerSnapshot {
                    id: "git".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 0,
                },
            ],
            ..LoadedResourcesSnapshot::default()
        };
        driver.set_loaded_resources_for_driver(pending_snap.clone());
        session.refresh_loaded_resources(&driver).await;

        assert_eq!(
            root.borrow().status_next_turn_cue_for_test().as_deref(),
            Some(crate::app::core::driver::MCP_PENDING_CUE),
            "pending MCP MUST set fixed short cue (right-aligned in status)"
        );

        session.ui_model_mut().set_busy_status("Drafting reply");
        session.sync_ui_root_from_model();
        root.borrow_mut().refresh_mcp_short_cue();
        assert_eq!(
            root.borrow().status_next_turn_cue_for_test().as_deref(),
            Some(crate::app::core::driver::MCP_PENDING_CUE),
            "busy MUST keep MCP short cue when no Next turn pending"
        );

        let snaps_before_open = driver.loaded_resources_snapshot_calls();
        root.borrow_mut().set_editor_text("/mcp");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().mcp_open(), "/mcp MUST open SelectList");
        assert_eq!(
            driver.loaded_resources_snapshot_calls(),
            snaps_before_open,
            "cached loaded_resources MUST avoid second snapshot await on /mcp"
        );
        let panel = root.borrow().mcp_panel_text_for_test();
        assert!(panel.contains("fs"), "panel shows server id: {panel}");
        assert!(panel.contains("connecting"), "panel shows phase: {panel}");
        assert!(panel.contains("not armed"), "panel shows armed: {panel}");
        assert!(panel.contains("configured 2"), "panel summary: {panel}");
        assert_eq!(
            root.borrow().mcp_selected_id_for_test().as_deref(),
            Some("fs"),
            "SelectList focuses first server"
        );

        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(!root.borrow().mcp_open(), "Esc MUST close /mcp panel");

        let armed_snap = LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_servers: vec![
                McpServerSnapshot {
                    id: "fs".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: true,
                    tool_count: 3,
                },
                McpServerSnapshot {
                    id: "git".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: true,
                    tool_count: 1,
                },
            ],
            ..LoadedResourcesSnapshot::default()
        };
        driver.set_loaded_resources_for_driver(armed_snap);
        session.refresh_loaded_resources(&driver).await;
        assert_eq!(
            root.borrow().status_next_turn_cue_for_test(),
            None,
            "cue MUST hide when all armed"
        );
    }

    #[tokio::test]
    async fn c1215_mcp_select_list_nav_enter_closes() {
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_servers: vec![
                McpServerSnapshot {
                    id: "fs".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: true,
                    tool_count: 3,
                },
                McpServerSnapshot {
                    id: "git".into(),
                    phase: McpServerPhase::Connecting,
                    tools_armed: false,
                    tool_count: 0,
                },
            ],
            ..LoadedResourcesSnapshot::default()
        });
        session.refresh_loaded_resources(&driver).await;

        root.borrow_mut().set_editor_text("/mcp");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().mcp_open());
        assert_eq!(
            root.borrow().mcp_selected_id_for_test().as_deref(),
            Some("fs")
        );
        assert_eq!(root.borrow().mcp_selected_index_for_test(), 0);

        session.step(HostEvent::Input(down_event())).unwrap();
        assert_eq!(
            root.borrow().mcp_selected_id_for_test().as_deref(),
            Some("git"),
            "↓ MUST move SelectList focus"
        );
        assert_eq!(root.borrow().mcp_selected_index_for_test(), 1);

        session.step(HostEvent::Input(up_event())).unwrap();
        assert_eq!(
            root.borrow().mcp_selected_id_for_test().as_deref(),
            Some("fs"),
            "↑ MUST move SelectList focus back"
        );

        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        assert!(
            !root.borrow().mcp_open(),
            "Enter MUST close /mcp SelectList (MVP; no fake disable)"
        );
    }

    #[tokio::test]
    async fn c1215_mcp_open_awaits_when_cache_empty() {
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        // Driver has MCP data, but UiRoot cache was never refreshed → must await.
        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_servers: vec![McpServerSnapshot {
                id: "only".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 2,
            }],
            ..LoadedResourcesSnapshot::default()
        });
        assert!(
            !session.mcp_cache_usable_for_open(),
            "empty UiRoot cache MUST be unusable"
        );
        let before = driver.loaded_resources_snapshot_calls();

        root.borrow_mut().set_editor_text("/mcp");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(root.borrow().mcp_open());
        assert!(
            driver.loaded_resources_snapshot_calls() > before,
            "empty cache MUST await loaded_resources_snapshot"
        );
        let panel = root.borrow().mcp_panel_text_for_test();
        assert!(
            panel.contains("only"),
            "await path mounts driver snap: {panel}"
        );
        assert!(panel.contains("armed"), "row shows armed: {panel}");
    }

    #[tokio::test]
    async fn c1115_theme_light_applies() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        assert_eq!(root.borrow().layout_theme().palette(), Palette::dark());
        root.borrow_mut().set_editor_text("/theme light");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(root.borrow().layout_theme().palette(), Palette::light());
        assert_eq!(session.theme_preference(), Some("light"));
        let notes = system_notes(&session);
        assert!(
            !notes.iter().any(|t| t.contains("theme →")),
            "success path must not emit theme scroll notice: {notes:?}"
        );
    }

    #[tokio::test]
    async fn c1115_theme_bad_name_keeps_palette() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/theme not-a-theme");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(root.borrow().layout_theme().palette(), Palette::dark());
        assert_eq!(session.theme_preference(), None);
        let notes = system_notes(&session);
        assert!(
            notes
                .iter()
                .any(|t| t.contains("unknown theme") || t.contains("usage: /theme")),
            "expected unknown note: {notes:?}"
        );
    }

    #[tokio::test]
    async fn c1780_theme_busy_allows_apply() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/theme light");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(root.borrow().layout_theme().palette(), Palette::light());
        assert_eq!(session.theme_preference().as_deref(), Some("light"));
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t.contains("/theme refused")),
            "busy MUST Allow /theme: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn c1780_busy_model_opens_picker() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(
            root.borrow().models_open(),
            "busy bare /model MUST open models slot"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t.contains("unavailable while busy") || t.contains("/model refused")),
            "must not refuse open: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn c1780_busy_session_resume_browse_but_switch_refused() {
        use crate::app::tui::commands::BUSY_SESSION_SWITCH_NOTICE;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_list(vec![
            SessionListEntry {
                id: "older".into(),
                name: Some("Old chat".into()),
                first_message: Some("hello from older".into()),
                message_count: 4,
                modified_unix: Some(1_700_000_000),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
            SessionListEntry {
                id: "newer".into(),
                name: None,
                first_message: Some("latest dialogue preview".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_100),
                parent_session_id: Some("older".into()),
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
        ]);
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert_eq!(driver.list_sessions_calls(), 1);
        assert!(
            root.borrow().session_resume_open(),
            "busy /session-resume MUST open panel for browse"
        );

        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(
            driver.switch_calls().is_empty(),
            "busy MUST NOT SwitchSession: {:?}",
            driver.switch_calls()
        );
        assert!(
            root.borrow().session_resume_open(),
            "panel may stay open after refused switch"
        );
        assert_eq!(
            chrome_toast_body(&session).as_deref(),
            Some(BUSY_SESSION_SWITCH_NOTICE),
            "expected chrome toast body A"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t == BUSY_SESSION_SWITCH_NOTICE),
            "MUST NOT ScrollNotice A: {:?}",
            system_notes(&session)
        );
        let frame = root.borrow_mut().render(80);
        let joined = frame.join("\n");
        assert!(
            joined.contains("Error: ") && joined.contains(BUSY_SESSION_SWITCH_NOTICE),
            "frame MUST show Error: + body: {joined}"
        );
    }

    #[tokio::test]
    async fn c1780_busy_session_resume_rename_delete_refused() {
        use crate::app::tui::commands::BUSY_SESSION_SWITCH_NOTICE;

        let mut session = HostSession::new_product_ui(TestTerminal::new(100, 30));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        *driver.active_session_id.lock().expect("sid") =
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
        driver.set_session_list(vec![
            SessionListEntry {
                id: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
                name: Some("Parent chat".into()),
                first_message: Some("parent preview".into()),
                message_count: 5,
                modified_unix: Some(1_700_000_200),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
            SessionListEntry {
                id: "child".into(),
                name: None,
                first_message: Some("child preview line".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_100),
                parent_session_id: Some("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into()),
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
        ]);
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().session_resume_open());

        // Rename child while busy → refuse write + notice A.
        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(ctrl_key_event('r'))).unwrap();
        for ch in "child-named".chars() {
            session.step(HostEvent::Input(char_event(ch))).unwrap();
        }
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.set_session_name_for_calls().is_empty(),
            "busy MUST NOT rename: {:?}",
            driver.set_session_name_for_calls()
        );
        assert_eq!(
            chrome_toast_body(&session).as_deref(),
            Some(BUSY_SESSION_SWITCH_NOTICE),
            "expected chrome toast A after rename"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t == BUSY_SESSION_SWITCH_NOTICE),
            "MUST NOT ScrollNotice A after rename: {:?}",
            system_notes(&session)
        );

        // Delete child while busy → refuse + toast A (replaces prior toast).
        session.step(HostEvent::Input(ctrl_key_event('d'))).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            driver.delete_session_calls().is_empty(),
            "busy MUST NOT delete: {:?}",
            driver.delete_session_calls()
        );
        assert_eq!(
            chrome_toast_body(&session).as_deref(),
            Some(BUSY_SESSION_SWITCH_NOTICE),
            "expected chrome toast A after delete"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t == BUSY_SESSION_SWITCH_NOTICE),
            "MUST NOT ScrollNotice A after delete: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn c1780_bang_busy_session_resume_switch_refused() {
        use crate::app::tui::commands::BUSY_SESSION_SWITCH_NOTICE;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_session_list(vec![
            SessionListEntry {
                id: "older".into(),
                name: Some("Old chat".into()),
                first_message: Some("hello from older".into()),
                message_count: 4,
                modified_unix: Some(1_700_000_000),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
            SessionListEntry {
                id: "newer".into(),
                name: None,
                first_message: Some("latest dialogue preview".into()),
                message_count: 2,
                modified_unix: Some(1_700_000_100),
                parent_session_id: Some("older".into()),
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            },
        ]);
        driver.set_session_messages(harness_sample_session_messages());
        let mut stream = None;

        session.begin_bash_exec("sleep 99", false);
        assert!(session.bash_active());
        assert!(session.is_busy());

        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(
            root.borrow().session_resume_open(),
            "bang-busy MUST still open resume for browse"
        );

        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(
            driver.switch_calls().is_empty(),
            "bang-busy MUST NOT SwitchSession: {:?}",
            driver.switch_calls()
        );
        assert_eq!(
            chrome_toast_body(&session).as_deref(),
            Some(BUSY_SESSION_SWITCH_NOTICE),
            "expected chrome toast A under bang busy"
        );
        assert!(
            !system_notes(&session)
                .iter()
                .any(|t| t == BUSY_SESSION_SWITCH_NOTICE),
            "MUST NOT ScrollNotice A under bang busy: {:?}",
            system_notes(&session)
        );
    }

    #[tokio::test]
    async fn c1800_chrome_toast_ttl_clears_on_tick() {
        use crate::app::tui::commands::BUSY_SESSION_SWITCH_NOTICE;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        session.push_chrome_toast(BUSY_SESSION_SWITCH_NOTICE);
        assert_eq!(
            chrome_toast_body(&session).as_deref(),
            Some(BUSY_SESSION_SWITCH_NOTICE)
        );
        root.borrow_mut().expire_chrome_toast_now();
        session.step(HostEvent::Tick).unwrap();
        assert_eq!(
            chrome_toast_body(&session),
            None,
            "expired toast MUST clear on idle_tick/Tick"
        );
    }

    /// Short terminal + busy Resume: Working stays in content-end viewport (atc23 / c1810).
    #[tokio::test]
    async fn busy_resume_short_terminal_keeps_working_in_viewport() {
        let term_rows = 16usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut entries = Vec::new();
        for i in 0..12 {
            entries.push(SessionListEntry {
                id: format!("s{i}"),
                name: Some(format!("chat {i}")),
                first_message: Some("preview".into()),
                message_count: 1,
                modified_unix: Some(1_700_000_000 + i),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            });
        }
        driver.set_session_list(entries);
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().session_resume_open());

        let frame = root.borrow_mut().render(80);
        let working_idx = frame.iter().position(|l| l.contains("Working"));
        assert!(
            working_idx.is_some(),
            "Working must still be in full render tree: {}",
            frame.len()
        );
        let working_idx = working_idx.expect("Working");
        let vp_top = frame.len().saturating_sub(term_rows);
        assert!(
            working_idx >= vp_top,
            "Working@{working_idx} MUST stay in content-end viewport [vp_top={vp_top}, rows={term_rows}]; frame_len={}",
            frame.len()
        );
    }

    /// Short terminal + busy Models: Working stays in content-end viewport (atc23 / c1810).
    #[tokio::test]
    async fn busy_models_short_terminal_keeps_working_in_viewport() {
        let term_rows = 12usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_available_models(
            (0..20)
                .map(|i| ModelInfo {
                    id: format!("m{i}"),
                    display_name: format!("Model {i}"),
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                })
                .collect(),
        );
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/model");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().models_open(), "bare /model must open Models");

        let frame = root.borrow_mut().render(80);
        assert_busy_lead_in_viewport(&frame, term_rows, "Working");
    }

    /// Short terminal + busy Themes: Working stays in content-end viewport (atc23).
    #[tokio::test]
    async fn busy_themes_short_terminal_keeps_working_in_viewport() {
        let term_rows = 10usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/theme");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().themes_open(), "bare /theme must open Themes");

        let frame = root.borrow_mut().render(80);
        assert_busy_lead_in_viewport(&frame, term_rows, "Working");
    }

    /// Short terminal + busy MCP list: Working stays in content-end viewport (atc23).
    #[tokio::test]
    async fn busy_mcp_short_terminal_keeps_working_in_viewport() {
        use crate::app::core::driver::{
            LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
        };

        let term_rows = 12usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let servers: Vec<_> = (0..16)
            .map(|i| McpServerSnapshot {
                id: format!("srv{i}"),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 2,
            })
            .collect();
        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            mcp_servers: servers,
            mcp_configured: 16,
            ..LoadedResourcesSnapshot::default()
        });
        session.refresh_loaded_resources(&driver).await;
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/mcp");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().mcp_open(), "bare /mcp must open MCP list");

        let frame = root.borrow_mut().render(80);
        assert_busy_lead_in_viewport(&frame, term_rows, "Working");
    }

    /// Short terminal + busy Import confirm: Working stays in content-end viewport (atc23).
    #[tokio::test]
    async fn busy_import_short_terminal_keeps_working_in_viewport() {
        let term_rows = 10usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();

        session.on_run_started("busy");
        session.mount_import_confirm("/tmp/export.jsonl");
        assert!(root.borrow().import_confirm_open());

        let frame = root.borrow_mut().render(80);
        assert_busy_lead_in_viewport(&frame, term_rows, "Working");
    }

    fn assert_busy_lead_in_viewport(frame: &[String], term_rows: usize, lead: &str) {
        let working_idx = frame.iter().position(|l| l.contains(lead));
        assert!(
            working_idx.is_some(),
            "{lead} must still be in full render tree: len={}",
            frame.len()
        );
        let working_idx = working_idx.expect(lead);
        let vp_top = frame.len().saturating_sub(term_rows);
        assert!(
            working_idx >= vp_top,
            "{lead}@{working_idx} MUST stay in content-end viewport [vp_top={vp_top}, rows={term_rows}]; frame_len={}",
            frame.len()
        );
    }

    /// Toast occupies reserved: short terminal keeps Working **and** chrome toast in viewport.
    #[tokio::test]
    async fn busy_resume_short_terminal_with_toast_keeps_working_and_toast_in_viewport() {
        use crate::app::tui::commands::{BUSY_SESSION_SWITCH_NOTICE, CHROME_TOAST_ERROR_PREFIX};

        let term_rows = 16usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut entries = Vec::new();
        for i in 0..12 {
            entries.push(SessionListEntry {
                id: format!("s{i}"),
                name: Some(format!("chat {i}")),
                first_message: Some("preview".into()),
                message_count: 1,
                modified_unix: Some(1_700_000_000 + i),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            });
        }
        driver.set_session_list(entries);
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().session_resume_open());
        session.push_chrome_toast(BUSY_SESSION_SWITCH_NOTICE);

        let frame = root.borrow_mut().render(80);
        let vp_top = frame.len().saturating_sub(term_rows);
        let working_idx = frame.iter().position(|l| l.contains("Working"));
        let toast_idx = frame.iter().position(|l| {
            l.contains(CHROME_TOAST_ERROR_PREFIX) && l.contains(BUSY_SESSION_SWITCH_NOTICE)
        });
        assert!(
            working_idx.is_some_and(|i| i >= vp_top),
            "Working MUST stay in viewport [vp_top={vp_top}]; frame_len={}",
            frame.len()
        );
        assert!(
            toast_idx.is_some_and(|i| i >= vp_top),
            "chrome toast MUST stay in viewport when reserved (atc23); vp_top={vp_top} toast={toast_idx:?} frame_len={}",
            frame.len()
        );
    }

    /// Queue strip occupies reserved: short terminal keeps Working **and** Steering line in viewport.
    #[tokio::test]
    async fn busy_resume_short_terminal_with_queue_keeps_working_and_steer_in_viewport() {
        let term_rows = 16usize;
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, term_rows as u16));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut entries = Vec::new();
        for i in 0..12 {
            entries.push(SessionListEntry {
                id: format!("s{i}"),
                name: Some(format!("chat {i}")),
                first_message: Some("preview".into()),
                message_count: 1,
                modified_unix: Some(1_700_000_000 + i),
                parent_session_id: None,
                tree_prefix: String::new(),
                cwd: Some(".".into()),
                path: None,
            });
        }
        driver.set_session_list(entries);
        let mut stream = None;

        session.on_run_started("busy");
        root.borrow_mut().set_editor_text("/session-resume");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().session_resume_open());
        session
            .ui_model_mut()
            .enqueue_steer_strip("nudge while listing".into());
        session.sync_ui_root_from_model();

        let frame = root.borrow_mut().render(80);
        let vp_top = frame.len().saturating_sub(term_rows);
        let working_idx = frame.iter().position(|l| l.contains("Working"));
        let steer_idx = frame.iter().position(|l| l.contains("Steering:"));
        assert!(
            working_idx.is_some_and(|i| i >= vp_top),
            "Working MUST stay in viewport [vp_top={vp_top}]; frame_len={}",
            frame.len()
        );
        assert!(
            steer_idx.is_some_and(|i| i >= vp_top),
            "queue Steering line MUST stay in viewport when reserved (atc23); vp_top={vp_top} steer={steer_idx:?} frame_len={}",
            frame.len()
        );
    }

    #[tokio::test]
    async fn c1115_theme_bare_opens_slot() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/theme");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(
            root.borrow().themes_open(),
            "bare /theme must open Themes slot"
        );
        let frame = root.borrow_mut().render(80);
        assert!(
            frame.iter().any(|l| l.contains("dark")),
            "expected dark in frame: {frame:?}"
        );
        assert!(
            frame.iter().any(|l| l.contains("light")),
            "expected light in frame: {frame:?}"
        );
    }

    #[tokio::test]
    async fn c1115_theme_slot_select_light() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/theme");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().themes_open());

        // dark is first (*); move down to light then Enter.
        session.step(HostEvent::Input(down_event())).unwrap();
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert!(!root.borrow().themes_open());
        assert_eq!(root.borrow().layout_theme().palette(), Palette::light());
        assert_eq!(session.theme_preference(), Some("light"));
    }

    #[tokio::test]
    async fn c1115_theme_slot_esc_keeps_dark() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        root.borrow_mut().set_editor_text("/theme");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();
        assert!(root.borrow().themes_open());
        session.step(HostEvent::Input(esc_event())).unwrap();
        assert!(!root.borrow().themes_open());
        assert_eq!(root.borrow().layout_theme().palette(), Palette::dark());
        assert_eq!(session.theme_preference(), None);
    }

    #[tokio::test]
    async fn c1115_theme_toggle_from_dark() {
        use xylitol_tui::Palette;

        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        let mut stream = None;

        // Seed preference dark so toggle has a known baseline.
        session.reload_themes("dark").unwrap();
        root.borrow_mut().set_editor_text("/theme toggle");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(root.borrow().layout_theme().palette(), Palette::light());
        assert_eq!(session.theme_preference(), Some("light"));
    }

    #[tokio::test]
    async fn c1115_theme_catalog_lists_theme() {
        use crate::app::core::product_commands::product_slash_commands;
        use crate::app::tui::layout::product_slash_commands_for_editor;

        let ssot: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
        assert!(ssot.contains(&"theme"));
        let catalog: Vec<String> = product_slash_commands_for_editor()
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert!(catalog.iter().any(|n| n == "theme"));
    }

    #[test]
    fn c1115_product_host_no_theme_auto() {
        use xylitol_tui::Palette;

        // Product HostSession has no theme-auto probe; default is fixed dark.
        let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui");
        assert_eq!(root.borrow().layout_theme().palette(), Palette::dark());
        assert_eq!(session.theme_preference(), None);
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

    // ── c1470: no global thinking cycle (picker-only) ──────────────

    #[test]
    fn c1470_scripted_driver_cycle_wraps_support_list() {
        let mut driver = ScriptedDriver::new();
        driver.set_thinking_levels(vec![ThinkingLevel::Off, ThinkingLevel::High]);
        assert_eq!(driver.thinking_level(), ThinkingLevel::Off);
        assert_eq!(driver.cycle_thinking_level().unwrap(), ThinkingLevel::High);
        assert_eq!(
            driver.cycle_thinking_level().unwrap(),
            ThinkingLevel::Off,
            "must wrap"
        );
        assert!(driver.set_thinking_level(ThinkingLevel::Xhigh).is_err());
        assert_eq!(driver.thinking_level(), ThinkingLevel::Off);
    }

    #[tokio::test]
    async fn c1470_shift_tab_does_not_cycle_outside_picker() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_thinking_levels(vec![ThinkingLevel::Off, ThinkingLevel::High]);
        session.apply_thinking_level_ui(driver.thinking_level());
        let mut stream = None;

        let entries_before = root.borrow().ui_model_entries_len_for_test();
        session.step(HostEvent::Input(shift_tab_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(driver.thinking_level(), ThinkingLevel::Off);
        assert_eq!(root.borrow().thinking_level_for_test(), ThinkingLevel::Off);
        assert_eq!(
            root.borrow().ui_model_entries_len_for_test(),
            entries_before,
            "MUST NOT push thinking-border scroll notice"
        );
    }

    #[tokio::test]
    async fn c1470_busy_shift_tab_does_not_cycle() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_thinking_levels(vec![ThinkingLevel::Off, ThinkingLevel::High]);
        session.apply_thinking_level_ui(driver.thinking_level());
        session.on_run_started("busy");
        assert!(session.is_busy());
        let mut stream = None;

        let entries_before = root.borrow().ui_model_entries_len_for_test();
        session.step(HostEvent::Input(shift_tab_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        assert_eq!(driver.thinking_level(), ThinkingLevel::Off);
        assert_eq!(
            root.borrow().ui_model_entries_len_for_test(),
            entries_before,
            "busy Shift+Tab must not cycle or emit refuse note"
        );
        let frame = root.borrow_mut().render(80).join("\n");
        assert!(!frame.contains("refused"));
        assert!(!frame.contains("thinking-border"));
    }

    #[tokio::test]
    async fn c1135_skills_visible_above_scrollback() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            skill_names: vec!["demo".into(), "other".into()],
            ..LoadedResourcesSnapshot::default()
        });
        session.refresh_loaded_resources(&driver).await;
        session.push_scroll_notice("scrollback marker");

        let frame = root.borrow_mut().render(80);
        let joined = frame.join("\n");
        assert!(
            joined.contains("xylitol")
                && joined.contains("skills")
                && joined.contains("demo")
                && !joined.contains("..."),
            "brand/skills missing or truncated: {joined}"
        );
        assert!(
            !joined.contains("Prompt") && !joined.contains("prompt template"),
            "MUST NOT list prompts: {joined}"
        );
        let skills_idx = frame
            .iter()
            .position(|l| l.contains("skills"))
            .expect("skills row");
        let marker_idx = frame
            .iter()
            .position(|l| l.contains("scrollback marker"))
            .expect("scrollback marker");
        assert!(
            skills_idx < marker_idx,
            "skills must render above scrollback: skills={skills_idx} marker={marker_idx}"
        );
    }

    #[tokio::test]
    async fn c1135_mcp_visible_when_scripted() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            mcp_connected: vec![("fs".into(), 3), ("git".into(), 1)],
            mcp_configured: 2,
            ..LoadedResourcesSnapshot::default()
        });
        session.refresh_loaded_resources(&driver).await;

        let frame = root.borrow_mut().render(80).join("\n");
        assert!(
            frame.contains("mcp") && frame.contains("fs(3)") && frame.contains("git(1)"),
            "MCP line missing: {frame}"
        );
    }

    #[tokio::test]
    async fn c1135_empty_snapshot_keeps_brand_only() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let driver = ScriptedDriver::new();
        session.refresh_loaded_resources(&driver).await;

        let frame = root.borrow_mut().render(80).join("\n");
        assert!(frame.contains("xylitol"), "brand must remain: {frame}");
        assert!(!frame.contains("木糖醇"), "must not show 木糖醇: {frame}");
        assert!(
            !frame.contains("Skills") && !frame.contains("MCP"),
            "empty resources must omit Skills/MCP rows: {frame}"
        );
    }

    #[tokio::test]
    async fn c1135_reload_refreshes_header() {
        let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
        let root = session.ui_root().expect("ui").clone();
        let mut driver = ScriptedDriver::new();
        driver.set_loaded_resources_for_driver(LoadedResourcesSnapshot {
            skill_names: vec!["old".into()],
            ..LoadedResourcesSnapshot::default()
        });
        session.refresh_loaded_resources(&driver).await;
        let before = root.borrow_mut().render(80).join("\n");
        assert!(before.contains("skills") && before.contains("old"));

        driver.set_dollar_skill_catalog_for_driver(vec![
            ("demo".into(), "demo skill".into()),
            ("other".into(), "other skill".into()),
        ]);
        let mut stream = None;
        root.borrow_mut().set_editor_text("/reload");
        session.step(HostEvent::Input(enter_event())).unwrap();
        pump_host_driver(&mut session, &mut driver, &mut stream)
            .await
            .unwrap();

        let after = root.borrow_mut().render(80).join("\n");
        assert!(
            after.contains("skills") && after.contains("demo") && after.contains("other"),
            "reload must refresh loaded-resources: {after}"
        );
        let skills_line = after
            .lines()
            .find(|l| l.contains("skills"))
            .expect("skills line after reload");
        assert!(
            !skills_line.contains("old"),
            "stale skill must not remain: {skills_line}"
        );
    }
}
