# Design: c1480-add-otel-genai-langfuse

## Phase 1 — Session 上下文

| 决策 | 选择 | 理由 |
|---|---|---|
| 存放位置 | `xylitol-ai-bridge` 内进程态（同 `provider_trace_active`） | agent ↛ infra；provider 与 ReAct obs 都能读 |
| session id 属性 | `langfuse.session.id` | Langfuse OTEL 映射 → `sessionId` |
| session name 属性 | `langfuse.trace.metadata.session_name` | 可过滤 metadata；不占用 sessionId |
| 写入时机 | 每个低频根 span 创建时读当前上下文 | 命名可后到；新 span 带新 name |
| 更新点 | `set_session` / `new_session` / `switch_session` / `set_session_name*` | app driver 已知真相 |

```text
app driver 切换/命名 session
  → bridge::set_obs_session(id, name?)
agent obs / provider.request
  → with_properties += langfuse.session.id [+ metadata.session_name]
fastrace → OTLP → Langfuse Session(uuid)
```

## Phase 2 — GenAI（摘要）

- 属性键对齐 Langfuse GenAI / `langfuse.observation.*`
- 敏感档：默认关完整 I/O；配置或 env 显式开
- 不在本 design 展开实现细节；apply Phase 2 时补齐

## 非目标

- 强制父子 span 树（各 `Span::root` 仍独立 trace，但同 sessionId）
