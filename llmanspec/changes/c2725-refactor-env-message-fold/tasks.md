# Tasks: c2725-refactor-env-message-fold

> pre-start。可与 c2700 并行。

## 1. 收口

- [ ] 1.1 列出所有 `EnvMessage::` match（`rg`）。
- [ ] 1.2 [blocked-by: 1.1] 在 `protocol::message` 实现穷举 helper（见 design §3）。
- [ ] 1.3 [blocked-by: 1.2] `llm_project` / session / bang / session_env 改用 helper；删除重复 match。

## 2. 验证

- [ ] 2.1 [blocked-by: 1.3] 相关单测；确认 JSONL 夹具无需改键。
- [ ] 2.2 [blocked-by: 2.1] `just qa` 触及面；`llman sdd validate c2725-refactor-env-message-fold --strict --no-check`。
