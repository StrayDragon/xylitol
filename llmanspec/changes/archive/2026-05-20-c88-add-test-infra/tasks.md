# c88-add-test-infra Tasks

- [x] 创建 `tests/support/` 共享测试模块目录
- [x] 实现 FauxProvider（FauxResponseStep::Message/Factory, 流式分块模拟, call_count 跟踪）
- [x] 实现 TestHarness + HarnessBuilder（全连线 AgentSession, 事件捕获, tempdir）
- [x] 实现内存管理器（SessionManager::in_memory, SettingsManager::in_memory, AuthStorage::in_memory）
- [x] 实现 VT100Backend（feature = "dev-vt100", crossterm → vt100 parser, ratatui Backend trait）
- [x] 实现 SSE mock 构建器（sse_event, sse_text_delta, sse_tool_call, build_sse_response）
- [x] 创建 `.config/nextest.toml`（slow-timeout, CI profile, junit output）
- [x] 定义 CI 分层（Tier 1-4: 快速单元 / 集成 / VT100 / E2E PTY）
- [x] 建立回归测试模板（`{issue_number}-{short-description}.rs`）
- [x] 全局测试初始化（`#[ctor]` 设置 INSTA_WORKSPACE_ROOT）
- [x] 编写 FauxProvider/TestHarness/VT100Backend 单元测试
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c88-add-test-infra --strict --no-interactive`
