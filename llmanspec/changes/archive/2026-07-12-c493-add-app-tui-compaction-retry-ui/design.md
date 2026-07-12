# design — c493 compaction / auto-retry UI

## 代码事实（explore）

| 已有 | 缺口 |
|---|---|
| `CompactionStart` → System + `set_busy_status("Compacting")` | `CompactionEnd` 只推 System，**不**恢复 Working |
| `AutoRetryStart` → `Retry {a}/{m}` | `AutoRetryEnd` 失败推 System，**不**恢复 Working |
| chrome `atc1`/`atc7`：busy 至多 1 行 | 无 Compacting/Retry 专项契约与单测 |
| `design/compaction-status.md` 草稿 | 未钉 MUST |

## 硬决议

1. **Status 短词**：`Compacting`；`Retry {attempt}/{max_retries}`（与现实现一致）。MUST NOT 多行 / 百分比墙。
2. **Scrollback**：Start/End（及 retry 失败）用既有 `UiEntry::System`（muted）；不新增 entry 变体。
3. **End 复位**：`CompactionEnd` / `AutoRetryEnd` 在 `phase == Busy` 时 MUST `status = Working`（对齐 `ToolExecutionEnd`）。Idle 时 MUST NOT 强行设 Working。
4. **Idle 触发 compact**（若未来 slash）：本切片不接线 `/compact`；若 Start 从 Idle 抬成 Busy，End 仍按「Busy → Working」——产品 slash 另 change 时可再区分 idle 回落。
5. **验收**：bridge 单测覆盖 Start/End 与 retry 成败；chrome 可复用既有 busy 单行断言或轻量 render 测。

## 非目标

改 orchestrator / retry 策略、流式 compaction 进度、Codex 浏览面。
