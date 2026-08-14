# 设计：在线节点钟

## 算法

每个节点 `Option<(Instant, unix_ms)>`。事件到达：

- start 类：空则写入，之后 no-op
- `thinkingEnded`：空则写入（TextDelta / ToolCall* / ThinkingEnd 先到者赢；禁止用 Done）
- `textEnded`：每次 TextDelta 覆盖（唯一「最后一次」节点）
- 读时长：`end.instant.saturating_duration_since(start.instant)`，不到 1s 则省略秒

AgentStart / TurnStart 在 ReAct 外层打戳，persist 该条 assistant 时拷入，不在 chunk 循环里重打。

## 落盘形状

assistant message 顶层附加（非 AgentMessage 字段，serde 忽略）：

```json
"thinkingElapsedSecs": 2,
"streamTiming": {
  "agentStartedAtMs": 1,
  "turnStartedAtMs": 2,
  "thinkingStartedAtMs": 3,
  "thinkingEndedAtMs": 4,
  "textStartedAtMs": 5,
  "textEndedAtMs": 6,
  "toolIntentAtMs": 7,
  "messageEndedAtMs": 8
}
```

只写发生过的键。已有的顶层 `thinkingStartedAtMs` / `thinkingEndedAtMs`（quick 修复）并入 `streamTiming`，不再双写顶层，以免两套键。

## TUI

Live 仍用思考通道钟：首个 TextDelta 封 Thought。Resume：`thinkingElapsedSecs` → `streamTiming` 思考节点差 → 省略。
