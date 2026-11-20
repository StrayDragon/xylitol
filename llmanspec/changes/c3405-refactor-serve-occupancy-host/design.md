# Design

## 结论

- c8（载荷手撕）与 c9（defer 惯用法）前提成立，计数勘误后按 proposal What Changes 前两块执行：`:1841-2005` 区间实测 12 处 `payload.get`（非 14），全文件含多行写法 23 处。
- c10（死接口）前提不成立：两个 gateway getter 全仓有 5 处生产调用（MCP 插件 client.rs），serve 模式 MCP OAuth / elicitation 依赖这条链。**不删 sdk setter/getter 与 host.rs:1260-1263 写入点**。本 change 第三块按 proposal 自身条件句（「先证零引用再删」）收缩为证伪落地：零生产代码删除。
- 全局 gateway 的真实坏味道是「每次槽位物化覆盖写」（多槽并发时后物化槽覆盖先前者，last-slot-wins），治理涉及行为权衡，转后续 change，不属本 change。

## 现状证据表

| 断言 | 实测 | 证据（文件:行号 / 命令） |
|---|---|---|
| host.rs 3905 行 | 属实 | `wc -l crates/xylitol-serve/src/occupancy/host.rs` = 3905 |
| `:1841-2005` 14 处 payload.get | **12 处**；其中 trust-mode 臂 `:1841` 不属 mcp/oauth/export-import，三臂实为 11 | `rg -Un 'payload\s*\.get\(' host.rs` |
| 全文件 payload.get | 23 处：单行 12（:1841-:2005）+ 多行 11（:214 / :1120 / :1134 / :1381 / :1413 / :1659 / :1703 / :1720 / :1725 / :1745 / :1751） | 同上（`-U` 多行模式） |
| 错误文案重复 | `"unavailable"` / `"no writer engine"` 全文件各 13 处（区间内 8 处；:3729 / :3834 为测试断言）；`.and_then(Value::as_str).unwrap_or("")` 区间内 7 处 | `rg -n '"no writer engine"' host.rs` |
| pending 裸 Mutex :95-96 | 属实 | host.rs:95-96 |
| 写入点 :1390/:1407/:1422 | 属实，均在 `defer_runtime_setting`（:1374-1426）内 | host.rs:1390 / :1407 / :1422 |
| flush :505-515 | 部分属实：`clear_pending_runtime` :504-511，`flush_pending_runtime` :513-533（take :514-515），引文只盖到 take 行 | host.rs:504-533 |
| defer 分支无直接单测 | 部分属实：set_thinking_level 臂有 `inflight_set_thinking_does_not_apply_until_flush`（但 flush 后未断言生效值）；set_model / cycle_model busy 臂无直接单测（fixture 中 set_model 测试 :1105 / :1221 非 busy 场景） | tests/occupancy/assembly_fixture_tests.rs:1812-1843 |
| sdk 单例 :118/:139 | 属实 | crates/xylitol-plugin-sdk/src/mcp.rs:118 / :139 |
| getter 全仓零调用方 | **不属实**：`mcp_oauth_gateway()` 生产调用 client.rs:1051-1055（缺失即报 "OAuth callback gateway is not available"）/:1204/:1132/:1324/:1357；`mcp_elicitation_gateway()` client.rs:69（缺失即静默 decline） | rg 见「删除验证」 |
| :1260-1261 每次槽位物化覆盖写 | 属实 | host.rs:1260-1263 |
| 写入的 gateway 无消费者 | **不属实**：serve host.rs:1912/1924/1936 → runtime in_process/mod.rs:629-658 → plugin hub.rs:1079-1093 → client.rs:1311 `start_oauth` → :1040 `start_oauth_flow` → :1051 getter。driver 路径的终点就是全局 getter | 上述行号链 |
| HostOAuthGateway 仅服务该路径 | 生产引用仅 host.rs:1261（import :17；定义与 impl oauth.rs:19-47；oauth.rs 自身单测 :177/:189/:197；mod.rs:15 re-export），但该路径是活路径 | `rg -n 'HostOAuthGateway'` |

## 删除验证

gateway 零引用证明（apply 期首个 task 复跑并记录输出；本 design 为深挖期实测）：

    rg -n 'mcp_oauth_gateway\(\)|mcp_elicitation_gateway\(\)' crates plugins tests
    # 生产命中 5 处：plugins/xylitol-official-plugin-mcp/src/client.rs:69/:1051-1055/:1204/:1324/:1357
    # → 零引用证明失败，不删 setter/getter/写入点/HostOAuthGateway
    rg -n 'set_mcp_oauth_gateway|set_mcp_elicitation_gateway' crates plugins tests
    # 生产调用仅 host.rs:1260-1261；测试 plugins/.../assemble.rs:231/:248（#[tokio::test] fixture :223-257）

