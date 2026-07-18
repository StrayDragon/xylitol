# Design: c1255 agent tool intent lifecycle

## 生命周期（对齐 pi）

```text
provider events          agent emits              side effects
─────────────────        ───────────────────      ────────────
toolcall_start/delta  →  message_update           UI 意图 only
                         (partial tool args)
done(toolUse)         →  message_end
                      →  tool_execution_start     真正 execute
                         tool_execution_update*
                         tool_execution_end
```

## XyEvent 策略

Prefer **少加类型、语义钉死**：

1. 流式：`MessageStart` → 多次 `MessageUpdate`（携带 partial，含 toolCall 块）+ 细粒度 `TextDelta`/`ThinkingDelta`（可保留）；
2. 可选：增加 `ToolCallDelta` 类事件供 TUI 少解析 partial——仅当 `MessageUpdate` 成本过高时再加；
3. 执行：`ToolExecutionStart|Update|End` **仅**在 execute 路径。

禁止：在 `ToolCallDelta` 路径调用工具。

## 与现网差异

| 今日 | 目标 |
|---|---|
| ToolCallEnd → ToolExecutionStart（流内） | tool 意图 ⊂ MessageUpdate；Start 在 MessageEnd 后 |
| 无/极少 ToolExecutionUpdate | 执行路径至少一次 Update（长输出多段可后续加强） |
| flush 时机由 TUI 绑在 Start | TUI（c1260）可在 Update 时挂旁路组件 |


## 截断 / 错误

对齐 pi：`stopReason=length` 且 tool 参数不完整时 **拒绝执行**，以错误 tool result 结束（具体文案可后续打磨）。
