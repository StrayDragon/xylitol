# Tasks: c1555-add-otel-turn-input-preview

## 1. 合约

- [x] 1.1 live `infra-otel`：otel15 req + feature 场景
- [x] 1.2 `llman sdd change attach c1555-add-otel-turn-input-preview`

## 2. 实现

- [x] 2.1 `AgentTurnSpan::start` 接受用户提示；observation_io ≠ none 时写 input
- [x] 2.2 react：从本轮 `user_parts` 生成 preview 传入
- [x] 2.3 单测：none 无 input 属性；truncated 有截断 input

## 3. 校验

- [x] 3.1 `llman sdd validate c1555-add-otel-turn-input-preview --strict --no-check`
- [x] 3.2 相关单测绿
