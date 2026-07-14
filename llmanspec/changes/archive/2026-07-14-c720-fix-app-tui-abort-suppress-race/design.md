# Design — c720-fix-app-tui-abort-suppress-race

## 推荐实现（默认方案）

```text
Busy Esc (try_busy_input):
  pending_abort = true
  pending_steer = None
  suppress_xy_until_stream_end = true   // NEW: sync arm
  // 可选：乐观 UI — 若调用 note_user_abort 会清 pending_abort，故 MUST NOT
  // 在 drain 前调用完整 note_user_abort；仅臂装 suppress + 可单独清 streaming 缓冲

drain_pending:
  take_abort → Driver::abort → note_user_abort（幂等）→ clear_queue(steer)
```

**为何不在 Esc 时直接 `note_user_abort`：** 它会 `pending_abort=false`，导致 drain 漏调 `Driver::abort`。

**为何不强制主环 biased：** 同步 suppress 已关闭窗口；biased 作可选加固，不阻塞本变更。

## 与 ath8 关系

ath8 原文「note_user_abort 后丢弃」→ 修改为「Esc latch 同步进起丢弃」，语义收紧、测试从 c715「现状」改为「期望不复活」。

## Bang

不变：`note_bash_cancelled`；无 `suppress_xy`。

## 开放点（见会话确认）

若需「Esc 立刻显示 Aborted 文案」而不等 drain：拆 `arm_abort_ui()`（写 note + 清 streaming + suppress）与 `pending_driver_abort` 标志，drain 只调 Driver。默认方案**不**拆，接受最多一帧延迟的 Aborted 文案，优先堵 delta。
