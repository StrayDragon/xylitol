---
change_id: c482-fix-agent-abort-resume
title: "agent-runtime：abort 后重置 cancel，TUI 可继续对话"
status: ready
priority: 482
depends_on:
  - "c461-expose-steer-followup-seam"
  - "c480-add-app-tui-input"
author: agent
track: B
---

# c482-fix-agent-abort-resume
## Why

产品 TUI 流中 Esc → `Driver::abort` 后，再输入任意新 prompt 会立刻出现 `Error: aborted`，会话无法继续。根因：`ReActAgent` 的 `CancellationToken` 在 `new` 时创建一次，`abort()` 永久 `cancel()`；下次 `run()` 仍 clone 已取消的 token，循环入口立即 `yield Error("aborted")`。

## What Changes

1. **Runtime**：每次 `run` 开始 MUST 使用**新的** `CancellationToken`（替换已取消的）；`abort()` 只取消**当前** run 的 token。
2. **事件**：用户 abort 结束时 MUST 发出可恢复的结束信号（优先 `AgentEnd` + 可选短 system/note；**MUST NOT** 把普通 Esc abort 当成粘性 `XyEvent::Error` 让后续输入继续刷 error）。若保留 `Error("aborted")` 过渡，bridge MUST 将其视为取消而非致命，并强制 idle。
3. **TUI**：abort 后 `run_active` / `UiPhase` MUST 回到可提交 idle（follow-up 仍可按既有 restore 语义）；用户可立即再发消息。
4. **单测**：abort 后再 `run` 必须产生正常流（mock model），不得立刻 aborted。

## Capabilities

- `agent-runtime`：cancel token 生命周期 / abort 可恢复
- `app-tui-input`：abort 后 UI 恢复可提交

## Out of scope

- 部分输出回滚 / rewind
- 远程 Driver abort 协议变更（若 Remote 已透传 abort，仅需本地 token 修复）

## Impact

- 触达：`src/agent/runtime/react.rs`（主修）、`src/app/tui/bridge.rs`（Error/abort 分类）、可选 `mod.rs`/`host.rs`。
- 风险：低；修复后行为对齐 pi「Esc 停当前轮，可继续聊」。
