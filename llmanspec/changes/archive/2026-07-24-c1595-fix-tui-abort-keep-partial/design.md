# c1595 Design

## Seam

- TUI harness：busy + TextDelta 后 Esc/Ctrl+C → scrollback 含 partial + abort 脚注；迟到 delta 仍不追加。
- `project_for_llm` 单测：history 含 aborted assistant → 投影不含。
- ReAct/store：abort 路径 persist 带 `XyStopReason::Aborted`。

## Flow

```text
Esc/Ctrl+C (busy, no overlay):
  latch pending.abort + suppress_xy (c670)
  flush streaming_* → UiEntry Thinking/Assistant (keep)
  append abort footer (not wipe)
  drain → Driver::abort
ReAct cancel mid-stream:
  build AssistantMessage from accumulators
  stop_reason = Aborted
  persist + MessageEnd (or persist-only) then Error("aborted")/AgentEnd
project_for_llm:
  skip Llm assistant with stop_reason aborted|error
```

## c670 修订

「clear_streaming_buffers」在 abort latch 上改为 **flush-then-clear**（缓冲清空，但内容已进 `entries`）。迟到 TextDelta 仍被 suppress。

## Non-goals

- Bang `(cancelled)` 行为
- 把 aborted partial 喂回下一轮模型
- 文案 i18n 表
