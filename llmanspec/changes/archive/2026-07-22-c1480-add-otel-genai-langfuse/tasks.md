# Tasks: c1480-add-otel-genai-langfuse

## Phase 1 — Session uuid + name

- [x] bridge：`ObsSessionContext`（id 必填槽 + optional name）+ get/set API
- [x] 根 span 属性注入：`langfuse.session.id`；有 name 时 `langfuse.trace.metadata.session_name`
- [x] 接线：`ProviderRequestTrace` + `agent/runtime/obs` 低频根 span
- [x] app driver：new/switch/set_session_name 时更新上下文
- [x] live specs：`otel6` / `otel7` + feature 场景
- [x] 单测：属性集合；无名/有名；切换 id

## Phase 2 — GenAI / observation

- [x] `provider.request` → generation 属性（model；I/O 默认不带）
- [x] `react.turn` / `tool.execute` observation 类型（agent / tool）
- [x] 敏感档：默认不带完整 I/O（otel8）；显式载荷档留后续
- [x] specs 扩展（otel8）+ 单测（generation helpers）
- [x] roadmap M2 一句兑现

## 校验

- [x] `llman sdd validate c1480-add-otel-genai-langfuse --strict --no-check`
- [x] `just lint` / 相关单测
