# c98-refactor-tool-args-and-app-split Tasks

- [ ] 提取 `src/agent/tools/args.rs` helper 模块（get_string/get_bool/get_u64 等）
- [ ] 迁移 7 个工具（read/write/edit/bash/grep/find/ls）的参数解析到 args helper
- [ ] 拆分 `src/interface/tui/app.rs`（键盘处理、overlay 管理、agent 事件分发）
- [ ] 统一错误类型约定（anyhow vs thiserror）并更新 AGENTS.md
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c98-refactor-tool-args-and-app-split --strict --no-interactive`
