# 设计注记：session/resources chrome 下行合约落地

## 背景

实现已在 main：`fab4f2f7 fix(attach): push MCP chrome on mux instead of polling`（先于本 change 草案 `cd02ae2e`）。本变更零生产行为变更，只把合约收进 live specs——属根 AGENTS「补合约也要走 propose」路径。

## 代码事实 → 合约锚点（对拍表）

| 合约点 | 实现锚点 | 测试锚点 |
|---|---|---|
| method 表登记下行 `session/resources` | `src/protocol/wire/method.rs:50` | method.rs 单测 `is_downlink_method`；ts_export 断言含该方法 |
| payload `{session_id, snapshot}`，snapshot 与 unary 同形状 | `src/protocol/wire/envelope.rs:121-127`（SessionResourcesPayload）；host.rs `push_resources` serde 同一 `LoadedResourcesSnapshot` | envelope roundtrip |
| 写者侧本地 poll、dirty 才推、幂等 watch | host.rs `ensure_mcp_resources_watch`（50ms poll + weakref 退出）；接线于 `materialize_writer_at` 两路径 | host.rs watch/push 测试 |
| 不耗 seq、不进 journal、重放不复播 | push_resources 注释「Not journaled」；不走 append 路径 | remote.rs 测试断言 `session/resources MUST NOT consume journal seq` |
| mux 广播给订阅连接 | `push_resources` → `broadcast` | 同上 |
| TUI tick 只读脏缓存、不轮询 unary | remote.rs `poll_mcp_bootstrap` = `resources_dirty.swap(false)`；mux 循环匹配帧→更新三份缓存置脏；proto.rs 契约注释 | remote.rs mux 帧驱动测试 |
| 首帧前初始快照一次性 unary | subscribe 路径与按需 `loaded_resources_snapshot()` | remote.rs loaded_resources 族测试 |
| 旧客户端忽略未知下行 | remote.rs `Some(Ok(_)) => {}` | — |

## 决策记录

1. **w1 扩列表 + 新专条 sr-resource2**：方法名清单是 wire 层完整性（w1），推送语义/非 journal 约束是行为合约（sr-resource2），分开写避免 w1 statement 膨胀。`w8` 已被冷恢复投影占用，故用 `sr-resource2` 沿 `sr-resource1`（Host 资源方法接线）族命名。
2. **snapshot 钉死同形状**：客户端已按 `LoadedResourcesSnapshot` 反序列化缓存；收窄字段会破坏 unary 与下行的单一形状，无收益。
3. **广播语义**：实现为 mux 连接级 broadcast。合约表述「向该 session 的 mux 连接广播」，不钉死单/多客户端数量。
4. **protocol-app 不动**：见 proposal Capabilities 节。

## 测试边界（前置确认）

- **复用既有边界**：驱动级异步测试（`app/core/driver/remote.rs` / host.rs）+ protocol 单测，全部已在闸内。
- **BDD 用 toon 文档行（feature:false）**，不新增 `.feature` 场景与步骤定义：现有 BDD 步骤为字面量式（tests/bdd/steps_server.rs），对 async mux/dirty 行为表达力差，新增只会重复既有异步测试的断言；与 `sr-q1` / `sr-abort1` 文档行先例一致。
- apply 阶段若对拍发现 spec 陈述与实现出入：以代码为准修措辞；发现覆盖缺口补最小驱动级测试。

## apply 预期

纯核验收口，零生产代码改动（参照 c2345 同型票）。
