---
change_id: c1140-add-package-tui-thinking-level-border
title: "xylitol-tui / agent_demo：thinking level 编辑器边框色"
status: draft
priority: 1140
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: thinking
domain: package-tui
ethics:
  risk_level: low
  prohibited_actions:
    - 把产品壳 / Driver / domain::ThinkingLevel 依赖塞进 xylitol-tui
    - 在本变更接线 src/app/tui 产品边框或 footer
    - 引入 /settings 面板
  required_evidence:
    - 包测：level→border 色可区分；set_border_color 应用后 render 边框变色
    - agent_demo harness：cycle 边框可测
  escalation_policy: 边框 vs footer 回退由 c1150；模型专属 level 枚举由 c1145
---

# c1140-add-package-tui-thinking-level-border

## Why

pi 用编辑器边框色表示 thinking level。产品不做 `/settings`。能力应先在 `packages/xylitol-tui` + `agent_demo` 验证，再进 app TUI（c1150）。

## Purpose

分层（本变更只覆盖前两层）：

1. **库 `xylitol-tui`**：提供 level→边框色映射（包内枚举/字符串，**零**主 crate 依赖）+ 经既有 `Editor::set_border_color` 应用
2. **`agent_demo`**：`Shift+Tab`（主）或 plate/slash **cycle** 验证边框变色
3. **`src/app/tui`**：**本变更不做** → `c1150`（消费本 API + c1145 模型 levels；产品绑 `Shift+Tab`，不加 `/thinking-level` slash）

## What Changes

- `Palette`（或等价）按 thinking border level 返回边框 `RgbColor` / paint 闭包
- 包内 `ThinkingBorderLevel`（或等价）：至少覆盖 `off|minimal|low|medium|high`；可 `cycle_next`；**MUST NOT** 引用 `xylitol::domain`
- `agent_demo`：cycle 入口 + 应用边框；footer/status 可短标当前 level（可选）
- 包测 / agent_demo harness
- delta：`package-tui-theme` · `package-tui-editor` · `package-tui-agent-demo`

## Capabilities

- `package-tui-theme`（modify）
- `package-tui-editor`（modify）
- `package-tui-agent-demo`（modify）

## Out of scope

- `/settings`
- 产品 `src/app/tui` 边框/footer 接线（c1150）
- 模型配置枚举 xhigh/max（c1145）
- 改 bash 边框语义（success 边框仍优先于 thinking，见 design）

## Impact

- demo 可目视/自动验证 thinking 边框；产品面待 c1145+c1150
