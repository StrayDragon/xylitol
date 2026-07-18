# Tasks: c1255-update-agent-tool-intent-lifecycle

## Promote

- [x] live specs：`agent-runtime` ar21 + `intent-before-execution`；`agent-session` a3
- [x] `llman sdd change attach c1255-update-agent-tool-intent-lifecycle`
- [x] proposal `status: full`

## 实施

- [x] react：消费 ToolCallStart/Delta/End；维护 partial；MessageEnd 前不 execute / 不 ToolExecutionStart
- [x] 发射 MessageUpdate（message 含渐进 ToolCall 部分）
- [x] MessageEnd 后批执行；Start → Update（≥1）→ End
- [x] 单测：`test_tool_intent_before_execution`
- [x] BDD：`intent-before-execution` step + scenario binding
- [x] `llman sdd validate c1255-… --strict --no-check`；agent-runtime OK
