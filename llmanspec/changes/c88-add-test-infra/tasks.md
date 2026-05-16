# c88-add-test-infra Tasks

- [ ] 创建 `tests/support/` 共享测试模块目录
- [ ] 实现 FauxProvider（FauxResponseStep::Message/Factory, 流式分块模拟, call_count 跟踪）
- [ ] 实现 TestHarness + HarnessBuilder（全连线 AgentSession, 事件捕获, tempdir）
- [ ] 实现内存管理器（SessionManager::in_memory, SettingsManager::in_memory, AuthStorage::in_memory）
- [ ] 实现 VT100Backend（feature = "dev-vt100", crossterm → vt100 parser, ratatui Backend trait）
- [ ] 实现 SSE mock 构建器（sse_event, sse_text_delta, sse_tool_call, build_sse_response）
- [ ] 创建 `.config/nextest.toml`（slow-timeout, CI profile, junit output）
- [ ] 定义 CI 分层（Tier 1-4: 快速单元 / 集成 / VT100 / E2E PTY）
- [ ] 建立回归测试模板（`{issue_number}-{short-description}.rs`）
- [ ] 全局测试初始化（`#[ctor]` 设置 INSTA_WORKSPACE_ROOT）
- [ ] 编写 FauxProvider/TestHarness/VT100Backend 单元测试
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c88-add-test-infra --strict --no-interactive`
