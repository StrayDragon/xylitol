# c25-add-agent-loop Tasks

- [x] 定义 AgentEvent 枚举（TextDelta, ToolCallStart, ToolCallEnd, StepComplete, Error）
- [x] 实现 agent 执行循环（prompt → LLM → 解析响应 → 工具调用/文本输出 → 循环）
- [x] 集成 adk-model LLM Provider（流式响应）
- [x] 集成 adk-session SQLite 后端（状态持久化）
- [x] 实现工具调用分派（调用 ToolRegistry）
- [x] 实现上下文构建（对话历史 + 系统提示词 + 工具结果）
- [x] 编写集成测试（MockLlm + InMemorySessionService + 事件断言）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c25-add-agent-loop --strict --no-interactive`
