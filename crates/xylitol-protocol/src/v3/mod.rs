pub mod generated;
pub use generated::*;

impl Clone for SessionTreeNode {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry.clone(),
            children: self.children.clone(),
            label: self.label.clone(),
        }
    }
}

impl PartialEq for SessionTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry && self.children == other.children && self.label == other.label
    }
}

impl Eq for SessionTreeNode {}

impl Clone for TreeResult {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
        }
    }
}

impl PartialEq for TreeResult {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
    }
}

impl Eq for TreeResult {}

impl PartialEq for ResponsePayload {
    fn eq(&self, other: &Self) -> bool {
        self.debug_repr() == other.debug_repr()
    }
}

impl Eq for ResponsePayload {}

impl PartialEq for Frame {
    fn eq(&self, other: &Self) -> bool {
        self.debug_repr() == other.debug_repr()
    }
}

impl Eq for Frame {}

trait DebugRepr {
    fn debug_repr(&self) -> String;
}

impl DebugRepr for ResponsePayload {
    fn debug_repr(&self) -> String {
        format!("{self:?}")
    }
}

impl Frame {
    fn kind_name(&self) -> &'static str {
        match self {
            Frame::ClientRequest(_) => "ClientRequest",
            Frame::ServerResponse(_) => "ServerResponse",
            Frame::ServerNotification(_) => "ServerNotification",
            Frame::Unknown(_) => "Unknown",
        }
    }
}

impl DebugRepr for Frame {
    fn debug_repr(&self) -> String {
        format!("{} {:?}", self.kind_name(), self)
    }
}

pub const METHOD_NAMES: &[(Method, &str)] = &[
    (Method::HostDescribe, crate::registry::METHOD_HOST_DESCRIBE),
    (Method::Prompt, crate::registry::METHOD_PROMPT),
    (Method::Abort, crate::registry::METHOD_ABORT),
    (Method::GetState, crate::registry::METHOD_GET_STATE),
    (Method::SetModel, crate::registry::METHOD_SET_MODEL),
    (Method::CycleModel, crate::registry::METHOD_CYCLE_MODEL),
    (
        Method::GetAvailableModels,
        crate::registry::METHOD_GET_AVAILABLE_MODELS,
    ),
    (
        Method::SetThinkingLevel,
        crate::registry::METHOD_SET_THINKING_LEVEL,
    ),
    (Method::Bash, crate::registry::METHOD_BASH),
    (Method::Compact, crate::registry::METHOD_COMPACT),
    (
        Method::GetSessionStats,
        crate::registry::METHOD_GET_SESSION_STATS,
    ),
    (Method::ExportHtml, crate::registry::METHOD_EXPORT_HTML),
    (Method::ExportJsonl, crate::registry::METHOD_EXPORT_JSONL),
    (Method::ImportJsonl, crate::registry::METHOD_IMPORT_JSONL),
    (
        Method::SwitchSession,
        crate::registry::METHOD_SWITCH_SESSION,
    ),
    (Method::Fork, crate::registry::METHOD_FORK),
    (Method::GetMessages, crate::registry::METHOD_GET_MESSAGES),
    (
        Method::EstimateContext,
        crate::registry::METHOD_ESTIMATE_CONTEXT,
    ),
    (Method::GetCommands, crate::registry::METHOD_GET_COMMANDS),
    (Method::SessionTree, crate::registry::METHOD_SESSION_TREE),
    (
        Method::TravelSessionTree,
        crate::registry::METHOD_TRAVEL_SESSION_TREE,
    ),
    (
        Method::AppendEntryLabel,
        crate::registry::METHOD_APPEND_ENTRY_LABEL,
    ),
    (Method::ListSessions, crate::registry::METHOD_LIST_SESSIONS),
    (
        Method::LoadSessionEntries,
        crate::registry::METHOD_LOAD_SESSION_ENTRIES,
    ),
    (Method::NewSession, crate::registry::METHOD_NEW_SESSION),
    (
        Method::GetSessionName,
        crate::registry::METHOD_GET_SESSION_NAME,
    ),
    (
        Method::SetSessionName,
        crate::registry::METHOD_SET_SESSION_NAME,
    ),
    (
        Method::SetSessionNameFor,
        crate::registry::METHOD_SET_SESSION_NAME_FOR,
    ),
    (
        Method::DeleteSession,
        crate::registry::METHOD_DELETE_SESSION,
    ),
    (Method::Reload, crate::registry::METHOD_RELOAD),
    (
        Method::LoadedResources,
        crate::registry::METHOD_LOADED_RESOURCES,
    ),
    (Method::GetQueueStats, crate::registry::METHOD_QUEUE_STATS),
    (Method::Steer, crate::registry::METHOD_STEER),
    (Method::FollowUp, crate::registry::METHOD_FOLLOW_UP),
    (Method::ClearQueue, crate::registry::METHOD_CLEAR_QUEUE),
    (Method::Subscribe, crate::registry::METHOD_SUBSCRIBE),
    (Method::ApproveTool, "approve_tool"),
    (Method::AnswerQuestion, "answer_question"),
    (Method::Quit, "quit"),
    (
        Method::ArmToolFreeze,
        crate::registry::METHOD_ARM_TOOL_FREEZE,
    ),
    (Method::PersistTrust, crate::registry::METHOD_PERSIST_TRUST),
];

