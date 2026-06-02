# 异步任务与 UI 反馈

## 6.1 Spinner 系统

```typescript
// packages/opencode/src/cli/cmd/tui/component/spinner.tsx
const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
// 使用 opentui-spinner 原生组件
<spinner frames={SPINNER_FRAMES} interval={80} color={color()} />
```

Prompt 区域使用更丰富的 spinner 配置（`prompt/index.tsx#L1444-L1466`），带有 agent 颜色匹配和淡入淡出效果。

## 6.2 会话状态追踪

```typescript
// context/sync.tsx#L511-L520
session.status(sessionID) {
  if (session.time.compacting) return "compacting"
  const last = messages.at(-1)
  if (last.role === "user") return "working"
  return last.time.completed ? "idle" : "working"
}
```

## 6.3 任务取消（中断机制）

三次 Esc 中断策略（`prompt/index.tsx#L466-L491`）：
1. 第一次 Esc — 显示 "again to interrupt"
2. 第二次 Esc — 发送 `session.abort`
3. 5 秒超时重置计数器

## 6.4 重试反馈

重试状态显示在 Prompt 底部，包含倒计时和错误信息（`prompt/index.tsx#L1640-L1696`）。

## 6.5 SSE 事件流重连

SDK 层实现指数退避重连（`context/sdk.tsx#L78-L100`），从 1s 到 30s 上限。
