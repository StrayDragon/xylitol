# c87-add-acp-mode Tasks

- [ ] 添加 `agent-client-protocol` + `agent-client-protocol-schema` 依赖到 Cargo.toml（feature = "infra-acp"）
- [ ] 创建 `src/interface/acp.rs` 存根（`run_acp_mode` 入口 + cfg 条件编译）
- [ ] 实现 ACP Agent handler（initialize, session/new, session/prompt, session/cancel, session/close）
- [ ] 实现 AgentEvent → ACP SessionNotification 转换器
- [ ] 集成 CLI 分派（`--mode acp`，由 c15 的 RunMode::Acp 路由）
- [ ] 编写测试（mock stdio，协议方法验证，事件转换正确性）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c87-add-acp-mode --strict --no-interactive`
