# Design — c04-add-agent-profile-foundation

## Context

为 c55 (planning/execution) 和未来多 agent 场景做准备。当前架构单一 model 贯穿全 session，需要让运行时支持按 profile 分配不同 model/prompt/tools。

## Goals

- 运行时灵活: `ResolvedProfile` 携带完整 agent 描述
- 配置表达力: YAML anchor/alias 友好的 flat struct
- 向后兼容: 无 `agents` section 时行为不变
- `AgentLoop` 通过 profile 接收所有参数（model, prompt, tools, iterations）

## Non-Goals

- 完整的多 agent 调度/协调（c55 范畴）
- 配置验证/迁移优化（后续重构）
- `--profile` CLI 参数（后续扩展）

## Decisions

### D1: `ResolvedProfile` 作为运行时核心

替代 `ModelConfig` 流经系统。包含 model_config + system_prompt + allowed_tools + max_iterations + name。

### D2: 配置层最小新增

`AgentProfile` (flat, 4 字段) + `AgentsConfig` (default_profile + profiles HashMap)。`serde(default)` 保证向后兼容。

### D3: 解析放在 AppConfig 上

`resolve_model()`, `resolve_profile()`, `resolve_default_profile()` 作为 `AppConfig` 的方法，CLI 层只做 `__fake__` short-circuit + `--model` override。

### D4: Tool scoping via `ToolRegistry::filtered()`

新增方法按白名单过滤工具。`None` 或空 = 全部工具。

## Risks

| Risk | Level | Mitigation |
|------|-------|------------|
| `ExecutionConfig` 字段重复 | Low | 保留不动，后续 deprecation |
| 无 `agents` section 时合成逻辑与旧行为一致 | Medium | 测试覆盖 |
