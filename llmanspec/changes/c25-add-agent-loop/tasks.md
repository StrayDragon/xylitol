# c25-add-agent-loop Tasks

- [ ] 定义 AgentEvent 枚举（TextDelta, ToolCallStart, ToolCallEnd, StepComplete, Error）
- [ ] 实现 agent 执行循环（prompt → LLM → 解析响应 → 工具调用/文本输出 → 循环）
- [ ] 集成 adk-model LLM Provider（流式响应）
- [ ] 集成 adk-session SQLite 后端（状态持久化）
- [ ] 实现工具调用分派（调用 ToolRegistry）
- [ ] 实现上下文构建（对话历史 + 系统提示词 + 工具结果）
- [ ] 编写集成测试（wiremock SSE mock + TestHarness 全连线 + 事件断言）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c25-add-agent-loop --strict --no-interactive`
