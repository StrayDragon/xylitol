//! Remote HTTP/WS [`XyRemoteDriver`] (feature = "server").

use std::path::Path;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

use crate::app::server::ws::{ClientFrame, ServerFrame};
use crate::protocol::ports::XyBashResult;
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};
use crate::protocol::types::ThinkingLevel;

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, ModelInfo, SessionListEntry,
    SessionStats, XyEvent, estimate_from_session_entries, session_tree_kind_unimplemented,
};

/// Remote driver — speaks protocol over REST/WS to a xylitol server.
///
/// Uses `reqwest` for control commands (prompt, abort, model, export, ...) and
/// `tokio-tungstenite` for WebSocket event streaming.
///
/// 预留：独立远程薄端客户端接线后由该面 `XyRemoteDriver::new` 实例化；
/// 落地条件：远程客户端应用面开闸。当前 Server 面用进程内 XyDriver，不构造本类型。
#[cfg(feature = "server")]
#[allow(dead_code)] // reserved remote thin-client surface; see doc above
pub struct XyRemoteDriver {
    base_url: String,
    session_id: String,
    client: reqwest::Client,
    cancel: CancellationToken,
    /// Cached thinking level (server does not expose a getter; tracked locally
    /// so get_state returns something sensible). NOTE: ceiling: server gains a
    /// state endpoint. upgrade: when server exposes GET /state.
    thinking: std::sync::Mutex<ThinkingLevel>,
}

