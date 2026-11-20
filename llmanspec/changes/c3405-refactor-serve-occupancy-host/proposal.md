---
depends_on: []
title: "serve occupancy host 加厚：载荷类型化、defer 设置状态化与死接口清理"
needs_specs_change: false
---

# serve occupancy host 加厚

## Why

arch-review c8 / c9 / c10（c13a 归 c3402 C17 范围、c13b 由 c3401 的 DriverAssembly 承担，本 change 不再包含）。host.rs 3905 行分派巨臂的三处薄点：

- **载荷手撕（c8）**：14 处 `payload.get(...)` 手工解析（mcp / oauth / export-import 臂，host.rs:1841-2005），同一「缺键→默认值 / 报错」模式逐臂手写，错误文案字面量重复；违反根 AGENTS「优先穷举类型 / 注册表」。
- **defer 惯用法三写（c9）**：busy 时 set_model / cycle_model / set_thinking_level 的「存 pending → flush take」散在 pending_model / pending_thinking 两个裸 Mutex 字段（host.rs:95-96）+ 写入点（:1390 / :1407 / :1422）+ flush（:505-515）；defer 分支无直接单测。
- **死接口（c10）**：sdk mcp.rs OAUTH_GATEWAY / ELICITATION_GATEWAY 全局单例（mcp.rs:118 / :139）的 getter 全仓零调用方；host.rs:1260-1261 每次槽位物化覆盖写——写入的 gateway 无消费者（MCP OAuth 实际经 OccupancyEngine 的 mcp_oauth_* driver 方法走）。

## What Changes

- subscribe / prompt 两臂先行载荷类型化（类型对齐 protocol 词表，错误单点），其余 payload.get 臂按同模式跟进。
- 新增 PendingRuntimeSetting 小状态类型收编 pending_model / pending_thinking 与 flush 惯用法；补 busy defer 三臂直接单测。
- 删 sdk 两个死 gateway setter / getter 与 host.rs 写入点；HostOAuthGateway 先证零引用再一并删（若 oauth.rs 仅服务该死路径）。

## Capabilities

- 无条款变更（needs_specs_change: false）：解析结果、错误文案、defer 生效时机、MCP OAuth 行为全部不变。

## Impact

- 代码：crates/xylitol-serve（occupancy/host.rs、可能 occupancy/oauth.rs）、crates/xylitol-plugin-sdk（mcp.rs）。
- 协同：host.rs 与 c3402（C17 `*_for_test` 挂门，:535 / :539）改不同区域；落地顺序无论先后，后落者 rebase。

## Open Questions

- HostOAuthGateway / oauth.rs 删除范围待零引用验证后定（apply 期首个 task）。
