# Tasks: c1255-update-agent-tool-intent-lifecycle

status: purpose-draft — promote 后再 apply。

## Promote 前

- [ ] live specs：`agent-runtime` / `agent-session` 增加意图-先于-执行与 tool-stream MUST
- [ ] `llman sdd change attach c1255-update-agent-tool-intent-lifecycle`
- [ ] 确认 c1250 已在本分支可用（或 depends 已归档）

## 实施

- [ ] react：消费 assistant stream events；维护 partial；流结束前不 execute
- [ ] 发射 MessageUpdate（含 tool 块渐进 args）
- [ ] message_end 后批执行工具；Start/Update/End
- [ ] protocol 映射：ToolStart 保留 args（若远程面需要）
- [ ] 单测：假流 toolcall_delta × N → 无 Start；done 后才 Start
- [ ] BDD：对齐/修复 tool-stream 场景
- [ ] `just test` 相关过滤或全量；validate change
