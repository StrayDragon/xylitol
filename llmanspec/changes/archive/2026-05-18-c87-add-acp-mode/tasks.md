# c87-add-acp-mode Tasks

- [x] 添加 `agent-client-protocol` 依赖到 Cargo.toml（feature = "infra-acp"）
- [x] 创建 `src/interface/acp.rs` 存根（`run_acp_mode` 入口 + cfg 条件编译）
- [x] 实现 ACP Agent handler（initialize, session/new, session/prompt, session/cancel, session/close）
- [x] 实现 AgentEvent → ACP SessionNotification 转换器
- [x] 集成 CLI 分派（`--mode acp`，由 c15 的 RunMode::Acp 路由）
- [x] 编写测试（mock stdio，协议方法验证，事件转换正确性）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c87-add-acp-mode --strict --no-interactive`
