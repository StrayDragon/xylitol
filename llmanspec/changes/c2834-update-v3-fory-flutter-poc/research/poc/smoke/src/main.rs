mod wire {
    include!("../../generated/rust/xy_wire_v3.rs");
}

use wire::{AgentMessage, ContentPart, Event, Frame, LlmMessage};

fn main() {
    // 场景 1:典型下行帧 —— ToolStart 携带动态工具参数(JSON 原文模式)
    let args_value = serde_json::json!({
        "path": "/home/l8ng/Projects/__straydragon__/xylitol",
        "pattern": "TODO",
        "max_results": 100,
        "include_hidden": false
    });
    let frame = Frame::ServerNotification(wire::ServerNotification {
        seq: 128,
        event: Event::ToolStart(wire::ToolStart {
            id: "tool-42".into(),
            name: "grep".into(),
            args_json: args_value.to_string(),
        }),
    });
    let bytes = frame.to_bytes().unwrap();
    let back = Frame::from_bytes(&bytes).unwrap();
    assert_eq!(frame, back, "ToolStart frame roundtrip");

    // 同一载荷今天的 JSON-RPC 形态,比个大小
    let json_equiv = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "events.mux",
        "params": {
            "type": "tool_start",
            "id": "tool-42",
            "name": "grep",
            "args": args_value
        }
    })
    .to_string();
     std::fs::write("../frame_toolstart.bin", &bytes).unwrap();
    println!("scene1 ToolStart:   fory {} B | json-rpc {} B", bytes.len(), json_equiv.len());

    // 场景 2:今天被 to_value 擦成动态 JSON 的 MessageStart —— 升级为判别 union
    let msg = MessageStartDemo {
        role: "assistant".into(),
        msg: Some(AgentMessage::Llm(LlmMessage {
            role: "assistant".into(),
            content: vec![
                ContentPart { r#type: "text".into(), text: Some("thinking...".into()), image_base64: None },
                ContentPart {
                    r#type: "image".into(),
                    text: None,
                    image_base64: Some("aGVsbG8gZm9yeQ==".into()),
                },
            ],
        })),
    };
    let ev2 = Event::MessageStart(wire::MessageStart {
        role: msg.role.clone(),
        msg: msg.msg.clone(),
    });
    let bytes2 = ev2.to_bytes().unwrap();
    let back2 = Event::from_bytes(&bytes2).unwrap();
    assert_eq!(ev2, back2, "MessageStart union roundtrip");
    let json_equiv2 = serde_json::json!({
        "jsonrpc": "2.0", "method": "events.mux",
        "params": { "type": "message_start", "role": "assistant",
                    "message": { "role": "assistant",
                                 "content": [ {"type": "text", "text": "thinking..."},
                                              {"type": "image", "image_base64": "aGVsbG8gZm9yeQ=="} ] } }
    })
    .to_string();
    println!("scene2 MessageStart: fory {} B | json-rpc {} B", bytes2.len(), json_equiv2.len());

    // 场景 3:大载荷代表 —— TodoUpdated 全量清单(嵌套 struct + enum)
    let list = wire::TodoList {
        items: (0..30)
            .map(|i| wire::TodoItem {
                id: format!("todo-{i}"),
                content: format!("任务条目 {i} 的内容文本"),
                status: match i % 3 {
                    0 => wire::TodoStatus::Pending,
                    1 => wire::TodoStatus::InProgress,
                    _ => wire::TodoStatus::Completed,
                },
            })
            .collect(),
    };
    let ev3 = wire::Event::TodoListSnapshot(wire::TodoListSnapshot {
        todo_list: Some(list.clone()),
    });
    let bytes3 = ev3.to_bytes().unwrap();
    let back3 = Event::from_bytes(&bytes3).unwrap();
    assert_eq!(ev3, back3, "TodoList roundtrip");
    let items_json: Vec<_> = list
        .items
        .iter()
        .map(|i| {
            let status = match i.status {
                wire::TodoStatus::Pending => "Pending",
                wire::TodoStatus::InProgress => "InProgress",
                wire::TodoStatus::Completed => "Completed",
            };
            serde_json::json!({"id": i.id, "content": i.content, "status": status})
        })
        .collect();
    let json_equiv3 = serde_json::json!({
        "jsonrpc": "2.0", "method": "events.mux",
        "params": { "type": "todo_updated", "list": { "items": items_json } }
    })
    .to_string();
    println!("scene3 TodoList(30): fory {} B | json-rpc {} B", bytes3.len(), json_equiv3.len());

    println!("all roundtrips OK");
}

struct MessageStartDemo {
    role: String,
    msg: Option<AgentMessage>,
}
