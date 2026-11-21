use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use crate::driver::{
    CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo, QueueStats, ReloadStepReport,
    RuntimeReloadReport, SessionListEntry, SessionStats, XyDriver, XyDriverError, XyEvent,
};
use xylitol_plugin_sdk::model::THINKING_OFF;
use xylitol_plugin_sdk::session::{
    SessionEntry, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};
use xylitol_plugin_sdk::{BashOutcome, XyBashResult};

pub struct ScriptedDriver {
    pub runs: Vec<String>,
    pub run_with_context_calls:
        Mutex<Vec<(String, Vec<xylitol_plugin_sdk::mcp::McpResourceContext>)>>,
    pub mcp_catalog: Mutex<Option<xylitol_plugin_sdk::mcp::McpCatalog>>,
    pub mcp_resource_reads: Mutex<Vec<(String, String)>>,
    pub mcp_resource_contents: Mutex<Vec<xylitol_plugin_sdk::mcp::McpResourceContent>>,
    pub mcp_resource_failures: Mutex<Vec<(String, String)>>,
    pub mcp_prompt_gets: Mutex<Vec<(String, String, serde_json::Value)>>,
    pub mcp_prompt_result: Option<serde_json::Value>,
    pub steer_calls: Vec<String>,
    pub follow_up_calls: Vec<String>,
    pub clear_calls: Vec<(bool, bool)>,
    pub observe_state: std::sync::Mutex<crate::driver::AttachObserve>,
    bash_calls: Mutex<Vec<(String, bool)>>,
    abort_count: AtomicUsize,
    models_calls: AtomicUsize,
    hang_bash_until_abort: AtomicBool,
    bash_delay: Option<Duration>,
    aborted: std::sync::Arc<AtomicBool>,
    scripts: VecDeque<Vec<XyEvent>>,
    default_script: Vec<XyEvent>,
    bash_results: Mutex<VecDeque<XyBashResult>>,
    default_bash: XyBashResult,
    steer_queued: Mutex<Vec<xylitol_plugin_sdk::session::QueueItem>>,
    follow_up_queued: Mutex<Vec<xylitol_plugin_sdk::session::QueueItem>>,
    next_queue_id: usize,
    pub(crate) model: ModelInfo,
    available_models: Vec<ModelInfo>,
    message_history_tree: Vec<SessionTreeNode>,
    session_messages: Vec<SessionEntry>,
    get_messages_error: Option<String>,
    travel_overrides: HashMap<String, SessionTreeTravel>,
    session_tree_calls: AtomicUsize,
    travel_calls: Mutex<Vec<String>>,
    fork_calls: Mutex<Vec<(String, xylitol_plugin_sdk::session::ForkPosition)>>,
    switch_calls: Mutex<Vec<String>>,
    label_calls: Mutex<Vec<(String, Option<String>)>>,
    pub(crate) active_session_id: Mutex<String>,
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
    generate_session_name_calls: AtomicUsize,
    set_session_name_calls: Mutex<Vec<String>>,
    set_session_name_for_calls: Mutex<Vec<(String, String)>>,
    delete_session_calls: Mutex<Vec<String>>,
    estimate_override: Option<xylitol_plugin_sdk::model::ContextTokenEstimate>,
    estimate_calls: AtomicUsize,
    reload_runtime_calls: AtomicUsize,
    hang_reload_until_cancel: AtomicBool,
    fail_reload: AtomicBool,
    persist_project_trust_calls: Mutex<Vec<crate::driver::ProjectTrustMode>>,
    copy_text_calls: Mutex<Vec<String>>,
    copy_pending_osc52: Mutex<Option<String>>,
    dollar_skill_catalog: Mutex<Vec<(String, String)>>,
    loaded_resources: Mutex<LoadedResourcesSnapshot>,
    loaded_resources_snapshot_calls: AtomicUsize,
    capability_snapshot: Mutex<xylitol_plugin_sdk::session::CapabilitySnapshot>,
    capability_calls: Mutex<Vec<(String, bool)>>,
    clear_capabilities_calls: AtomicUsize,
    thinking_level: String,
    thinking_levels: Vec<String>,
    clipboard_image: Mutex<Option<(Vec<u8>, String)>>,
    staged_paste_paths: Mutex<Vec<String>>,
    clipboard_image_error: Mutex<Option<String>>,
    clipboard_text: Mutex<Option<String>>,
    tools_frozen: AtomicBool,
    arm_park: Option<tokio::sync::oneshot::Receiver<()>>,
    pub force_zero_queue_stats: AtomicBool,
    bash_run_sink: Mutex<Option<xylitol_plugin_sdk::BashOutputSink>>,
}

