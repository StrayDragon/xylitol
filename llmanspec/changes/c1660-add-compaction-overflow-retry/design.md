# Design: c1660-add-compaction-overflow-retry

## 对照 pi（一手）

| pi | xylitol 现状 | 本 change |
|---|---|---|
| `_checkCompaction` Case1：`sameModel && isContextOverflow` | 仅 Case2 threshold（`try_threshold_auto_compact`） | ✅ Case1 |
| `_overflowRecoveryAttempted` 一次 | 无 | ✅ |
| `willRetry = stopReason !== "stop"`；成功超窗 compact 不 continue | 无 | ✅ |
| 重试前从 agent state 摘掉末条错误 assistant（session 可保留） | 无 | ✅ |
| `compaction_start/end` reason=`overflow`；二次失败固定文案 | reason 仅 manual/threshold；`CompactionEnd` 无 reason/willRetry/error | ✅ 扩展事件 |
| `is_retryable` 排除 overflow | `retry.rs` 注释称交给 compaction，但**无** overflow 识别与 compact-retry | ✅ 同源检测器 |

## Overflow 识别（锁定）

SSOT helper（建议落点：`agent/compaction` 或 `agent/runtime` 内聚模块，**一处**供 ReAct + retry）：

1. **Usage 超窗**：`usage.input`（或等价）> `context_window`（>0）→ overflow（对齐 pi / z.ai 静默超窗）。
2. **length + 零输出填窗**：`stop_reason=length/max_tokens` 且 output≈0 且 input 填满窗（MiMo 类）。
3. **错误文案**：`stop_reason=error` 且 `error_message` 匹配 overflow 模式集，且**不**命中 non-overflow 排除（rate limit 等）——模式集以 pi `overflow.ts` 为蓝本，集中常量，禁止散落 `contains`。
4. **sameModel**：assistant 的 `provider`+`model` 必须等于当前模型；否则跳过 Case1。

测试注入：Fake 产出 `AssistantMessage { stop_reason: Error, error_message: <匹配模式> }`（或 usage 超窗）；**不**依赖专用生产钩子。

## 编排流（turn-end，在 threshold 之前）

```text
TurnEnd (非 abort):
  if !enabled → skip
  if stale (assistant ≤ latest CompactionEntry) → skip
  if sameModel && is_context_overflow(assistant, window):
    willRetry = stop_reason != Stop
    if !willRetry:
      auto_compact(reason=overflow, willRetry=false); return
    if overflow_recovery_attempted:
      emit CompactionEnd { reason=overflow, willRetry=false,
        errorMessage="Context overflow recovery failed after one compact-and-retry..." }
      return  # 不循环
    overflow_recovery_attempted = true
    从 ReAct 工作 history 摘掉末条错误 assistant（已落盘可保留）
    auto_compact(reason=overflow, willRetry=true)
    if willRetry && compact 成功 → 继续本 run 下一模型调用（同 turn 续跑）
  else:
    既有 threshold Case2
```

`overflow_recovery_attempted`：按 **run / session 回合** 持有；新 user prompt / 成功恢复后可复位（对齐 pi：新 prompt 清零）。

## 事件

| 事件 | 字段 |
|---|---|
| `CompactionStart` | `reason`: `manual` \| `threshold` \| **`overflow`** |
| `CompactionEnd` | 增补 `reason`（可选兼容）、**`will_retry`**、**`error_message`**（失败时）；既有 `result`/`aborted` 保留 |

二次 overflow 失败：可只发 `CompactionEnd`（无对应 Start），对齐 pi。

## 与 retry 边界

- `is_retryable_error` / 流错误分类 MUST 调用同一 overflow 检测；overflow → **false**（不进 AutoRetry）。
- MUST NOT 把 429/5xx 误判为 overflow（non-overflow 排除表）。

## 非目标

c1670 instructions · c1680 TUI% · extension compact · 可配置 max retries（钉死 = 1）· 动态策略

## 验收 seam

| 锚点 | 覆盖 |
|---|---|
| overflow-retry-ok | Fake overflow → compact → 重试成功 |
| overflow-once | 第二次 → 固定失败文案，无第三轮 |
| wrong-model | 换模后旧 overflow 不触发 |
| reason-overflow | Start/End reason 含 overflow |
