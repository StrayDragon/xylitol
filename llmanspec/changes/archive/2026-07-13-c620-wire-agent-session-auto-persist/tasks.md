# Tasks — c620-wire-agent-session-auto-persist

- [x] 1.1 bootstrap/`into_runtime`：绑定 `session_id` 到 Agent；确认 Driver 与 Agent 共享同一 `Arc` store
- [x] 1.2 `InProcessDriver::run` 改为 `run_with_id(当前 session_id)`，禁止每轮新 Uuid
- [x] 2.1 `SessionManager` persisted：pi 式延迟落盘（首条 assistant 刷出）；`in_memory` 不写盘；单测
- [x] 2.2 ReAct：user / assistant / toolResult 完成时 `append_session_entry(Message)`；单测 jsonl/load 含消息
- [x] 2.3 `run_with_id`：从 `build_session_context` 灌 history 再追加本轮 user；双轮同 sid 测
- [x] 3.1 回归：一轮后 `session_tree(MessageHistory)` 非空；产品/harness 树可见 user（ast7）
- [x] 4.1 `llman sdd validate c620-wire-agent-session-auto-persist --strict --no-interactive`
- [x] 4.2 `cargo test -p xylitol --lib`（含 agent/runtime、session、driver session_tree 相关）