impl ScriptedDriver {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            run_with_context_calls: Mutex::new(Vec::new()),
            mcp_catalog: Mutex::new(None),
            mcp_resource_reads: Mutex::new(Vec::new()),
            mcp_resource_contents: Mutex::new(Vec::new()),
            mcp_resource_failures: Mutex::new(Vec::new()),
            mcp_prompt_gets: Mutex::new(Vec::new()),
            mcp_prompt_result: None,
            steer_calls: Vec::new(),
            follow_up_calls: Vec::new(),
            clear_calls: Vec::new(),
            observe_state: std::sync::Mutex::new(crate::driver::AttachObserve::Live),
            bash_calls: Mutex::new(Vec::new()),
            abort_count: AtomicUsize::new(0),
            models_calls: AtomicUsize::new(0),
            hang_bash_until_abort: AtomicBool::new(false),
            bash_delay: None,
            aborted: std::sync::Arc::new(AtomicBool::new(false)),
            scripts: VecDeque::new(),
            default_script: vec![
                XyEvent::AgentStart {
                    session_id: "s".into(),
                    model: "fake".into(),
                },
                XyEvent::TextDelta("hello".into()),
                XyEvent::AgentEnd {
                    reason: None,
                    messages: Vec::new(),
                },
            ],
            bash_results: Mutex::new(VecDeque::new()),
            default_bash: XyBashResult {
                output: "ok".into(),
                exit_code: Some(0),
                outcome: BashOutcome::Completed,
                truncated: false,
                full_output_path: None,
            },
            steer_queued: Mutex::new(Vec::new()),
            follow_up_queued: Mutex::new(Vec::new()),
            next_queue_id: 1,
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
                    id: "model-fast".into(),
                    display_name: "Model Fast".into(),
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                },
                ModelInfo {
                    id: "model-think".into(),
                    display_name: "Model Think".into(),
                    thinking: true,
                    thinking_levels: Vec::new(),
                    context_window: 32_000,
                },
            ],
            message_history_tree: Vec::new(),
            session_messages: Vec::new(),
            get_messages_error: None,
            travel_overrides: HashMap::new(),
            session_tree_calls: AtomicUsize::new(0),
            travel_calls: Mutex::new(Vec::new()),
            fork_calls: Mutex::new(Vec::new()),
            switch_calls: Mutex::new(Vec::new()),
            label_calls: Mutex::new(Vec::new()),
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
            generate_session_name_calls: AtomicUsize::new(0),
            set_session_name_calls: Mutex::new(Vec::new()),
            set_session_name_for_calls: Mutex::new(Vec::new()),
            delete_session_calls: Mutex::new(Vec::new()),
            estimate_override: None,
            estimate_calls: AtomicUsize::new(0),
            reload_runtime_calls: AtomicUsize::new(0),
            hang_reload_until_cancel: AtomicBool::new(false),
            fail_reload: AtomicBool::new(false),
            persist_project_trust_calls: Mutex::new(Vec::new()),
            copy_text_calls: Mutex::new(Vec::new()),
            copy_pending_osc52: Mutex::new(None),
            dollar_skill_catalog: Mutex::new(Vec::new()),
            loaded_resources: Mutex::new(LoadedResourcesSnapshot::default()),
            loaded_resources_snapshot_calls: AtomicUsize::new(0),
            capability_snapshot: Mutex::new(
                xylitol_plugin_sdk::session::CapabilitySnapshot::default(),
            ),
            capability_calls: Mutex::new(Vec::new()),
            clear_capabilities_calls: AtomicUsize::new(0),
            thinking_level: THINKING_OFF.into(),
            thinking_levels: vec![
                "off".into(),
                "minimal".into(),
                "low".into(),
                "medium".into(),
                "high".into(),
            ],
            clipboard_image: Mutex::new(None),
            staged_paste_paths: Mutex::new(Vec::new()),
            clipboard_image_error: Mutex::new(None),
            clipboard_text: Mutex::new(None),
            tools_frozen: AtomicBool::new(true),
            arm_park: None,
            force_zero_queue_stats: AtomicBool::new(false),
            bash_run_sink: Mutex::new(None),
        }
    }

    pub fn set_tools_frozen(&self, frozen: bool) {
        self.tools_frozen.store(frozen, Ordering::SeqCst);
    }

    pub fn park_arm_tool_freeze(&mut self) -> tokio::sync::oneshot::Sender<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.arm_park = Some(rx);
        tx
    }

    pub fn set_clipboard_image(&self, bytes: Vec<u8>, mime: impl Into<String>) {
        *self.clipboard_image.lock().expect("clipboard_image") = Some((bytes, mime.into()));
    }

    pub fn set_clipboard_image_error(&self, err: impl Into<String>) {
        *self
            .clipboard_image_error
            .lock()
            .expect("clipboard_image_error") = Some(err.into());
    }

    pub fn set_clipboard_text(&self, text: impl Into<String>) {
        *self.clipboard_text.lock().expect("clipboard_text") = Some(text.into());
    }

    pub fn staged_paste_paths(&self) -> Vec<String> {
        self.staged_paste_paths
            .lock()
            .expect("staged_paste_paths")
            .clone()
    }

    pub fn set_thinking_levels(&mut self, levels: Vec<String>) {
        self.thinking_levels = levels;
        if !self.thinking_levels.is_empty()
            && !self
                .thinking_levels
                .iter()
                .any(|level| level == &self.thinking_level)
        {
            self.thinking_level = self.thinking_levels[0].clone();
        }
    }

    pub fn set_next_copy_pending_osc52(&self, sequence: Option<String>) {
        *self.copy_pending_osc52.lock().expect("copy_pending_osc52") = sequence;
    }

    pub fn reload_runtime_calls(&self) -> usize {
        self.reload_runtime_calls.load(Ordering::SeqCst)
    }

    pub fn set_hang_reload_until_cancel(&self, hang: bool) {
        self.hang_reload_until_cancel.store(hang, Ordering::SeqCst);
    }

    pub fn set_fail_reload(&self, fail: bool) {
        self.fail_reload.store(fail, Ordering::SeqCst);
    }

    pub fn persist_project_trust_calls(&self) -> Vec<crate::driver::ProjectTrustMode> {
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

    pub fn set_loaded_resources_for_driver(&self, snap: LoadedResourcesSnapshot) {
        *self.loaded_resources.lock().expect("loaded_resources") = snap;
    }

    pub fn loaded_resources_snapshot_calls(&self) -> usize {
        self.loaded_resources_snapshot_calls.load(Ordering::SeqCst)
    }

    pub fn seed_capabilities(&self, entries: Vec<xylitol_plugin_sdk::session::CapabilityEntry>) {
        *self
            .capability_snapshot
            .lock()
            .expect("capability_snapshot") = xylitol_plugin_sdk::session::CapabilitySnapshot {
            entries,
            pending: false,
        };
    }

    pub fn capability_calls(&self) -> Vec<(String, bool)> {
        self.capability_calls
            .lock()
            .expect("capability_calls")
            .clone()
    }

    pub fn clear_capabilities_calls(&self) -> usize {
        self.clear_capabilities_calls.load(Ordering::SeqCst)
    }

    pub fn new_session_calls(&self) -> usize {
        self.new_session_calls.load(Ordering::SeqCst)
    }

    pub fn generate_session_name_calls(&self) -> usize {
        self.generate_session_name_calls.load(Ordering::SeqCst)
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

    pub fn set_get_messages_error(&mut self, err: Option<String>) {
        self.get_messages_error = err;
    }

    pub fn set_session_messages(&mut self, entries: Vec<SessionEntry>) {
        self.session_messages = entries;
    }

    pub fn set_estimate_override(
        &mut self,
        estimate: Option<xylitol_plugin_sdk::model::ContextTokenEstimate>,
    ) {
        self.estimate_override = estimate;
    }

    pub fn estimate_calls(&self) -> usize {
        self.estimate_calls.load(Ordering::SeqCst)
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

    pub fn fork_calls(&self) -> Vec<(String, xylitol_plugin_sdk::session::ForkPosition)> {
        self.fork_calls.lock().expect("fork_calls").clone()
    }

    pub fn label_calls(&self) -> Vec<(String, Option<String>)> {
        self.label_calls.lock().expect("label_calls").clone()
    }

    pub fn set_leaf_entry_id(&self, id: Option<String>) {
        *self.leaf_entry_id.lock().expect("leaf") = id;
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

    pub fn current_model_id(&self) -> String {
        self.model.id.clone()
    }

    pub fn active_session_id(&self) -> String {
        self.active_session_id.lock().expect("sid").clone()
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

    pub fn set_bash_delay(&mut self, delay: Option<Duration>) {
        self.bash_delay = delay;
    }

    pub fn bash_calls(&self) -> Vec<(String, bool)> {
        self.bash_calls.lock().expect("bash_calls").clone()
    }

    pub fn abort_count(&self) -> usize {
        self.abort_count.load(Ordering::SeqCst)
    }

    pub fn models_calls(&self) -> usize {
        self.models_calls.load(Ordering::SeqCst)
    }
}

impl Default for ScriptedDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl XyDriver for ScriptedDriver {
    fn attach_observe(&self) -> crate::driver::AttachObserve {
        *self.observe_state.lock().unwrap_or_else(|e| e.into_inner())
    }

    async fn run(&mut self, prompt: &str) -> EventStream {
        self.runs.push(prompt.to_string());
        let events = self
            .scripts
            .pop_front()
            .unwrap_or_else(|| self.default_script.clone());
        Box::pin(futures::stream::iter(events))
    }

    async fn run_with_context(
        &mut self,
        prompt: &str,
        contexts: Vec<xylitol_plugin_sdk::mcp::McpResourceContext>,
    ) -> EventStream {
        self.run_with_context_calls
            .lock()
            .expect("run_with_context_calls")
            .push((prompt.to_string(), contexts));
        self.run(prompt).await
    }

    async fn mcp_catalog(&mut self) -> Result<xylitol_plugin_sdk::mcp::McpCatalog, XyDriverError> {
        Ok(self
            .mcp_catalog
            .lock()
            .expect("mcp_catalog")
            .clone()
            .unwrap_or_default())
    }

    async fn mcp_resource_read(
        &mut self,
        server: &str,
        uri: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpResourceContent, XyDriverError> {
        self.mcp_resource_reads
            .lock()
            .expect("mcp_resource_reads")
            .push((server.to_string(), uri.to_string()));
        if self
            .mcp_resource_failures
            .lock()
            .expect("mcp_resource_failures")
            .iter()
            .any(|(s, u)| s == server && u == uri)
        {
            return Err(XyDriverError::message(format!(
                "scripted read failure for {server}/{uri}"
            )));
        }
        if let Some(content) = self
            .mcp_resource_contents
            .lock()
            .expect("mcp_resource_contents")
            .iter()
            .find(|c| c.server == server && c.uri == uri)
        {
            return Ok(content.clone());
        }
        Ok(xylitol_plugin_sdk::mcp::McpResourceContent {
            server: server.to_string(),
            uri: uri.to_string(),
            mime: Some("text/plain".into()),
            text: format!("content of {server}/{uri}"),
            truncated: false,
            binary: false,
        })
    }

    async fn mcp_prompt_get(
        &mut self,
        server: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, XyDriverError> {
        self.mcp_prompt_gets.lock().expect("mcp_prompt_gets").push((
            server.to_string(),
            name.to_string(),
            arguments,
        ));
        Ok(self.mcp_prompt_result.clone().unwrap_or(serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": {"type": "text", "text": format!("prompt text {server}/{name}")}
                }
            ]
        })))
    }

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<crate::driver::BashRun, XyDriverError> {
        self.aborted.store(false, Ordering::SeqCst);
        self.bash_calls
            .lock()
            .expect("bash_calls")
            .push((command.to_string(), exclude_from_context));
        let aborted = self.aborted.clone();
        let hang = self.hang_bash_until_abort.load(Ordering::SeqCst);
        let delay = self.bash_delay;
        let sink = self.bash_run_sink.lock().expect("bash_run_sink").take();
        let result = self
            .bash_results
            .lock()
            .expect("bash_results")
            .pop_front()
            .unwrap_or_else(|| self.default_bash.clone());
        Ok(Box::pin(async move {
            if let Some(delay) = delay {
                tokio::time::sleep(delay).await;
            }
            if hang {
                while !aborted.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                return Ok(XyBashResult {
                    output: String::new(),
                    exit_code: None,
                    outcome: BashOutcome::Killed,
                    truncated: false,
                    full_output_path: None,
                });
            }
            if let Some(sink) = sink {
                let _ = sink.tx.try_send(xylitol_plugin_sdk::BashChunk {
                    bash_id: "scripted-bash".into(),
                    seq: 1,
                    data: result.output.clone(),
                });
            }
            Ok(result)
        }) as crate::driver::BashRun)
    }

    fn abort(&self) {
        self.abort_count.fetch_add(1, Ordering::SeqCst);
        self.aborted.store(true, Ordering::SeqCst);
    }

    fn queue_items(&self) -> Vec<xylitol_plugin_sdk::session::QueueItem> {
        ScriptedDriver::queue_items(self)
    }

    fn revoke_queue_item(&self, id: &str) -> xylitol_plugin_sdk::RevokeQueueItemOutcome {
        self.revoke_queued_item(id)
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        *self.bash_run_sink.lock().expect("bash_run_sink") = sink;
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

    fn thinking_level(&self) -> String {
        self.thinking_level.clone()
    }

    fn session_id(&self) -> Option<String> {
        Some(self.active_session_id.lock().expect("sid").clone())
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        self.estimate_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(est) = self.estimate_override.clone() {
            return Ok(est);
        }
        Ok(crate::driver::estimate_from_session_entries(
            &self.session_messages,
            self.current_model().map(|m| m.id),
            None,
        ))
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        Vec::new()
    }

    fn leaf_entry_id(&self) -> Option<String> {
        self.leaf_entry_id.lock().expect("leaf").clone()
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

    fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        self.capability_snapshot
            .lock()
            .expect("capability_snapshot")
            .clone()
    }

    fn capability_pending(&self) -> bool {
        self.capability_snapshot()
            .entries
            .iter()
            .any(|entry| entry.pending())
    }

    async fn set_capability(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        self.capability_calls
            .lock()
            .expect("capability_calls")
            .push((id.to_string(), enabled));
        let mut snap = self.capability_snapshot();
        let Some(position) = snap.entries.iter().position(|entry| entry.id == id) else {
            return Err(XyDriverError::invalid_input(format!(
                "unknown capability `{id}`"
            )));
        };
        let entry = &snap.entries[position];
        let updated = xylitol_plugin_sdk::session::CapabilityEntry::new(
            entry.id.clone(),
            entry.kind,
            entry.label.clone(),
            !enabled,
            enabled,
        );
        snap.entries[position] = updated;
        snap.pending = true;
        *self
            .capability_snapshot
            .lock()
            .expect("capability_snapshot") = snap.clone();
        Ok(snap)
    }

    async fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        self.clear_capabilities_calls.fetch_add(1, Ordering::SeqCst);
        let mut snap = self.capability_snapshot();
        for entry in std::mem::take(&mut snap.entries) {
            snap.entries
                .push(xylitol_plugin_sdk::session::CapabilityEntry::new(
                    entry.id,
                    entry.kind,
                    entry.label,
                    false,
                    false,
                ));
        }
        snap.pending = false;
        *self
            .capability_snapshot
            .lock()
            .expect("capability_snapshot") = snap.clone();
        Ok(snap)
    }

    fn is_tools_frozen(&self) -> bool {
        self.tools_frozen.load(Ordering::SeqCst)
    }

    async fn arm_tool_freeze_gate(&mut self) {
        if let Some(rx) = self.arm_park.take() {
            let _ = rx.await;
        }
    }

    async fn reload_runtime(
        &mut self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        self.reload_runtime_calls.fetch_add(1, Ordering::SeqCst);
        if self.hang_reload_until_cancel.load(Ordering::SeqCst) {
            tokio::select! {
                () = cancel.cancelled() => {}
                () = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
            }
            return Ok(RuntimeReloadReport {
                steps: vec![ReloadStepReport {
                    step: "skills",
                    ok: true,
                    message: "scripted partial before cancel".into(),
                }],
                cancelled: true,
            });
        }
        if self.fail_reload.load(Ordering::SeqCst) {
            return Ok(RuntimeReloadReport {
                steps: vec![ReloadStepReport {
                    step: "mcp",
                    ok: false,
                    message: "scripted fail".into(),
                }],
                cancelled: false,
            });
        }
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
            cancelled: false,
        })
    }

    async fn persist_project_trust(
        &mut self,
        mode: crate::driver::ProjectTrustMode,
    ) -> Result<crate::driver::ProjectTrustPersistReport, XyDriverError> {
        use crate::driver::{ProjectTrustMode, ProjectTrustPersistReport};
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
    ) -> Result<crate::driver::ClipboardCopyOutcome, XyDriverError> {
        self.copy_text_calls
            .lock()
            .expect("copy_text_calls")
            .push(text.to_string());
        let pending_osc52 = self
            .copy_pending_osc52
            .lock()
            .expect("copy_pending_osc52")
            .take();
        Ok(crate::driver::ClipboardCopyOutcome { pending_osc52 })
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

    fn bind_debug_scene(&mut self, load: &crate::driver::DebugSceneLoad) {
        self.session_messages = load.entries.clone();
        self.message_history_tree = xylitol_plugin_sdk::session::build_session_tree(&load.entries);
        *self.active_session_id.lock().expect("sid") = load.session_id.clone();
    }
}

impl ScriptedDriver {
    pub fn set_mcp_catalog_for_driver(&self, catalog: xylitol_plugin_sdk::mcp::McpCatalog) {
        *self.mcp_catalog.lock().expect("mcp_catalog") = Some(catalog);
    }

    pub fn set_mcp_resource_failure_for_driver(&self, server: &str, uri: &str) {
        self.mcp_resource_failures
            .lock()
            .expect("mcp_resource_failures")
            .push((server.to_string(), uri.to_string()));
    }

    pub fn set_mcp_prompt_result_for_driver(&mut self, result: serde_json::Value) {
        self.mcp_prompt_result = Some(result);
    }

    pub fn run_with_context_calls_snapshot(
        &self,
    ) -> Vec<(String, Vec<xylitol_plugin_sdk::mcp::McpResourceContext>)> {
        self.run_with_context_calls
            .lock()
            .expect("run_with_context_calls")
            .clone()
    }

    pub async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        if let Some(err) = &self.get_messages_error {
            return Err(XyDriverError::message(err.clone()));
        }
        Ok(self.session_messages.clone())
    }

    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        if !self.thinking_levels.is_empty()
            && !self
                .thinking_levels
                .iter()
                .any(|supported| supported == &level)
        {
            return Err(format!(
                "thinking level `{}` is not supported by the current model",
                level
            )
            .into());
        }
        self.thinking_level = level;
        Ok(())
    }

    pub fn queue_stats(&self) -> QueueStats {
        let items = self.queue_items();
        QueueStats {
            steer_count: items
                .iter()
                .filter(|i| i.kind == xylitol_plugin_sdk::session::QueueItemKind::Steer)
                .count(),
            follow_up_count: items
                .iter()
                .filter(|i| i.kind == xylitol_plugin_sdk::session::QueueItemKind::FollowUp)
                .count(),
        }
    }

    fn allocate_queue_id(&mut self) -> String {
        let id = format!("q{}", self.next_queue_id);
        self.next_queue_id += 1;
        id
    }

    fn queue_items(&self) -> Vec<xylitol_plugin_sdk::session::QueueItem> {
        if self.force_zero_queue_stats.load(Ordering::SeqCst) {
            return Vec::new();
        }
        let mut items = self.steer_queued.lock().expect("steer_queued").clone();
        items.extend(
            self.follow_up_queued
                .lock()
                .expect("follow_up_queued")
                .iter()
                .cloned(),
        );
        items
    }

    fn revoke_queued_item(&self, id: &str) -> xylitol_plugin_sdk::RevokeQueueItemOutcome {
        let before = self.queue_items().len();
        self.steer_queued
            .lock()
            .expect("steer_queued")
            .retain(|item| item.id != id);
        self.follow_up_queued
            .lock()
            .expect("follow_up_queued")
            .retain(|item| item.id != id);
        let items = self.queue_items();
        xylitol_plugin_sdk::RevokeQueueItemOutcome {
            revoked: items.len() != before,
            items,
        }
    }
}

