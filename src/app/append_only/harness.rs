//! ScriptedDriver harness for append-only (atao5/atao6/atao7).

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use crate::app::append_only::host::pump_scripted;
use crate::app::append_only::keys::is_exit_slash;
use crate::app::append_only::{AppendOnlyModel, BlockKind};
use crate::app::core::driver::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, ModelInfo, QueueStats,
    SessionListEntry, SessionStats, XyDriver, XyDriverError, XyEvent,
};
use crate::protocol::model::THINKING_OFF;
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};

/// Minimal ScriptedDriver for append-only surface harness.
pub struct ScriptedDriver {
    pub runs: Vec<String>,
    abort_count: AtomicUsize,
    scripts: VecDeque<Vec<XyEvent>>,
    model: ModelInfo,
    session_id: String,
}

impl ScriptedDriver {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            abort_count: AtomicUsize::new(0),
            scripts: VecDeque::new(),
            model: ModelInfo {
                id: "fake".into(),
                display_name: "Fake".into(),
                thinking: false,
                thinking_levels: Vec::new(),
                context_window: 8_000,
            },
            session_id: "append-scripted".into(),
        }
    }

    pub fn push_script(&mut self, events: Vec<XyEvent>) {
        self.scripts.push_back(events);
    }

    pub fn abort_count(&self) -> usize {
        self.abort_count.load(Ordering::SeqCst)
    }
}

fn unsupported<T>() -> Result<T, XyDriverError> {
    Err(XyDriverError::message("not used by append-only harness"))
}

#[async_trait]
impl XyDriver for ScriptedDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.runs.push(prompt.to_string());
        let events = self.scripts.pop_front().unwrap_or_else(|| {
            vec![
                XyEvent::AgentStart {
                    session_id: self.session_id.clone(),
                    model: self.model.id.clone(),
                },
                XyEvent::TextDelta("ok".into()),
                XyEvent::AgentEnd {
                    messages: Vec::new(),
                },
            ]
        });
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

    fn select_model(&mut self, _model_id: &str) -> Result<ModelInfo, XyDriverError> {
        Ok(self.model.clone())
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        Ok(self.model.clone())
    }

    fn set_thinking_level(&mut self, _level: String) -> Result<(), XyDriverError> {
        Ok(())
    }

    fn thinking_level(&self) -> String {
        THINKING_OFF.into()
    }

    fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
        Ok(THINKING_OFF.into())
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn execute_bash(
        &self,
        _command: &str,
        _exclude_from_context: bool,
        _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        unsupported()
    }

    async fn compact(&mut self, _instructions: Option<String>) -> Result<bool, XyDriverError> {
        unsupported()
    }

    async fn export_html(&mut self, _path: &Path) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn export_jsonl(&mut self, _path: &Path) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn import_jsonl(&mut self, _path: &Path) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn fork_session(
        &mut self,
        _entry_id: &str,
        _position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn switch_session(&mut self, _session_id: &str) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        Ok(Vec::new())
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        unsupported()
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        unsupported()
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        // Closed set: no slash catalog beyond /exit (handled locally).
        Vec::new()
    }

    fn steer(&mut self, _message: &str) -> Result<(), XyDriverError> {
        unsupported()
    }

    fn follow_up(&mut self, _message: &str) -> Result<(), XyDriverError> {
        unsupported()
    }

    fn clear_queue(
        &mut self,
        _clear_steer: bool,
        _clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        Ok(())
    }

    fn queue_stats(&self) -> QueueStats {
        QueueStats::default()
    }

    async fn session_tree(
        &self,
        _kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        Ok(Vec::new())
    }

    async fn travel_session_tree(
        &self,
        _kind: SessionTreeKind,
        _entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        unsupported()
    }

    async fn append_entry_label(
        &mut self,
        _target_id: &str,
        _label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        Ok(())
    }

    fn leaf_entry_id(&self) -> Option<String> {
        None
    }

    async fn load_debug_scene(&mut self, _scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        unsupported()
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        Ok(Vec::new())
    }

    async fn load_session_entries(
        &self,
        _session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        Ok(Vec::new())
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        Ok(None)
    }

    async fn set_session_name(&mut self, _name: &str) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn set_session_name_for(
        &mut self,
        _session_id: &str,
        _name: &str,
    ) -> Result<String, XyDriverError> {
        unsupported()
    }

    async fn delete_session(&mut self, _session_id: &str) -> Result<(), XyDriverError> {
        unsupported()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        LoadedResourcesSnapshot::default()
    }
}

#[tokio::test]
async fn atao7_scripted_stream_tool_spill_abort_exit() {
    let tmp = tempfile::tempdir().unwrap();
    let spill = tmp.path().join("append-scripted.spill");
    std::fs::create_dir_all(&spill).unwrap();

    let mut driver = ScriptedDriver::new();
    let long = (0..10)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    driver.push_script(vec![
        XyEvent::AgentStart {
            session_id: "append-scripted".into(),
            model: "fake".into(),
        },
        XyEvent::TextDelta(format!("{long}\n")),
        XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "bash".into(),
            result: format!("{long}\n"),
            is_error: false,
        },
        XyEvent::AgentEnd {
            messages: Vec::new(),
        },
    ]);

    let mut model = AppendOnlyModel::new(Some(spill.clone()));
    pump_scripted(&mut model, &mut driver, "hello")
        .await
        .unwrap();

    assert_eq!(driver.runs, vec!["hello".to_string()]);
    assert!(!model.any_expandable());
    assert!(
        model
            .blocks
            .iter()
            .any(|b| b.visible.contains("[Full output:")),
        "spill footer expected: {:?}",
        model.blocks
    );
    assert!(
        model.blocks.iter().any(|b| b
            .full_output_path
            .as_ref()
            .is_some_and(|p| p.starts_with(&spill))),
        "spill path under session spill dir"
    );

    // abort path
    driver.abort();
    assert_eq!(driver.abort_count(), 1);

    // exit slash (local closed set)
    assert!(is_exit_slash("/exit"));
    model.quit = is_exit_slash("/exit");
    assert!(model.quit);

    // write/diff uses taller cap
    model
        .commit_capped(
            BlockKind::WriteDiff,
            "write",
            &(0..8)
                .map(|i| format!("w{i}"))
                .collect::<Vec<_>>()
                .join("\n"),
            "write",
        )
        .unwrap();
    let write_block = model.blocks.last().unwrap();
    assert_eq!(write_block.kind, BlockKind::WriteDiff);
    assert!(write_block.visible.contains("[Full output:"));
}
