# design — c482 abort resume

## 根因（已证实）

```text
ReActAgent::new → CancellationToken::new()   // 一次
abort()         → cancel.cancel()            // 永久 cancelled
run()           → cancel.clone()             // 仍是 cancelled
loop            → is_cancelled() → Error("aborted")
```

用户体感：Esc 后任意输入 → scrollback 再刷 `aborted` error，无法恢复。

## 决策

### A. Token 重置（主修）

`cancel` 改为可替换（如 `Mutex<CancellationToken>` / `RwLock`）：

| 时机 | 行为 |
|---|---|
| `run` 入口 | `*guard = CancellationToken::new();` 再 clone 进 stream |
| `abort` | 取消 **当前** guard 内 token；不清 follow-up（既有 ar-q4） |

`cancel_token()` 返回当前 token 的 clone（工具 ctx 同）。

### B. 结束事件形态

Abort 路径在 break 前：

1. `flush` 已有 streaming（bridge 侧已有 flush_streaming）
2. `yield AgentEnd { … }`（messages = 当前 history），**或** 至少确保 stream 结束触发 `on_run_stream_closed` → idle
3. `Error("aborted")`：**要么删除**，要么 bridge 把 `msg == "aborted"` 映射为 dim System「aborted」并 `phase = Idle`，**禁止**当作阻塞性故障

推荐：保留一行 muted system「aborted」，并 `AgentEnd`，去掉粘性 Error 样式（或 Error 也强制 idle）。

### C. 非目标

不改队列 abort 语义（清 steer、留 follow-up）；不改双 Esc 树。
