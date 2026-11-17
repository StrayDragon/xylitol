# Design

## 决议

`poll_tool_gate` 从「`hub.wait(window)` 分段等待」改为「`hub.view()` 纯读轮询」。

| 方案 | 问题 | 结论 |
|---|---|---|
| `hub.wait(window)` 每轮调用(现状) | `wait` = `timeout(window, ensure())`:ensure 重跑打断连接(连接抖动)、窗口超时刷门限通知 | 拒绝 |
| **`hub.view()` 纯读轮询** | settle 判据只看 view;进度由 materialize 时 `begin_session_prepare` → `hub.fire()` 自启的 ensure 推进 | **采用** |

## 实现设计

- settle 判据:`hub.view().configured == 0 || view.complete`(与 in-process `arm_tool_freeze_gate` 同判据)。
- 未 settle 且未到 `MCP_FIRST_TURN_GATE_TIMEOUT` → 返回 false(宿主后台 50ms 后再轮询;每次调用零阻塞、零副作用)。
- 门限耗尽 → 写一次 `SESSION_PREPARE_TIMEOUT_NOTICE` + `hub_snapshot_result()`(armed 子集)收口,与 `join_session_prepare` 超时路径同语义。
- settle 尾复用 `settle_tool_gate`(apply tools + freeze + ack)。
- `TOOL_GATE_POLL_WINDOW` 常量与 `hub_wait` 调用删除。

## 单测(`prepare.rs` tests)

计数 stub hub(记录 `ensure` / `wait` 调用数、view 可控):
1. view 未完成 → `poll_tool_gate` 返回 false 且 `ensure`/`wait` 调用数为 0(轮询零副作用)。
2. view 完成 → 返回 true 且工具表冻结。
3. 把内部起始时刻回拨超过门限 → 返回 true、`mcp_gate_notice` 恰一次、冻结收口。
