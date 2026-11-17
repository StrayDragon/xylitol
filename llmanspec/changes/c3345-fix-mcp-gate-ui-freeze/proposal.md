---
depends_on: []
blocks: []
title: MCP 准备门武装不再阻塞会话引擎锁
---

# MCP 准备门武装不再阻塞会话引擎锁

## Why

MCP 正在准备时,产品 TUI 提交首条消息会整体冻结到 MCP 就绪:gated 队列与 Assembling 状态按设计落位,但随后主循环停摆(键入 / tick / 重绘全停),冻结完成后消息才上行。根因:host 的 `arm_tool_freeze` unary 在持有会话引擎(`slot.driver`)互斥锁的临界区内等待工具表冻结(`ensure_tool_table_frozen` → `join_session_prepare`),而端主循环内联 await 的 `LoadedResources` / `mcp_catalog` 等 unary 在这把锁上排队,直到准备 settle 才返回(`xylitol-serve/src/occupancy/host.rs` 的 arm 处理器与 `loaded_resources_snapshot_for`;端侧主循环 `drain_pending` 内联 await 与 arm 后强刷)。

## What Changes

- `arm_tool_freeze` unary 改为不等待决议:已冻结 → 原样返回权威快照;未冻结 → 立即返回当前快照,「等冻结 + 冻结 + 回推」移入后台任务。
- 后台任务以非阻塞 `XyDriver::arm_tool_freeze_gate` 短锁轮询直至冻结,完成后经既有 `session/resources` 下行推送权威快照;端侧消费链路(`Downlink::SessionResources` → 快照缓存 → `resources_dirty` → drain gated-start)已存在,端侧产品代码零改动。
- 有界性保留:连不上的 MCP 仍由门时限(`MCP_FIRST_TURN_GATE_TIMEOUT`)收口——超时 detach 未连 bootstrap、冻结已 armed 子集,决议结果同样经下行回推。
- specs:`server-core` r1923 / r1925 规则与场景改写为新合约;`app-tui-host` 新增裸规则「MCP 准备期提交保持可交互」。

## Design Choice: 立即返回 + 下行收口,而非维持阻塞返回

| 方案 | 问题 | 结论 |
|---|---|---|
| 保持 arm 阻塞到冻结(现状) | 持锁等待阻塞同会话所有 unary;端主循环内联 await 时整体冻结 | 拒绝 |
| 端侧给所有 RPC 加超时 | 治标;任何新增 unary 都会复发;超时掩盖真实决议 | 拒绝 |
| 主循环 drain 与输入并发化 | 改动大,碰「effects 唯一 drain 循环」硬约束 | 不在本次 |
| **arm 立即返回 + 后台冻结 + `session/resources` 回推** | unary 语义变化(未冻结时可能返回未冻结快照);端侧推送消费链路已确认存在 | **采用** |

兼容性:client 与 host 同仓同发;快照形状无 schema 变更;协议版本仍只经 `PROTOCOL_VERSION` / `host.describe` 协商。

## Impact

- crates:`xylitol-serve`(arm 处理器)、`tests/bdd`(步骤绑定)、`xylitol-app-tui`(仅 harness 测试切片,产品代码零改动)。
- specs:`server-core`(r1923 / r1925 改写)、`app-tui-host`(新增裸规则)。

## Further Notes

- 端侧下行消费已确认:`Downlink::SessionResources` → `store_resources_snapshot` → `resources_dirty`(`xylitol-app-tui/src/driver/remote.rs`);gated start 在 drain 中由缓存的 `is_tools_frozen` 驱动(`effects/mod.rs`),MCP 准备进度经 `poll_prepare_progress` 轮询刷新。
- host 侧 50ms 短锁轮询已有同形先例:`ensure_mcp_resources_watch`(`xylitol-serve/src/occupancy/host.rs`)。
