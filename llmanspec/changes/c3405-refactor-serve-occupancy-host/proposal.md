---
depends_on: []
title: "serve occupancy host 加厚：载荷类型化、defer 设置状态化与 gateway 链路证据固化"
needs_specs_change: false
---

# serve occupancy host 加厚

## Why

arch-review c8 / c9 / c10（c13a 归 c3402 C17 范围、c13b 由 c3401 的 DriverAssembly 承担，本 change 不再包含）。host.rs 3905 行分派巨臂的三处薄点：

- **载荷手撕（c8）**：`payload.get(...)` 手工解析区间（mcp / oauth / export-import 臂，host.rs:1841-2005）单行口径 12 处、全文件含多行写法 23 处，同一「缺键→默认值 / 报错」模式逐臂手写，错误文案字面量重复（`"unavailable"` / `"no writer engine"` 全文件各 13 处）；违反根 AGENTS「优先穷举类型 / 注册表」。
- **defer 惯用法三写（c9）**：busy 时 set_model / cycle_model / set_thinking_level 的「存 pending → flush take」散在 pending_model / pending_thinking 两个裸 Mutex 字段（host.rs:95-96）+ 写入点（:1390 / :1407 / :1422）+ flush（:505-515）；defer 分支无直接单测。
- **gateway（c10，深挖证伪原判）**：sdk mcp.rs OAUTH_GATEWAY / ELICITATION_GATEWAY 全局单例（mcp.rs:118 / :139）的 getter **有 5 处生产消费**（`plugins/xylitol-official-plugin-mcp/src/client.rs:69` / :1051-1055 / :1204 / :1324 / :1357）；链路为 serve host.rs:1912/1924/1936 → runtime `driver/in_process/mod.rs:629-658` → plugin `hub.rs:1079-1093` → client.rs:1051。删除会破坏 serve 模式 MCP OAuth（报错）与 elicitation（静默 decline）。「死接口」原判不成立；「全局单例按槽覆盖写」（host.rs:1260-1263，last-slot-wins）的多槽语义问题转后续 change。

## What Changes

- payload.get 手撕全量收敛（23 处）：subscribe / prompt 两臂先行载荷类型化（类型对齐 protocol 词表，错误单点），mcp / oauth / trust / export-import / defer 臂按同模式跟进。
- 新增 PendingRuntimeSetting 小状态类型收编 pending_model / pending_thinking 与 flush 惯用法；补 busy defer 三臂直接单测。
- gateway 不删（深挖证伪「死接口」）：以 rg 输出固化 getter 消费链证据（T5），`HostOAuthGateway` 与两个全局单例原样保留；「按槽覆盖写」多槽语义转后续 change。

## Capabilities

- 无条款变更（needs_specs_change: false）：解析结果、错误文案、defer 生效时机、MCP OAuth 行为全部不变。

## Impact

- 代码：crates/xylitol-serve（occupancy/host.rs）。`xylitol-plugin-sdk` mcp.rs 与 occupancy/oauth.rs 经深挖证伪为活路径，不改。
- 协同：host.rs 与 c3402（C17 `*_for_test` 挂门，:535 / :539）改不同区域；落地顺序无论先后，后落者 rebase。

## Open Questions

- ~~HostOAuthGateway / oauth.rs 删除范围~~（已确认，用户 2026-11-20）：证伪不删；「全局 gateway 按槽覆盖写」多槽语义转后续 change。
