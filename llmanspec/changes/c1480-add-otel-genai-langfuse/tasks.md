# Tasks: c1480-add-otel-genai-langfuse

## Phase 1 — Session uuid + name

- [x] bridge：`ObsSessionContext`（id 必填槽 + optional name）+ get/set API
- [x] 根 span 属性注入：`langfuse.session.id`；有 name 时 `langfuse.trace.metadata.session_name`
- [x] 接线：`ProviderRequestTrace` + `agent/runtime/obs` 低频根 span
- [x] app driver：new/switch/set_session_name 时更新上下文
- [x] live specs：`otel6` / `otel7` + feature 场景
- [x] 单测：属性集合；无名/有名；切换 id

## Phase 2 — GenAI / observation

- [ ] `provider.request` → generation 属性（model / usage；I/O 档位）
- [ ] `react.turn` / `tool.execute` observation 类型与工具字段
- [ ] 敏感档配置（默认不带完整 I/O）
- [ ] specs 扩展 + 单测
- [ ] roadmap M2 一句兑现

## 校验

- [x] `llman sdd validate c1480-add-otel-genai-langfuse --strict --no-check`（Phase 1；Phase 2 任务仍开放）
- [x] `just lint` / 相关单测（Phase 1）
