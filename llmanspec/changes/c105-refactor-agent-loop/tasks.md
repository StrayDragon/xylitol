# Tasks: c105-refactor-agent-loop

## Session 层

- [ ] 创建 `src/agent/session.rs`：`XySession` trait + `InMemorySession` 实现
- [ ] 修改 `src/interface/cli/mod.rs`：用 `InMemorySession` 替换 `adk_session::InMemorySessionService`
- [ ] 修改 `src/interface/print.rs`：同上
- [ ] 修改 `src/interface/tui/app.rs`：同上
- [ ] 修改 `src/interface/acp.rs`：同上

## ReAct Loop

- [ ] 创建 `src/agent/runner.rs`：`XyRunner` 实现（prompt 组装 + tool dispatch + iteration 控制 + streaming event 发射）
- [ ] 实现 streaming chunk 解析：Text / Thinking / FunctionCall 分流
- [ ] 实现 tool dispatch：registry lookup → security check → execute → FunctionResponse
- [ ] 实现 max_iterations 保护

## Agent Loop 迁移

- [ ] 修改 `src/agent/loop.rs`：用 `XyRunner` 替换 `adk_runner::Runner` + `adk_agent::LlmAgentBuilder`
- [ ] 删除 `map_adk_event` 函数（XyRunner 直接产生 AgentEvent）
- [ ] 修改 `src/agent/planner.rs`：直接使用 `XyModel`（已在 c104 完成基础迁移）

## 清理

- [ ] 删除 `src/agent/compat.rs`（adk 兼容层）
- [ ] 从 `Cargo.toml` 移除 `adk-runner`、`adk-agent`、`adk-session`、`adk-core`
- [ ] 搜索并确认 `src/` 中无任何 `use adk_` 残留

## 测试

- [ ] 重写 `tests/support/harness.rs`：使用 `XyRunner` 构建测试 harness
- [ ] 新增单元测试：`test_runner_text_only`（无 tool call 场景）
- [ ] 新增单元测试：`test_runner_tool_call_loop`（多轮 tool call）
- [ ] 新增单元测试：`test_runner_max_iterations`（超限终止）
- [ ] 新增单元测试：`test_runner_streaming_events`（事件流顺序验证）
- [ ] `cargo test` 全量通过
- [ ] `just qa` 通过
