# Design — c670-fix-app-tui-abort-clears-stream

## Root cause

`drain_pending` → `driver.abort()` + `note_user_abort()`（idle + Aborted），但 `agent_stream` 仍 poll；`TextDelta`/`ThinkingDelta` 调用 `set_busy_status` → 轮次「复活」。

## Fix

| 层 | 行为 |
|---|---|
| bridge `note_user_abort` | 清 `streaming_*` |
| host | abort 后 `suppress_xy_until_stream_end`；丢弃 Xy 直至 stream `None` |
| 新 run | `on_run_started` 清 suppress |

不在 app 假截断 Driver channel；丢弃已缓冲事件即可保证 UI 停轮。
