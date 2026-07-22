---
change_id: c1480-add-otel-genai-langfuse
title: GenAI / Langfuse 属性映射（fastrace → Langfuse）
status: full
priority: 1480
depends_on:
- c1475-add-otel-export
branch: feature/c1480-add-otel-genai-langfuse
author: agent
base_sha: 746ab42b7e36c8d33b87420b0cfff83856ce6ef3
checkpointed: true
checkpoint_sha: 746ab42b7e36c8d33b87420b0cfff83856ce6ef3
---

# c1480-add-otel-genai-langfuse

## Why

`c1475` 只解决「OTLP 管子通」。当前 Langfuse 里能看到 `react.turn` / `provider.request` 等壳，但 **`sessionId` 为空**、无 generation/tool 语义、无 input/output。需要 GenAI / `langfuse.*` 属性才能在 UI 里按会话聚合并看清一轮对话。

## What Changes

### Phase 1 — Session 关联（先做）

- 进程内维护当前会话观测上下文（可跨 agent / ai-bridge，不破坏分层）
- **始终**在低频 fastrace 根 span 上写入 `langfuse.session.id`（= xylitol session UUID）
- 若用户已设置 session display name（`/session-name`），**额外**写入 `langfuse.trace.metadata.session_name`；未命名时省略
- uuid 与 name **不互相覆盖**；后期命名只更新 name，uuid 不变
- 切换 / 新建 session 时更新上下文

### Phase 2 — GenAI / observation 语义（随后）

- `provider.request` → generation（model、usage；input/output 按敏感档）
- `react.turn` → agent/span 类型提示
- `tool.execute` → tool observation（name、id、status）
- 默认不带完整载荷；显式档才带 observation input/output

## Out of scope

- 新 OTLP 装配（已归 c1475）
- 自研 Inspect UI
- Langfuse Prompt / Score / Experiment 全套
- 把分散的 `Span::root` 改成完整父子树（可后续；本 change 先保证同 session 可聚合）

## Capabilities

- `infra-otel`（扩展 session / GenAI 属性合约）
- 触及 `package-ai-bridge` 观测辅助、`agent-runtime` obs、`app` 会话切换接线

## Impact

- 开 `[otel]` 时 Langfuse Sessions 能按 uuid 聚合
- 未开 otel / 未激活 span 闸时零/近零开销（沿用既有 provider_trace 闸）
