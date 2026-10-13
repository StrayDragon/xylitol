---
depends_on: []
title: 侧栏标准档门槛与窄档覆盖弹出框降级
needs_specs_change: true
---

## Why

c2910 把侧栏宽度定为常量 24 后，40–47 列窄终端开侧栏正文只剩 15–23 列，
不可读。主内容（对话区）优先于辅助面板；窄档的正确形态是换形态而非挤着塞。
宽度档位表（c2520）已落，侧栏这类 width-keyed 决策应消费档位表而非裸列数
（tier.rs 文档纪律）。另有一处双口径隐患：`session_sidebar_reserved_cols`
只看开关不看宽度，与 `session_sidebar_layout` 无单一真值源绑定。

## What Changes

- 侧栏门槛：`session_sidebar_layout` 挂 `width_tier >= Standard`（≥80 列，
  物理宽口径）——标准档以下不渲染侧栏带（状态保留，回宽自愈）。
- 窄档降级：标准档以下按 `alt+u` 降级为打开覆盖弹出框（信息不丢、形态切换；
  Esc 收起走 c2914）。降级决策在 host drain（pending_ui），root 保持无宽度状态。
- 口径统一：`session_sidebar_reserved_cols(width)` 改由 `session_sidebar_layout`
  推导；drain 对账带终端物理宽，消除双口径。
- 同步件：designing `session-sidebar` draft/intent、`src/app/tui/DESIGN.md`
  档位表补侧栏行。

## Capabilities

- `app-tui-fixed-zone`（r2904 增补门槛与降级条款 + 场景）

## Impact

`src/app/tui/layout/root/mod.rs`、`src/app/tui/host/session_ops.rs`、
`src/app/tui/effects/pending_ui/mod.rs`、`tests/bdd/steps_app_tui_host.rs`、
`tests/bdd/bindings_app_tui_host.rs`、designing 与 DESIGN.md；无迁移。