#[cfg(feature = "server")]
#[allow(dead_code)] // reserved with XyRemoteDriver until thin client wires it
impl XyRemoteDriver {
    /// Create a new XyRemoteDriver connected to `base_url`.
    ///
    /// `base_url` should be the server root, e.g. `http://127.0.0.1:8080`.
    pub fn new(base_url: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            session_id: session_id.into(),
            client: reqwest::Client::new(),
            cancel: CancellationToken::new(),
            thinking: std::sync::Mutex::new(ThinkingLevel::Medium),
        }
    }

    fn run_url(&self) -> String {
        format!("{}/api/v1/session/{}/run", self.base_url, self.session_id)
    }

    fn cancel_url(&self) -> String {
        format!("{}/api/v1/session/{}", self.base_url, self.session_id)
    }

    fn ws_url(&self) -> String {
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{ws_base}/api/v1/session/{}/ws", self.session_id)
    }

    fn api(&self, suffix: &str) -> String {
        format!(
            "{}/api/v1/session/{}/{}",
            self.base_url, self.session_id, suffix
        )
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

    async fn get_data(&self, suffix: &str) -> Result<serde_json::Value, XyDriverError> {
        let resp = self
            .client
            .get(self.api(suffix))
            .send()
            .await
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        Self::parse_envelope(resp).await
    }

    async fn post_data(
        &self,
        suffix: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, XyDriverError> {
        let resp = self
            .client
            .post(self.api(suffix))
            .json(&body)
            .send()
            .await
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        Self::parse_envelope(resp).await
    }

    async fn parse_envelope(resp: reqwest::Response) -> Result<serde_json::Value, XyDriverError> {
        let status = resp.status();
        let env: crate::protocol::Envelope<serde_json::Value> = resp
            .json()
            .await
            .map_err(|e| XyDriverError::remote(e.to_string()))?;
        if env.code != crate::protocol::ErrorCode::Ok {
            let err = XyDriverError::remote(
                env.msg
                    .unwrap_or_else(|| format!("server error ({status})")),
            );
            err.log_failure("remote.parse_envelope");
            return Err(err);
        }
        Ok(env.data.unwrap_or(serde_json::Value::Null))
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
            .unwrap_or_else(|| {
                if thinking {
                    ThinkingLevel::STANDARD
                        .iter()
                        .map(|l| l.as_str().to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            });
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
        let client = self.client.clone();
        let run_url = self.run_url();
        let cancel_url = self.cancel_url();
        let ws_url = self.ws_url();
        let cancel = self.cancel.clone();
        let prompt = prompt.to_string();

        let stream = async_stream::stream! {
            // 1. Submit prompt via REST (triggers agent execution)
            let payload = serde_json::json!({"prompt": prompt});
            match client.post(&run_url).json(&payload).send().await {
                Ok(resp) if !resp.status().is_success() => {
                    let status = resp.status();
                    yield XyEvent::Error(format!("server returned {status}"));
                    return;
                }
                Err(e) => {
                    yield XyEvent::Error(format!("connection failed: {e}"));
                    return;
                }
                _ => {} // success
            }

            // 2. Connect to WS for event streaming
            let ws_stream = match connect_async(&ws_url).await {
                Ok((ws, _)) => ws,
                Err(e) => {
                    yield XyEvent::Error(format!("WS connect failed: {e}"));
                    return;
                }
            };

            let (mut ws_writer, mut ws_reader) = ws_stream.split();

            // 3. Send Subscribe
            let subscribe = serde_json::to_string(&ClientFrame::Subscribe {
                session_id: String::new(), // server knows from URL
                last_seq: 0,
            })
            .unwrap();
            if ws_writer.send(Message::Text(subscribe.into())).await.is_err() {
                yield XyEvent::Error("WS send failed".into());
                return;
            }

            // 4. Read events until AgentEnd or abort
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        let _ = client.delete(&cancel_url).send().await;
                        break;
                    }
                    msg = ws_reader.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(frame) = serde_json::from_str::<ServerFrame>(&text) {
                                    match frame {
                                        ServerFrame::ServerHello { .. } | ServerFrame::Ack { .. } => {
                                            // Handshake frames, ignore
                                        }
                                        ServerFrame::Event { event, .. } => {
                                            if let Ok(agent_event) = XyEvent::try_from(&event) {
                                                let is_end = matches!(agent_event, XyEvent::AgentEnd { .. });
                                                yield agent_event;
                                                if is_end {
                                                    break;
                                                }
                                            }
                                        }
                                        ServerFrame::ResyncRequired { .. } => {
                                            yield XyEvent::Error("journal truncated, resync required".into());
                                            break;
                                        }
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        };

        Box::pin(stream)
    }

    fn abort(&self) {
        self.cancel.cancel();
        let url = self.cancel_url();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.delete(&url).send().await;
        });
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.block_on(async {
            let data = self.get_data("state").await?;
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
            let data = self.get_data("models").await?;
            let arr = data
                .get("models")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            arr.iter().map(Self::model_from_value).collect()
        })
        .unwrap_or_default()
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        let model_id = model_id.to_string();
        let url = format!(
            "{}/api/v1/session/{}/model?model_id={}",
            self.base_url,
            self.session_id,
            urlencoding_loose(&model_id)
        );
        self.block_on(async {
            let resp = self
                .client
                .post(&url)
                .send()
                .await
                .map_err(|e| XyDriverError::remote(e.to_string()))?;
            let data = Self::parse_envelope(resp).await?;
            // Endpoint returns { model, display_name }; enrich via list if needed.
            if data.get("id").is_some() {
                Self::model_from_value(&data)
            } else {
                Ok(ModelInfo {
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
                })
            }
        })
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        self.block_on(async {
            let data = self.post_data("model/cycle", serde_json::json!({})).await?;
            Self::model_from_value(&data)
        })
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyDriverError> {
        *self.thinking.lock().unwrap() = level;
        let level_str = level.as_str();
        self.block_on(async {
            self.post_data("thinking", serde_json::json!({ "level": level_str }))
                .await
        })?;
        Ok(())
    }

    fn thinking_level(&self) -> ThinkingLevel {
        *self.thinking.lock().unwrap()
    }

    fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyDriverError> {
        // Remote REST has set-only; cycle locally over STANDARD then POST.
        let levels = ThinkingLevel::STANDARD;
        let cur = self.thinking_level();
        let idx = levels.iter().position(|l| *l == cur).unwrap_or(0);
        let next = levels[(idx + 1) % levels.len()];
        self.set_thinking_level(next)?;
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
            .post_data(
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

    async fn compact(&mut self) -> Result<bool, XyDriverError> {
        let data = self.post_data("compact", serde_json::json!({})).await?;
        Ok(data
            .get("compacted")
            .and_then(|c| c.as_bool())
            .unwrap_or(false))
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self
            .post_data(
                "export/html",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
            .await?;
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self
            .post_data(
                "export/jsonl",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
            .await?;
        Ok(data
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let data = self
            .post_data(
                "import/jsonl",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
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
            .post_data(
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
            .post_data("switch", serde_json::json!({ "session_id": session_id }))
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
        let data = self.get_data("messages").await?;
        let entries = data
            .get("entries")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(entries).map_err(|e| XyDriverError::remote(e.to_string()))
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        let data = self.get_data("stats").await?;
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
    ) -> Result<crate::protocol::types::ContextTokenEstimate, XyDriverError> {
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
            let data = self.get_data("commands").await?;
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
            self.post_data("steer", serde_json::json!({ "message": message }))
                .await?;
            Ok(())
        })
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.block_on(async {
            self.post_data("follow-up", serde_json::json!({ "message": message }))
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
            self.post_data(
                "queue/clear",
                serde_json::json!({
                    "clear_steer": clear_steer,
                    "clear_follow_up": clear_follow_up,
                }),
            )
            .await?;
            Ok(())
        })
    }

    fn queue_stats(&self) -> crate::agent::session::QueueStats {
        self.block_on(async {
            let data = self.get_data("queue").await?;
            Ok(crate::agent::session::QueueStats {
                steer_count: data
                    .get("steer_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize,
                follow_up_count: data
                    .get("follow_up_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize,
            })
        })
        .unwrap_or_default()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        match kind {
            SessionTreeKind::MessageHistory => {
                let data = self.get_data("trees/message-history").await?;
                let tree = data.get("tree").cloned().unwrap_or(serde_json::Value::Null);
                serde_json::from_value(tree).map_err(|e| XyDriverError::remote(e.to_string()))
            }
            SessionTreeKind::FileBrowser => Err(XyDriverError::unsupported(
                session_tree_kind_unimplemented(kind),
            )),
        }
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        match kind {
            SessionTreeKind::MessageHistory => {
                let data = self
                    .post_data(
                        "trees/message-history/travel",
                        serde_json::json!({ "entry_id": entry_id }),
                    )
                    .await?;
                serde_json::from_value(data).map_err(|e| XyDriverError::remote(e.to_string()))
            }
            SessionTreeKind::FileBrowser => Err(XyDriverError::unsupported(
                session_tree_kind_unimplemented(kind),
            )),
        }
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
        self.block_on(async {
            let data = self.get_data("sessions").await?;
            let arr = data
                .get("sessions")
                .and_then(|s| s.as_array())
                .cloned()
                .unwrap_or_default();
            Ok(arr
                .iter()
                .filter_map(|row| {
                    Some(SessionListEntry {
                        id: row.get("id")?.as_str()?.to_string(),
                        name: row
                            .get("name")
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        first_message: row
                            .get("first_message")
                            .or_else(|| row.get("firstMessage"))
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        message_count: row
                            .get("message_count")
                            .or_else(|| row.get("messageCount"))
                            .and_then(|n| n.as_u64())
                            .unwrap_or(0) as usize,
                        modified_unix: row
                            .get("modified_unix")
                            .or_else(|| row.get("modified"))
                            .and_then(|n| n.as_u64()),
                        parent_session_id: row
                            .get("parent_session_id")
                            .or_else(|| row.get("parentSession"))
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        tree_prefix: String::new(),
                        cwd: row
                            .get("cwd")
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        path: row
                            .get("path")
                            .and_then(|n| n.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                    })
                })
                .collect())
        })
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

#[cfg(feature = "server")]
#[allow(dead_code)] // used by reserved XyRemoteDriver; keep while thin-client surface is dormant
fn urlencoding_loose(s: &str) -> String {
    s.replace(' ', "%20")
}
