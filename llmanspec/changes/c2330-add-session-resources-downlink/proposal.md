---
depends_on: []
---

# 下行补 session/resources：MCP chrome 走 mux，不进 journal

产品 TUI attach 后，MCP / skills 头卡不能靠 TUI 每 tick unary `loaded_resources`。Host 在写者侧本地 poll，脏了才推一帧 chrome 快照。实现已由 `fab4f2f7`（fix(attach): push MCP chrome on mux instead of polling）落在 main；本变更是纯合约收口——把该方法收进 live specs。依据根 AGENTS：默认分支禁止为已落地实现补 live specs，补合约也走 propose（绑定分支 land）。

## Why

`server-core` 的 `w1` 产品下行 method 目前 MUST 含 `session/event`、`session/subscribed`、`session/resync_required`。MCP 连接态刷新不是回合磁带：塞进 `session/event` 会占 seq、进 journal，断线重放会把 chrome 当实况事件。TUI 16Hz unary 则在 Assembling 时反复打网。

需要一条 **非 journal** 的 mux 下行，专推 LoadedResources 快照。未知下行对旧客户端已是忽略（`Some(Ok(_))`），加法兼容。

## What Changes

- **server-core**：
  - `w1` 下行信封的 MUST-contains method 列表扩入 `session/resources`。
  - 新增 requirement `sr-resource2`（chrome 资源下行）：Host 为已物化写者的会话提供该下行——写者侧在 Host 进程内本地 poll MCP bootstrap，快照变化时向该 session 的 mux 连接广播一帧；payload 含 `session_id` 与资源快照（形状与 `loaded_resources` unary 结果一致）。该帧 MUST NOT 消耗 journal seq、MUST NOT 写入 EventJournal；断线重放与冷恢复 MUST NOT 复播 chrome 帧。不识别该方法的旧客户端 MUST 可忽略且不受影响；MUST NOT 要求 bump 协议版本。
  - scenarios[] 补 `sr-resource2` 文档行（feature:false）。
- **app-tui-host**：
  - 新增 requirement `ath38`（attach chrome 由下行驱动）：attach 下 MCP/skills 头卡的连接态刷新由 Host `session/resources` 下行置脏、tick 读本地缓存刷新；MUST NOT 以周期性 unary 轮询同一刷新；首帧前的初始快照 MAY 经一次性 `loaded_resources` unary 获取。
  - scenarios[] 补 `ath38` 文档行（feature:false）。

## 已拍板决策（原开放决策）

1. **req 组织**：`w1` 扩列表 + 另开专条 `sr-resource2`（`w8` 已被「冷恢复投影」占用）；命名沿 `sr-resource1` 族。
2. **snapshot 形状**：钉死为与 `loaded_resources` unary 同一 Value 形状（`LoadedResourcesSnapshot` 序列化），代码事实即如此。
3. **广播 vs 写者专属**：mux 连接级广播（订阅该 session 的每个连接各收一份）；poll 仅在写者槽跑（`materialize_writer_at` 接线，watch 幂等）。reader 槽无 driver 可 poll。

## Capabilities

- `server-core`（w1 扩列表 + sr-resource2）
- `app-tui-host`（ath38 tick 只读脏缓存）

protocol-app 不动：方法表列举归 server-core `w1`；payload wire 类型受 `pa-bind1` TS 导出闸自动覆盖；`pa-map1`「不为每个增量另开方法名」不冲突——chrome 快照非 Event 增量。

## Impact

Attach TUI 的 MCP 头卡与同进程一致，且不把 chrome 污染进回合磁带。合约与已落地实现对齐；零生产行为变更。

## 非目标

改 ReAct / 工具闸；把 skills 文件内容塞进下行；c2307 冷恢复 journal 回放；bump `PROTOCOL_VERSION`。

## Further Notes

决策依据、测试边界与对拍锚点见 `design.md`。