pub fn method_name(method: &Method) -> &'static str {
    METHOD_NAMES
        .iter()
        .find(|(m, _)| m == method)
        .map(|(_, name)| *name)
        .unwrap_or_else(|| panic!("unmapped v3 Method discriminant {method:?}"))
}

pub fn dump_frame(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let frame = Frame::from_bytes(bytes)?;
    Ok(format!("{frame:#?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg_entry(id: &str, parent: Option<&str>, ts: u64, json: &str) -> SessionEntry {
        SessionEntry::MessageEntry(MessageEntry {
            base: Some(EntryBase {
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: ts,
            }),
            message_json: json.into(),
        })
    }

    #[test]
    fn dump_frame_readable_and_bounded() {
        let f = Frame::ClientRequest(ClientRequest {
            rpc_id: 1,
            request: Request::Describe(Describe {}),
            writer_token: None,
        });
        let text = dump_frame(&f.to_bytes().unwrap()).expect("dump");
        assert!(text.contains("ClientRequest"), "{text}");
        assert!(text.contains("rpc_id: 1"), "{text}");
        assert!(dump_frame(&[0x01, 0x00]).is_err(), "非法字节稳定报错");
    }

    #[test]
    fn method_table_aligns_with_registry() {
        let v3_names: std::collections::BTreeSet<&str> =
            METHOD_NAMES.iter().map(|(_, n)| *n).collect();
        let registry_names: std::collections::BTreeSet<&str> = crate::registry::names().collect();
        let non_registry = ["approve_tool", "answer_question", "quit"];

        let missing: Vec<_> = registry_names.difference(&v3_names).collect();
        assert!(
            missing.is_empty(),
            "registry 方法 {missing:?} 缺少 v3 Method 判别值"
        );
        let extra: Vec<_> = v3_names
            .difference(&registry_names)
            .filter(|n| !non_registry.contains(n))
            .collect();
        assert!(extra.is_empty(), "v3 Method 表混入未登记方法: {extra:?}");
        assert_eq!(METHOD_NAMES.len(), 41, "方法判别值集合大小漂移");
    }

    #[test]
    fn codec_conformance_bytes_locked() {
        let toolstart = Frame::ServerNotification(ServerNotification {
            seq: 43,
            notification: Notification::Event(Event::ToolStart(ToolStart {
                id: "t1".into(),
                name: "grep".into(),
                args_json: r#"{"pattern":"TODO","max":100}"#.into(),
            })),
        });
        let steer = Frame::ClientRequest(ClientRequest {
            rpc_id: 7,
            request: Request::Command(Command::Steer(Steer {
                message: "hi".into(),
            })),
            writer_token: None,
        });
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(
            hex(&toolstart.to_bytes().unwrap()),
            "01ff22bbbea2d00603001c000a80951e81276433c2ec8dc19905c40ec8212b01ff22d49e99b60304001c020cf040624b9be125c389affcb603c415c815cc150a74311267726570727b227061747465726e223a22544f444f222c226d6178223a3130307d"
        );
        assert_eq!(
            hex(&steer.to_bytes().unwrap()),
            "01ff22bbbea2d00601001c000cd02af27ee5bd45c3edd6aabd07c40ec821ce150701ff2283d98bc70420001c0207404cb24a470d4dc1fbabbc16c4150a6869fd"
        );
    }

    #[test]
    fn size_baseline_vs_json_large_transcript() {
        let entries: Vec<SessionEntry> = (0..500)
            .map(|i| {
                SessionEntry::MessageEntry(MessageEntry {
                    base: Some(EntryBase {
                        id: format!("entry-{i:04}"),
                        parent_id: (i > 0).then(|| format!("entry-{:04}", i - 1)),
                        timestamp: 1_700_000_000 + i as u64,
                    }),
                    message_json: format!(
                        r#"{{"type":"llm","role":"assistant","content":[{{"type":"text","text":"reply chunk {i} with some realistic length body text"}}]}}"#
                    ),
                })
            })
            .collect();
        let frame = Frame::ServerResponse(ServerResponse {
            rpc_id: 1,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::MessagesResult(MessagesResult { entries })),
            writer_token: None,
        });
        let v3 = frame.to_bytes().unwrap().len();

        let json_equiv = {
            let items: Vec<serde_json::Value> = (0..500)
                .map(|i| {
                    serde_json::json!({
                        "type": "message",
                        "id": format!("entry-{i:04}"),
                        "parentId": if i > 0 { serde_json::json!(format!("entry-{:04}", i-1)) } else { serde_json::Value::Null },
                        "timestamp": 1_700_000_000 + i,
                        "message": {
                            "type": "llm", "role": "assistant",
                            "content": [{"type": "text", "text": format!("reply chunk {i} with some realistic length body text")}]
                        }
                    })
                })
                .collect();
            serde_json::to_string(&serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "result": { "entries": items }
            }))
            .unwrap()
            .len()
        };

        let ratio = v3 as f64 / json_equiv as f64;
        println!("large transcript(500 entries): v3={v3}B json-rpc={json_equiv}B ratio={ratio:.2}");
        assert!(ratio < 0.95, "v3 体积收益消失: ratio={ratio:.2}");
    }

    fn roundtrip(frame: &Frame) -> Frame {
        let bytes = frame.to_bytes().expect("encode");
        Frame::from_bytes(&bytes).expect("decode")
    }

    #[test]
    fn client_request_roundtrip() {
        let f = Frame::ClientRequest(ClientRequest {
            rpc_id: 7,
            request: Request::Command(Command::Steer(Steer {
                message: "hi".into(),
            })),
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    #[test]
    fn notification_with_compaction_end_roundtrip() {
        let f = Frame::ServerNotification(ServerNotification {
            seq: 42,
            notification: Notification::Event(Event::CompactionEnd(CompactionEnd {
                result: None,
                aborted: false,
                reason: "manual".into(),
                will_retry: false,
                error_message: None,
                summary: Some("sum".into()),
                tokens_before: Some(1000),
                tokens_after: None,
                notice: None,
            })),
        });
        assert_eq!(roundtrip(&f), f);
    }

    #[test]
    fn messages_result_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 9,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::MessagesResult(MessagesResult {
                entries: vec![
                    SessionEntry::SessionHeader(SessionHeader {
                        version: 7,
                        id: "s0".into(),
                        timestamp: 1,
                        cwd: "/tmp".into(),
                        parent_session: None,
                        fork_at_entry_id: None,
                    }),
                    msg_entry("e1", None, 2, r#"{"role":"user"}"#),
                ],
            })),
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    #[test]
    fn recursive_tree_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 10,
            ok: true,
            error: None,
            payload: Some(ResponsePayload::TreeResult(TreeResult {
                nodes: vec![SessionTreeNode {
                    entry: msg_entry("n1", Some("e1"), 3, "x"),
                    children: vec![SessionTreeNode {
                        entry: SessionEntry::LabelEntry(LabelEntry {
                            base: Some(EntryBase {
                                id: "l1".into(),
                                parent_id: None,
                                timestamp: 4,
                            }),
                            target_id: "n1".into(),
                            label: Some("L".into()),
                        }),
                        children: vec![],
                        label: None,
                    }],
                    label: None,
                }],
            })),
            writer_token: None,
        });
        let back = roundtrip(&f);
        assert_eq!(back, f, "recursive tree with manual trait impls");
    }

    #[test]
    fn error_response_roundtrip() {
        let f = Frame::ServerResponse(ServerResponse {
            rpc_id: 11,
            ok: false,
            error: Some(RpcError {
                code: "session_write_held".into(),
                details: "d".into(),
            }),
            payload: None,
            writer_token: None,
        });
        assert_eq!(roundtrip(&f), f);
    }

    #[test]
    fn tool_start_args_json_survives_wire() {
        let raw = r#"{"pattern":"TODO","max":100}"#;
        let f = Frame::ServerNotification(ServerNotification {
            seq: 43,
            notification: Notification::Event(Event::ToolStart(ToolStart {
                id: "t1".into(),
                name: "grep".into(),
                args_json: raw.into(),
            })),
        });
        let back = roundtrip(&f);
        let Frame::ServerNotification(n) = back else {
            unreachable!()
        };
        match &n.notification {
            Notification::Event(Event::ToolStart(ts)) => {
                assert_eq!(ts.args_json, raw, "args JSON 原文逐字节保真")
            }
            _ => unreachable!(),
        }
    }
}
