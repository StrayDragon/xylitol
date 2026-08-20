//! Remote HTTP/WS [`XyRemoteDriver`] (feature = "server").

use std::path::Path;

use async_trait::async_trait;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::app::core::host_client::{HostClient, HttpWsClient};
use crate::protocol::model::THINKING_OFF;
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};
use crate::protocol::{Event, RpcMessage};

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, ModelInfo, SessionListEntry,
    SessionStats, XyEvent, estimate_from_session_entries, session_tree_kind_unimplemented,
};

/// Remote driver — [`XyDriver`] over [`HttpWsClient`] (HTTP POST unary + WS downlink).
#[cfg(feature = "server")]
pub struct XyRemoteDriver {
    host: HttpWsClient,
    session_id: String,
    cancel: CancellationToken,
    thinking: std::sync::Mutex<String>,
}

#[cfg(feature = "server")]
impl XyRemoteDriver {
    /// Create a new XyRemoteDriver connected to `base_url`.
    ///
    /// `base_url` should be the server root, e.g. `http://127.0.0.1:18790`.
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            host: HttpWsClient::new(base_url),
            session_id: session_id.into(),
            cancel: CancellationToken::new(),
            thinking: std::sync::Mutex::new(THINKING_OFF.into()),
        }
    }

    async fn unary(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, XyDriverError> {
        let result = self
            .host
            .unary(method, payload)
            .await
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        result
            .into_std()
            .map_err(|e| XyDriverError::remote(format!("{}: {}", e.code, e.details)))
    }

    fn block_on<T>(
        &self,
        fut: impl std::future::Future<Output = Result<T, XyDriverError>>,
    ) -> Result<T, XyDriverError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
            Err(_) => tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| XyDriverError::io(e.to_string()))?
                .block_on(fut),
        }
    }

    fn model_from_value(v: &serde_json::Value) -> Result<ModelInfo, XyDriverError> {
        let thinking = v.get("thinking").and_then(|x| x.as_bool()).unwrap_or(false);
        let thinking_levels = v
            .get("thinking_levels")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_else(|| vec![THINKING_OFF.into()]);
        Ok(ModelInfo {
            id: v
                .get("id")
                .or_else(|| v.get("model"))
                .and_then(|x| x.as_str())
                .ok_or("missing model id")?
                .to_string(),
            display_name: v
                .get("display_name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            thinking,
            thinking_levels,
            context_window: v
                .get("context_window")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
        })
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl XyDriver for XyRemoteDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let host = self.host.clone();
        let cancel = self.cancel.clone();
        let prompt = prompt.to_string();

        let stream = async_stream::stream! {
            let mut mux = match host.mux().await {
                Ok(s) => s,
                Err(e) => {
                    yield XyEvent::error_msg(format!("WS connect failed: {e}"));
                    return;
                }
            };

            if let Err(e) = host
                .unary("prompt", serde_json::json!({"message": prompt}))
                .await
            {
                yield XyEvent::error_msg(format!("prompt failed: {e}"));
                return;
            }

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = host.unary("abort", serde_json::json!({})).await;
                        break;
                    }
                    msg = mux.next() => {
                        match msg {
                            Some(Ok(RpcMessage::ServerRequest { method, payload, .. }))
                                if method == "session/event" =>
                            {
                                let event_val =
                                    payload.get("event").cloned().unwrap_or(payload);
                                let Ok(ev) = serde_json::from_value::<Event>(event_val) else {
                                    continue;
                                };
                                let Ok(agent_event) = XyEvent::try_from(&ev) else {
                                    continue;
                                };
                                let is_end =
                                    matches!(agent_event, XyEvent::AgentEnd { .. });
                                yield agent_event;
                                if is_end {
                                    break;
                                }
                            }
                            Some(Ok(RpcMessage::ServerRequest { method, .. }))
                                if method == "session/resync_required" =>
                            {
                                yield XyEvent::error_msg("journal truncated, resync required");
                                break;
                            }
                            Some(Ok(_)) => {}
                            Some(Err(e)) => {
                                yield XyEvent::error_msg(e.to_string());
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        self.cancel.cancel();
        let host = self.host.clone();
        tokio::spawn(async move {
            let _ = host.unary("abort", serde_json::json!({})).await;
        });
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.block_on(async {
            let data = self.unary("get_state", serde_json::json!({})).await?;
            match data.get("model") {
                Some(m) if !m.is_null() => Self::model_from_value(m).map(Some),
                _ => Ok(None),
            }
        })
        .ok()
        .flatten()
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.block_on(async {
            let data = self
                .unary("get_available_models", serde_json::json!({}))
                .await?;
            let arr = data
                .get("models")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            arr.iter().map(Self::model_from_value).collect()
        })
        .unwrap_or_default()
    }

    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        let model_id = model_id.to_string();
        let data = self
            .unary(
                "set_model",
                serde_json::json!({ "provider": "", "model_id": model_id }),
            )
            .await?;
        // Endpoint returns { model, display_name }; enrich via list if needed.
        let selected = if data.get("id").is_some() {
            Self::model_from_value(&data)?
        } else {
            ModelInfo {
                id: data
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or(&model_id)
                    .to_string(),
                display_name: data
                    .get("display_name")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                thinking: false,
                thinking_levels: Vec::new(),
                context_window: 0,
            }
        };
        let default = selected
            .thinking_levels
            .last()
            .cloned()
            .unwrap_or_else(|| THINKING_OFF.into());
        *self.thinking.lock().unwrap() = default;
        Ok(selected)
    }

    async fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        let data = self.unary("cycle_model", serde_json::json!({})).await?;
        let selected = Self::model_from_value(&data)?;
        *self.thinking.lock().unwrap() = selected
            .thinking_levels
            .last()
            .cloned()
            .unwrap_or_else(|| THINKING_OFF.into());
        Ok(selected)
    }

    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.unary("set_thinking_level", serde_json::json!({ "level": level }))
            .await?;
        *self.thinking.lock().unwrap() = level;
        Ok(())
    }

    fn thinking_level(&self) -> String {
        self.thinking.lock().unwrap().clone()
    }

    async fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
        // Remote REST has set-only; cycle locally over the selected model's
        // declared support list.
        let levels = self
            .current_model()
            .map(|model| model.thinking_levels)
            .filter(|levels| !levels.is_empty())
            .unwrap_or_else(|| vec![THINKING_OFF.into()]);
        let cur = self.thinking_level();
        let next = match levels.iter().position(|level| level == &cur) {
            Some(index) => levels[(index + 1) % levels.len()].clone(),
            None => levels
                .last()
                .cloned()
                .unwrap_or_else(|| THINKING_OFF.into()),
        };
        self.set_thinking_level(next.clone()).await?;
        Ok(next)
    }

    fn session_id(&self) -> Option<String> {
        Some(self.session_id.clone())
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        _chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        // Remote REST bash is request/response — no live chunk uplink.
        let data = self
            .unary(
                "bash",
                serde_json::json!({
                    "command": command,
                    "exclude_from_context": exclude_from_context,
                }),
            )
            .await?;
        Ok(XyBashResult {
            output: data
                .get("output")
                .and_then(|o| o.as_str())
                .unwrap_or("")
                .to_string(),
            exit_code: data
                .get("exit_code")
                .and_then(|c| c.as_i64())
                .map(|c| c as i32),
            cancelled: data
                .get("cancelled")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            timed_out: data
                .get("timed_out")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            truncated: data
                .get("truncated")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            full_output_path: None,
        })
    }

    async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        let data = self
            .unary(
                "compact",
                serde_json::json!({ "instructions": instructions }),
            )
            .await?;
        Ok(data
            .get("compacted")
            .and_then(|c| c.as_bool())
            .unwrap_or(false))
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self.unary("export_html", serde_json::json!({})).await?;
        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
            std::fs::write(path, content).map_err(|e| XyDriverError::io(e.to_string()))?;
            return Ok(path.to_string_lossy().into_owned());
        }
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self.unary("export_jsonl", serde_json::json!({})).await?;
        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
            std::fs::write(path, content).map_err(|e| XyDriverError::io(e.to_string()))?;
            return Ok(path.to_string_lossy().into_owned());
        }
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| XyDriverError::io(e.to_string()))?;
        let data = self
            .unary("import_jsonl", serde_json::json!({ "content": content }))
            .await?;
        Ok(data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        let data = self
            .unary(
                "fork",
                serde_json::json!({
                    "entry_id": entry_id,
                    "position": match position {
                        crate::protocol::session::ForkPosition::At => "at",
                        crate::protocol::session::ForkPosition::Before => "before",
                    },
                }),
            )
            .await?;
        Ok(data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        let data = self
            .unary(
                "switch_session",
                serde_json::json!({ "session_id": session_id }),
            )
            .await?;
        let id = data
            .get("session_id")
            .and_then(|p| p.as_str())
            .unwrap_or(session_id)
            .to_string();
        self.session_id = id.clone();
        Ok(id)
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let data = self.unary("get_messages", serde_json::json!({})).await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(entries).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        let data = self
            .unary("get_session_stats", serde_json::json!({}))
            .await?;
        Ok(SessionStats {
            session_id: data
                .get("session_id")
                .and_then(|s| s.as_str())
                .unwrap_or(&self.session_id)
                .to_string(),
            user_messages: data
                .get("user_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            assistant_messages: data
                .get("assistant_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            total_messages: data
                .get("total_messages")
                .and_then(|n| n.as_u64())
                .unwrap_or(0) as usize,
            thinking_level: data
                .get("thinking_level")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            model: data.get("model").and_then(|m| {
                Some((
                    m.get("provider")?.as_str()?.to_string(),
                    m.get("model_id")?.as_str()?.to_string(),
                ))
            }),
        })
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        let entries = self.get_messages().await.unwrap_or_default();
        // Remote surface: tokenizer mapping lives on the server; do not inject
        // local AppConfig override here.
        Ok(estimate_from_session_entries(
            &entries,
            self.current_model().map(|m| m.id),
            None,
        ))
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.block_on(async {
            let data = self.unary("get_commands", serde_json::json!({})).await?;
            let arr = data
                .get("commands")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(arr
                .iter()
                .filter_map(|c| {
                    Some(CommandInfo {
                        name: c.get("name")?.as_str()?.to_string(),
                        description: c
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect())
        })
        .unwrap_or_default()
    }

    fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.block_on(async {
            self.unary("steer", serde_json::json!({ "message": message }))
                .await?;
            Ok(())
        })
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.block_on(async {
            self.unary("follow_up", serde_json::json!({ "message": message }))
                .await?;
            Ok(())
        })
    }

    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.block_on(async {
            self.unary(
                "clear_queue",
                serde_json::json!({
                    "clear_steer": clear_steer,
                    "clear_follow_up": clear_follow_up,
                }),
            )
            .await?;
            Ok(())
        })
    }

    fn queue_stats(&self) -> crate::agent::capabilities::QueueStats {
        crate::agent::capabilities::QueueStats::default()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
            kind,
        )))
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let _ = entry_id;
        Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
            kind,
        )))
    }

    async fn append_entry_label(
        &mut self,
        _target_id: &str,
        _label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: append_entry_label not implemented",
        ))
    }

    fn leaf_entry_id(&self) -> Option<String> {
        None
    }

    async fn load_debug_scene(&mut self, _scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: load_debug_scene not implemented",
        ))
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "list_sessions is not a v1 unary method",
        ))
    }

    async fn load_session_entries(
        &self,
        _session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: load_session_entries not implemented",
        ))
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: new_session not implemented",
        ))
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: get_session_name not implemented",
        ))
    }

    async fn set_session_name(&mut self, _name: &str) -> Result<String, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: set_session_name not implemented",
        ))
    }

    async fn set_session_name_for(
        &mut self,
        _session_id: &str,
        _name: &str,
    ) -> Result<String, XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: set_session_name_for not implemented",
        ))
    }

    async fn delete_session(&mut self, _session_id: &str) -> Result<(), XyDriverError> {
        Err(XyDriverError::unsupported(
            "remote: delete_session not implemented",
        ))
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        LoadedResourcesSnapshot::default()
    }
}
