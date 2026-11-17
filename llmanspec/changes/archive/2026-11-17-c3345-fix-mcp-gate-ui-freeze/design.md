# Design

## 决议

`arm_tool_freeze` unary 立即返回当前快照；「等冻结 + 冻结 + 回推」移入 host 后台任务。有界性（门时限）保留。

| 方案 | 问题 | 结论 |
|---|---|---|
| 保持 arm 阻塞到冻结（现状） | 持会话引擎锁等待，阻塞同会话所有 unary；端主循环内联 await 时整体冻结 | 拒绝 |
| 端侧 RPC 加超时 | 治标；新增 unary 复发；掩盖真实决议 | 拒绝 |
| 主循环并发化 | 大改，碰「effects 唯一 drain 循环」约束 | 不在本次 |
| **arm 立即返回 + 后台冻结 + `session/resources` 回推** | unary 语义变化；端侧消费链路已存在 | **采用** |

## 关键事实（已对照代码）

- 冻结决议的门限时（`MCP_FIRST_TURN_GATE_TIMEOUT` = 15s）在 runtime `join_session_prepare` 内部：fire Generate 瀑布 → `timeout` 等待 → 超时回落 `hub.view().armed_result()`（天然 detach 未连 bootstrap）。该方法需要 `&mut self` 且整段阻塞——后台任务持锁调用它会把锁持住最长 15s，等于把 bug 挪个位置。
- 非阻塞路径只有 `arm_tool_freeze_gate`（gate 已 armed、hub complete 即冻结，否则 fire prepare）；但挂起 MCP 的 hub 永远不会自行 complete，超时决议无人执行。
- 端侧消费链路已存在：`Downlink::SessionResources` → `store_resources_snapshot` → `resources_dirty`；drain 由缓存 `is_tools_frozen` 驱动 gated start；`poll_prepare_progress` 轮询刷新缓存。

## 实现设计

### runtime：分段驱动冻结决议（`crates/xylitol-runtime/src/driver/in_process/`）

新增方法（与 `ensure_tool_table_frozen` 同语义、可分段驱动，供 host 后台短锁轮询）：

- `poll_tool_gate(&mut self) -> bool`：
  1. 已冻结 → `true`。
  2. `begin_tool_gating`（幂等）并记录首次 armed 时刻。
  3. 短窗等待 hub（≤100ms，且不超过门限余量）；门限耗尽 → 门限通知 + `hub.view().armed_result()`（等价 `join_session_prepare` 的超时回落）。
  4. 复用 `join_session_prepare` 尾部逻辑（`apply_prepare_tools` → freeze → `prepare_acked`），抽公共 helper 避免两份决议尾。
  5. 返回 `is_tools_frozen`。

`ensure_tool_table_frozen` / `join_session_prepare` 现有调用方（generate 路径等）不动。

### serve：arm 处理器与 trait 面

- `OccupancyEngine`（`crates/xylitol-serve/src/occupancy/engine.rs`）新增 `async fn poll_tool_gate(&mut self) -> bool`；kernel 侧 `OccupancyDriver` 委派内层驱动；host.rs 测试 `ScriptedEngine` 恒 `true`。
- `METHOD_ARM_TOOL_FREEZE`（`host.rs`）新流程：
  1. `materialize_writer_at`（不变，短锁）。
  2. 短锁：已冻结 → 取快照返回（原快速路径）。
  3. 未冻结 → 返回当前快照（如实反映未冻结），并 `tokio::spawn` 后台任务：50ms 间隔短锁轮询 `poll_tool_gate`（每次持锁 ≤ ~100ms，与 `ensure_mcp_resources_watch` 同形锁纪律）；冻结后取权威快照并 `slot.push_resources`。
  4. 任何路径不得在持锁状态下 await 会话准备 settle。

### 端侧（确认零改动）

arm 早回快照照常入缓存（未冻结态，gated submit 继续等待）；冻结完成经下行推送刷新缓存 → dirty → ticker 刷新 → 下一轮 drain 起 run。

### BDD 绑定（`tests/bdd/`）

- 场景 A 更名：`武装冻结有界低于挂起MCP` → `武装冻结不等待挂起MCP`（`bindings_server.rs` 同步）。
- then「立即返回当前快照且冻结决议经下行收口」：断言 unary ok、耗时远小于门限时（阻塞路径会等满门限时）、随后在门限时内 slot 引擎冻结收口（经 mux 订阅断言 `session/resources` 推送，或等价 host 状态断言）。
- then「不取写占用立即返回快照」：断言 unary ok 且快速返回（fixture MCP 正常连接，不阻塞）。
- 分支上步骤文本变更会使既有绑定暂时红，随 t1 落地转绿。

## 风险

- arm 早回改变了「返回即冻结」的旧契约：依赖该契约的消费方仅产品 TUI（已确认走缓存 + 下行）；server 既有 BDD 场景随本 change 改写。
- 后台轮询与 `ensure_mcp_resources_watch` 并发：均为短锁，快照推送幂等（端侧 store 幂等），无新增竞态面。
