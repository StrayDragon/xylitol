---
depends_on:
  - c25-add-agent-loop
  - c70-add-session-snapshot
---

# c120-fix-session-reset

## Why

交互模式（TUI/CLI/ACP）中，同一个会话 `session_id` 的上下文会在每次提交 prompt 时被“重置”，导致模型无法记住上一轮用户设定（例如“后续每句话都要带喵~”）。

根因是 `AgentLoop::ensure_session()` 在每次 `run()` 开始时都会调用 `SessionService::create()`：

- `adk-session` 的 `InMemorySessionService::create()` 会 **覆盖同 ID 的既有 session**，并清空 `events`。
- `adk-runner` 随后用 `SessionService::get()` 读取到的历史为空，于是 LLM 上下文丢失，表现为“设定被重置”。

这属于实现 bug（与 `agent-runtime` 的 session 持久化/可恢复要求不一致），无需补充新的 spec。

## What changes

- 将 `ensure_session()` 改为 **先 `get()` 探测是否存在**（`num_recent_events: Some(0)`），仅在返回 `session not found` 时才 `create()`。
- 不再吞掉 session 错误，避免隐藏真实的存储/权限问题。

## Impact

- 修复对话上下文在同一 `session_id` 内被重复创建清空的问题。
- 新增回归测试，确保多次运行会话事件会累计而不是被重置。
