---
change_id: c570-add-package-tui-theme-palette
title: "package-tui-theme：Dark/Light 语义色板 + paint + demo 全量换肤 + 探测接线"
status: full
priority: 570
depends_on: []
author: agent
track: P
---

# c570-add-package-tui-theme-palette

## Why

`xylitol-tui` 组件已是闭包注入 `*Theme`，但 demo / DESIGN 侧真彩色板散落硬编码 SGR；`theme_auto` 仅换 footer muted，markdown/diff/tool-bg 仍固定 Mocha。需要**仅两套**（Dark 默认 + Light）的可扩展色板与工厂，以及 OSC11/CSI 探测的可测试接线，且不引入 coding-agent 式 JSON 主题市场。

## Purpose

在包内提供语义 `SemanticPalette`（Dark=Mocha/DESIGN、Light=Latte 对齐）+ `paint`（truecolor fg/bg，39/49 复位）+ 组件主题工厂；`agent_demo` 随 `theme_mode` 全量换肤；opt-in 探测（COLORFGBG + OSC/CSI 查询与 host feed）。产品面仍固定暗色 MVP。

## What Changes

1. 新模块 `packages/xylitol-tui/src/theme/`：`paint`、`SemanticPalette::{dark,light,for_scheme}`、markdown/diff/choice 等工厂。
2. `terminal_colors`：导出 OSC11 / CSI 996 查询常量；可解析 reply 的 host 辅助（feed into `resolve_terminal_color_scheme`）。
3. `agent_demo`：所有 chrome 主题经 palette 工厂；`theme_mode` 切换时重建；`XYLITOL_AGENT_DEMO_THEME_AUTO=1` 时写查询 + harness `feed` 更新。
4. `theme-tokens.md` / DESIGN 文档：补 Light 映射与「仅两套」约定。
5. 单测：palette 工厂非空闭包、Dark≠Light、探测优先级 + demo harness 换肤。

## Capabilities

- `package-tui-theme`（新增）

## Impact

- `packages/xylitol-tui/src/theme/**`、`terminal_colors.rs`、`lib.rs`
- `packages/xylitol-tui/examples/agent_demo.rs` + 相关 tests
- `src/app/tui/design/theme-tokens.md`（及必要时 DESIGN 指针）

## Out of scope

- 产品 `src/app/tui` 主题切换 / `/theme` 命令
- JSON 主题文件、热重载、主题市场
- ImageTheme / 51-token Theme class 整移植
- Frappé / Macchiato 等多 flavor（扩展点保留，本变更只交付两套）

## Ethics

- risk_level: low
- prohibited_actions: 不改产品 host 默认暗色；不把主 crate agent 类型引入包
- required_evidence: `llman sdd validate`；`cargo test -p xylitol-tui` 相关绿；demo `THEME_AUTO` 可手验 light chrome