#[async_trait]
impl crate::dispatch::SessionCommandExecutor for ScriptedDriver {
    async fn execute_session_command(
        &mut self,
        cmd: xylitol_protocol::Command,
    ) -> Result<crate::dispatch::DispatchOutcome, crate::dispatch::DispatchRejection> {
        use crate::dispatch::DispatchOutcome;
        use xylitol_protocol::Command;
        let outcome: Result<DispatchOutcome, XyDriverError> = match cmd {
            Command::Abort { .. } => {
                crate::driver::XyDriver::abort(self);
                Ok(DispatchOutcome::Aborted { cancelled: true })
            }
            Command::GetState { .. } => Ok(DispatchOutcome::State(
                crate::driver::XyDriver::get_state(self),
            )),
            Command::GetAvailableModels { .. } => {
                self.models_calls.fetch_add(1, Ordering::SeqCst);
                Ok(DispatchOutcome::Models(
                    crate::driver::XyDriver::available_models(self),
                ))
            }
            Command::SetModel { model_id, .. } => {
                self.model = ModelInfo {
                    id: model_id.clone(),
                    display_name: model_id,
                    thinking: false,
                    thinking_levels: Vec::new(),
                    context_window: 8_000,
                };
                Ok(DispatchOutcome::Model(self.model.clone()))
            }
            Command::CycleModel { .. } => Ok(DispatchOutcome::Model(self.model.clone())),
            Command::PluginCommand { plugin, args } => Ok(DispatchOutcome::PluginCommandOutput {
                plugin,
                text: format!("plugin command ran: {args}"),
            }),
            Command::SetThinkingLevel { level, .. } => {
                if !self.thinking_levels.is_empty()
                    && !self
                        .thinking_levels
                        .iter()
                        .any(|supported| supported == &level)
                {
                    return Err(XyDriverError::from(format!(
                        "thinking level `{}` is not supported by the current model",
                        level
                    ))
                    .into());
                }
                self.thinking_level = level.clone();
                Ok(DispatchOutcome::ThinkingLevel(level))
            }
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => {
                let run =
                    crate::driver::XyDriver::bash_run(self, &command, exclude_from_context).await?;
                let result = run.await?;
                Ok(DispatchOutcome::Bash(result))
            }
            Command::Compact { instructions, .. } => {
                self.compact_calls
                    .lock()
                    .expect("compact_calls")
                    .push(instructions);
                Ok(DispatchOutcome::Compacted(false))
            }
            Command::GetSessionStats { .. } => {
                let stats = self
                    .session_stats
                    .clone()
                    .ok_or_else(|| XyDriverError::from("scripted: no stats"))?;
                Ok(DispatchOutcome::SessionStats(serde_json::json!({
                    "session_id": stats.session_id,
                    "user_messages": stats.user_messages,
                    "assistant_messages": stats.assistant_messages,
                    "total_messages": stats.total_messages,
                    "thinking_level": stats.thinking_level,
                    "model": stats.model.map(|(p, m)| {
                        serde_json::json!({ "provider": p, "model_id": m })
                    }),
                })))
            }
            Command::ExportHtml { output_path, .. } => {
                let s = output_path.unwrap_or_else(|| "export.html".to_string());
                self.export_html_calls
                    .lock()
                    .expect("export_html_calls")
                    .push(s.clone());
                Ok(DispatchOutcome::ExportedPath(s))
            }
            Command::ExportJsonl { output_path, .. } => {
                let s = output_path.unwrap_or_else(|| "export.jsonl".to_string());
                self.export_jsonl_calls
                    .lock()
                    .expect("export_jsonl_calls")
                    .push(s.clone());
                Ok(DispatchOutcome::ExportedPath(s))
            }
            Command::ImportJsonl { input_path, .. } => {
                self.import_jsonl_calls
                    .lock()
                    .expect("import_jsonl_calls")
                    .push(input_path);
                Ok(DispatchOutcome::NewSession("imported".into()))
            }
            Command::Fork {
                entry_id, position, ..
            } => {
                let pos = match position.as_deref() {
                    Some("before") => xylitol_plugin_sdk::session::ForkPosition::Before,
                    _ => xylitol_plugin_sdk::session::ForkPosition::At,
                };
                self.fork_calls
                    .lock()
                    .expect("fork_calls")
                    .push((entry_id, pos));
                Ok(DispatchOutcome::NewSession("forked-child".into()))
            }
            Command::SwitchSession { session_path, .. } => {
                let id = std::path::Path::new(&session_path)
                    .file_stem()
                    .and_then(|st| st.to_str())
                    .unwrap_or(&session_path)
                    .to_string();
                self.switch_calls
                    .lock()
                    .expect("switch_calls")
                    .push(id.clone());
                *self.active_session_id.lock().expect("sid") = id.clone();
                Ok(DispatchOutcome::SwitchedSession(id))
            }
            Command::GetMessages { .. } => {
                if let Some(err) = &self.get_messages_error {
                    return Err(XyDriverError::message(err.clone()).into());
                }
                let session_id = self.active_session_id.lock().expect("sid").clone();
                Ok(DispatchOutcome::Messages {
                    session_id,
                    entries: self.session_messages.clone(),
                    has_more: false,
                })
            }
            Command::EstimateContext { .. } => {
                let est = crate::driver::XyDriver::estimate_context_tokens(self).await?;
                Ok(DispatchOutcome::EstimateContext(est))
            }
            Command::SessionTree { kind, .. } => match kind {
                xylitol_protocol::SessionTreeKind::MessageHistory => {
                    self.session_tree_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(DispatchOutcome::SessionTree(
                        self.message_history_tree.clone(),
                    ))
                }
                xylitol_protocol::SessionTreeKind::FileBrowser => {
                    Err("scripted: file_browser tree not implemented".into())
                }
            },
            Command::TravelSessionTree { kind, entry_id, .. } => match kind {
                xylitol_protocol::SessionTreeKind::MessageHistory => {
                    self.travel_calls
                        .lock()
                        .expect("travel_calls")
                        .push(entry_id.clone());
                    if let Some(travel) = self.travel_overrides.get(&entry_id) {
                        return Ok(DispatchOutcome::SessionTreeTravel(travel.clone()));
                    }
                    let travel = plan_message_history_travel(&self.session_messages, &entry_id)
                        .map_err(XyDriverError::from)?;
                    Ok(DispatchOutcome::SessionTreeTravel(travel))
                }
                xylitol_protocol::SessionTreeKind::FileBrowser => {
                    Err("scripted: file_browser travel not implemented".into())
                }
            },
            Command::AppendEntryLabel {
                target_id, label, ..
            } => {
                let cleaned = label
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                self.label_calls
                    .lock()
                    .expect("label_calls")
                    .push((target_id, cleaned));
                Ok(DispatchOutcome::Empty)
            }
            Command::ListSessions { .. } => {
                self.list_sessions_calls.fetch_add(1, Ordering::SeqCst);
                Ok(DispatchOutcome::Sessions(
                    self.session_list.lock().expect("session_list").clone(),
                ))
            }
            Command::LoadSessionEntries { .. } => Ok(DispatchOutcome::SessionEntries(
                self.session_messages.clone(),
            )),
            Command::NewSession { .. } => {
                self.new_session_calls.fetch_add(1, Ordering::SeqCst);
                let sid = format!("new-{}", self.new_session_calls());
                *self.active_session_id.lock().expect("sid") = sid.clone();
                self.session_messages.clear();
                *self.leaf_entry_id.lock().expect("leaf") = None;
                *self.session_name.lock().expect("session_name") = None;
                Ok(DispatchOutcome::NewSession(sid))
            }
            Command::GetSessionName { .. } => Ok(DispatchOutcome::SessionName(
                self.session_name.lock().expect("session_name").clone(),
            )),
            Command::SetSessionName { name, .. } => {
                let stored = xylitol_plugin_sdk::sanitize_session_display_name(&name);
                self.set_session_name_calls
                    .lock()
                    .expect("set_session_name_calls")
                    .push(name);
                *self.session_name.lock().expect("session_name") = Some(stored.clone());
                Ok(DispatchOutcome::SessionName(Some(stored)))
            }
            Command::GenerateSessionName { force, .. } => {
                let _ = force;
                self.generate_session_name_calls
                    .fetch_add(1, Ordering::SeqCst);
                let name = self
                    .session_name
                    .lock()
                    .expect("session_name")
                    .clone()
                    .unwrap_or_else(|| "生成阶段名".to_string());
                *self.session_name.lock().expect("session_name") = Some(name.clone());
                Ok(DispatchOutcome::SessionName(Some(name)))
            }
            Command::SetSessionNameFor {
                session_id, name, ..
            } => {
                let stored = xylitol_plugin_sdk::sanitize_session_display_name(&name);
                self.set_session_name_for_calls
                    .lock()
                    .expect("set_session_name_for_calls")
                    .push((session_id.clone(), name));
                if let Some(entry) = self
                    .session_list
                    .lock()
                    .expect("session_list")
                    .iter_mut()
                    .find(|e| e.id == session_id)
                {
                    entry.name = Some(stored.clone());
                }
                Ok(DispatchOutcome::SessionName(Some(stored)))
            }
            Command::DeleteSession { session_id, .. } => {
                self.delete_session_calls
                    .lock()
                    .expect("delete_session_calls")
                    .push(session_id.clone());
                self.session_list
                    .lock()
                    .expect("session_list")
                    .retain(|e| e.id != session_id);
                Ok(DispatchOutcome::Empty)
            }
            Command::SessionDetach { .. } => {
                self.detach_session().await?;
                Ok(DispatchOutcome::Empty)
            }
            Command::Reload { .. } => Ok(DispatchOutcome::Reload(
                crate::driver::XyDriver::reload_runtime(
                    self,
                    &tokio_util::sync::CancellationToken::new(),
                )
                .await?,
            )),
            Command::LoadedResources { .. } => Ok(DispatchOutcome::LoadedResources(
                crate::driver::XyDriver::loaded_resources_snapshot(self).await,
            )),
            Command::GetQueueStats { .. } => Ok(DispatchOutcome::QueueItems(self.queue_items())),
            Command::GetCommands { .. } => Ok(DispatchOutcome::Commands(
                crate::driver::XyDriver::get_commands(self),
            )),
            Command::GetCapabilities { .. } => Ok(DispatchOutcome::Capabilities(
                crate::driver::XyDriver::capability_snapshot(self),
            )),
            Command::SetCapability { id, enabled } => {
                crate::driver::XyDriver::set_capability(self, &id, enabled)
                    .await
                    .map(DispatchOutcome::Capabilities)
            }
            Command::ClearCapabilities { .. } => crate::driver::XyDriver::clear_capabilities(self)
                .await
                .map(DispatchOutcome::Capabilities),
            Command::Steer { message, .. } => {
                self.steer_calls.push(message.clone());
                let id = self.allocate_queue_id();
                self.steer_queued.lock().expect("steer_queued").push(
                    xylitol_plugin_sdk::session::QueueItem {
                        id: id.clone(),
                        kind: xylitol_plugin_sdk::session::QueueItemKind::Steer,
                        preview: xylitol_plugin_sdk::session::QueueItem::preview_of(&message),
                    },
                );
                Ok(DispatchOutcome::QueueItemEnqueued {
                    id,
                    items: self.queue_items(),
                })
            }
            Command::FollowUp { message, .. } => {
                self.follow_up_calls.push(message.clone());
                let id = self.allocate_queue_id();
                self.follow_up_queued
                    .lock()
                    .expect("follow_up_queued")
                    .push(xylitol_plugin_sdk::session::QueueItem {
                        id: id.clone(),
                        kind: xylitol_plugin_sdk::session::QueueItemKind::FollowUp,
                        preview: xylitol_plugin_sdk::session::QueueItem::preview_of(&message),
                    });
                Ok(DispatchOutcome::QueueItemEnqueued {
                    id,
                    items: self.queue_items(),
                })
            }
            Command::RevokeQueueItem { id } => {
                let outcome = self.revoke_queued_item(&id);
                let revoked = outcome.revoked;
                if !revoked {
                    return Err(crate::dispatch::DispatchRejection::NotFound {
                        message: format!("queue item not found: {id}"),
                        source_kind: Some("NotFound"),
                    });
                }
                Ok(DispatchOutcome::RevokedQueueItem(outcome))
            }
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => {
                self.clear_calls.push((clear_steer, clear_follow_up));
                if clear_steer {
                    self.steer_queued.lock().expect("steer_queued").clear();
                }
                if clear_follow_up {
                    self.follow_up_queued
                        .lock()
                        .expect("follow_up_queued")
                        .clear();
                }
                Ok(DispatchOutcome::QueueItems(self.queue_items()))
            }

            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. }
            | Command::McpCatalog { .. }
            | Command::McpResourceRead { .. }
            | Command::McpPromptGet { .. }
            | Command::McpOAuthStart { .. }
            | Command::McpOAuthCancel { .. }
            | Command::McpOAuthStatus { .. } => {
                Err(crate::dispatch::transport_variant_error(&cmd).into())
            }
        };
        outcome.map_err(crate::dispatch::DispatchRejection::from)
    }
}