载荷与 defer 的收敛验证（各 task 判据）：

    rg -Uc 'payload\s*\.get\(' crates/xylitol-serve/src/occupancy/host.rs   # 23 → 0
    rg -n 'pending_model|pending_thinking' crates/xylitol-serve/src           # 0 命中

（occupancy/ws.rs:153/:156 两处 payload.get 不在本 change 范围。）

## 目标形状

块一（载荷类型化，c8）：

- 每臂一个私有 `#[derive(Deserialize)]` payload struct（`#[serde(default)]`，字段缺省容忍度与现状逐字段等价），`serde_json::from_value` 单点解析，解析后分支逻辑不变。
- 顺序照 What Changes：subscribe / prompt 两臂先行（:1699-1756，5 处），随后 mcp / oauth 臂（:1877-1940，8 处），最后 trust-mode（:1841-1848）、stage_payload export-import（:1997-2005）与 defer 两参复用（:1381-1385 / :1413-1417，6 处）。
- 错误单点 = 每臂「一次解析 + 一处校验」，code 与文案逐字沿用现行值。

块二（PendingRuntimeSetting，c9）：

- 新增小状态类型 `PendingRuntimeSetting`（两个 Mutex 字段 + `set_model` / `set_thinking` / `take_all` / `clear` 方法），替换 host.rs:95-96 两字段；调用点迁移：写 :1390/:1407/:1422，clear :504-511 与调用方 :1763，take :513-533，flush 调用方 :1346/:1787 与测试 seam :539-541 不变。
- 补 busy defer 三臂直接单测与 flush 生效断言。

块三（gateway 证伪落地，c10 收缩）：零生产代码删除；证明输出记录进提交说明；「按槽覆盖写」治理转后续。

## 不变量

1. RPC code 与文案逐字不变：既有断言 host.rs:3530 / :3548 / :1387 / :1420 / :3729 / :3834 保持绿。
2. 缺键 → 默认值语义逐字段不变（`Option` 缺省等价 `unwrap_or("")` / `unwrap_or(0)` / `unwrap_or(Value::Null)`；stage_payload 的 null 与缺键等价）。
3. defer 生效时机不变：busy 写 pending、turn 边界 flush（:1346 / :1787）、detach clear（:1763）。
4. MCP OAuth / elicitation 行为不变：gateway seam 全保留，sdk 导出面（crates/xylitol-plugin-sdk/src/lib.rs:84）不动。
5. 公共 API 面不变：不改任何 `pub` 签名（`flush_pending_runtime_for_test` 是 c3402 C17 挂门，:535-541 保留）。

## 测试边界（seam）

- 单测落点：host.rs 既有 tests mod（subscribe / prompt 错误断言 :3530 / :3548 所在）与 tests/occupancy/assembly_fixture_tests.rs（defer fixture 先例 :1812-1843）。seam = `handle_unary` + `materialize_writer` + `mark_run_inflight_for_test` + `flush_pending_runtime_for_test`。
- 新增单测只证明「解析等价 + defer 三臂 + flush 生效」，不改既有断言文本。
- BDD：不改任何场景文本，serve 相关场景全量保持绿。
- 协同：c3402 触 `:535` / `:539` 挂门，本 change T4 触 `:504-533` 相邻区域，落地顺序无论先后，后落者 rebase（照 proposal Impact）。

## 决策记录

| 决策 | 选择 | 理由 |
|---|---|---|
| gateway seam 去留 | 全保留，零删除 | getter 有 5 处生产消费（client.rs:69/:1051/:1204/:1324/:1357）；删除直接破坏 serve MCP OAuth / elicitation，违反 Capabilities「行为全部不变」 |
| `impl Default for HostOAuthGateway`（oauth.rs:40-44） | 保留 | 全仓无 `HostOAuthGateway::default` 调用，但删除会触发 clippy `new_without_default`（:25 `pub fn new`），净删除为零 |
| 载荷类型化载体 | 臂级私有 Deserialize struct + `serde_json::from_value` | crate 内无既有 Deserialize struct 先例（rg 0 命中），但 host.rs:1728 `from_value::<Vec<McpResourceContext>>` 已有同向用法；不新增对 xylitol-protocol 的依赖方向 |
| subscribe / prompt 先行 | 照 What Changes | 宿主最热路径先立模式，其余臂机械跟进 |
| defer 两参解析 | 复用块一 typed payload | defer_runtime_setting :1381 / :1413 与 idle 臂同字段同语义，是同一次收敛点 |
| `"no writer engine"` ×11 收敛 | 不做 | 收敛会触碰 :3729 / :3834 断言与潜在 BDD 文本，与「文案逐字不变」判据冲突；留后续 change |
| 全局 gateway 覆盖写治理 | 不做，转后续 | last-slot-wins 是多槽语义权衡，超出纯实现层 refactor 边界 |
| proposal 计数勘误 | design 按实数记录（12 / 23 / 13 处），不改 proposal | proposal 只读；勘误不影响 What Changes 三块拆分 |
