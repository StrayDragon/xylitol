---
change_id: c462-demo-tool-status-bg
title: "demo/产品：工具块 pending/success/error 全行背景三态"
status: purpose-draft
priority: 462
depends_on: ["c453-demo-conditional-display"]
blocks: ["c470-add-app-tui-transcript"]
author: agent
track: A
---

# c462-demo-tool-status-bg

> **status: purpose-draft** — 仅提案；token 已入 `DESIGN.md`，实现等验收后再升格。

## Why

pi coding-agent 用极淡的 **背景 tint**（非仅 fg）表达工具块状态：

| 状态 | pi token | xylitol token（`DESIGN.md`） |
|---|---|---|
| 运行中 | `toolPendingBg` | `{colors.tool-pending-bg}` |
| 成功 | `toolSuccessBg` | `{colors.tool-success-bg}` |
| 失败 | `toolErrorBg` | `{colors.tool-error-bg}` |

包侧已有 `apply_background_to_line`；缺的是 **demo / 产品壳** 按工具结果切换 `bgFn`，以及 bg-only reset（`\x1b[49m`）不冲掉内容 fg。

## Purpose

在 `agent_demo`（及日后 c470 transcript）为 tool / edit 块套三态全行背景；色值以 `DESIGN.md` 为准（Mocha tint，可微调）。

## What Changes（意向）

1. demo：tool 块 pending → success/error 切换时更新 `Box`/`Text`/`Markdown` 的 bg 闭包。
2. 映射层：`{colors.tool-*-bg}` → truecolor/256 bg ANSI（见 `design/theme-tokens.md`）。
3. 验收：肉眼可见浅绿/浅红 tint；复制内容仍可读（bg 不进纯文本语义）。
4. 回写 `design/expandable.md` 锁定已验证策略。

## Capabilities

- `app-tui` / demo 壳（modify；不新增包 capability）
- 可选：若需包级 `ToolBlock` 壳再开 `package-tui-*`（默认 **应用面** 组合 Box+bg）

## Soft depends / 解锁

- **depends_on** `c453`：共用 expandable 工具块状态机
- **blocks** `c470`：产品 transcript 工具呈现应复用同一三态，避免再手搓
- 与 **c459** 并行：c459 管 Diff 行号/edit 格式；本变更管块级 bg

## Out of scope

- 实现本提案（升格 full 后另 apply）
- bash 模式边框色（pi 用 fg，见 `bash-mode.md`）
- 用户主题切换 UI（c458）

## Promote trigger

demo 需要工具成功/失败 bg 肉眼验收，或 c470 接线前升格 full。
