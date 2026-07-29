# Tasks: c1720-add-otel-turn-terminal-status

## 1. Specs

- [x] 1.1 修订 live `infra-otel`：新增 `otel20`（agent.turn 终态 ok/aborted）；必要时轻触 otel8；`.feature` 或 `feature:false` 文档场景
- [x] 1.2 修订 live `infra-observability` `ipt4`：低频 span 终态可关联 abort

## 2. Implementation

- [x] 2.1 `AgentTurnSpan` 支持结束原因（finish 或等价）；abort 写 level/status_message
- [x] 2.2 ReAct / cancel 路径在 turn 结束前标记 aborted；正常路径 ok
- [x] 2.3 CollectingReporter 单测

## 3. Docs + gate

- [x] 3.1 更新 `docs/architecture/进程内观测.md` 过程树终态一句
- [x] 3.2 `llman sdd validate c1720-add-otel-turn-terminal-status --strict`
