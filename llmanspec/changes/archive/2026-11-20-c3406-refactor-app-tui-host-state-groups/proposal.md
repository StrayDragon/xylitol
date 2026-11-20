---
depends_on: []
title: "HostSession 状态字段按组类型化（照 HostBusy 范式）"
needs_specs_change: false
branch: sdd/c3406-refactor-app-tui-host-state-groups
base_branch: main
base_sha: 6c6678de4c2502716b80cd4ff879aa83c9eb7f8e
---

# HostSession 状态组类型化

## Why

arch-review c12。HostSession 41 个字段平铺（crates/xylitol-app-tui/src/host/mod.rs:82-124，含 2 个 `#[cfg(test)]` 字段），其中三组内聚字段以裸字段暴露、语义靠字段名猜：

- stream-paint 组：paint_dirty / stream_sync_pending / suppress_xy_until_stream_end（:95 / :99-100）
- stage-name 组：stage_name_policy / stage_name_attempted / stage_name_request_pending（:90-92）
- footer-token 组：footer_token_gen / footer_token_tx / footer_token_rx / last_mid_turn_footer_refresh / run_applied_token_settlement / pending_settlement_estimate（:107-112）

HostBusy（:96）已示范「小状态类型 + 组内转移」的形状，三组未跟进。在 190 个 pub fn 的宿主上，字段分组是局部性与 AI 可导航性成本最低的改善：改 paint 策略只动一个类型，不再在散字段间找齐。

## What Changes

- 三组各收成小状态类型（私有字段 + 组内方法，照 HostBusy 形状），HostSession 持组值；逐组一提交。
- 不拆文件、不动方法归属、不改任何可观察行为。

## Capabilities

- 无条款变更（needs_specs_change: false）：呈现与交互行为不变，属代码组织演进。

## Impact

- 代码：crates/xylitol-app-tui/src/host/mod.rs（组类型可落 host/ 下小组件文件）。
- 协同：c3400 触碰 app-tui 测试护栏（harness / tests / snapshots；host/mod.rs 只读），本 change 只动 host/mod.rs 内部字段组织，冲突面小。
