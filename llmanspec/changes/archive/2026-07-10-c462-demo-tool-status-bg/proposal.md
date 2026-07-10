---
change_id: c462-demo-tool-status-bg
title: "demo/产品：工具块 pending/success/error 全行背景三态"
status: full
priority: 462
depends_on: ["c453-demo-conditional-display"]
blocks: ["c470-add-app-tui-transcript"]
author: agent
track: A
---

# c462-demo-tool-status-bg

## Why

pi coding-agent 用极淡的 **背景 tint**（非仅 fg）表达工具块状态：

| 状态 | pi token | xylitol token（`DESIGN.md`） |
|---|---|---|
| 运行中 | `toolPendingBg` | `{colors.tool-pending-bg}` `#313244` |
| 成功 | `toolSuccessBg` | `{colors.tool-success-bg}` `#24352a` |
| 失败 | `toolErrorBg` | `{colors.tool-error-bg}` `#352428` |

包侧已有 `apply_background_to_line`；缺的是 **demo / 产品壳** 按工具结果切换 bg，以及 bg-only reset（`\x1b[49m`）不冲掉内容 fg。

> 与 pi Edit 块「完全一致」的观感（整块浅绿/浅红 **bg** + fg）依赖本变更；c459 只保证 Diff 行格式/对齐。

## Purpose

在 `agent_demo` 为 tool / edit(diff) 块套三态全行背景；色值以 `DESIGN.md` 为准；为 c470 transcript 留下可复用模式（应用面组合，不进包通用 Chat）。

## What Changes

1. demo：`ToolBlockStatus { Pending, Success, Error }` 挂在 Tool / Diff 条目上。
2. 渲染：块内每行经 `apply_background_to_line` + truecolor `\x1b[48;2;R;G;B m…\x1b[49m`（只重置 bg）。
3. 脚本：工具/Edit 可先 Pending 再翻 Success；seed 含 success（及可选 error）样例。
4. 验收 harness：视口含对应 `48;2;…` 序列。
5. 回写 `design/expandable.md`：三态策略标为已在 demo 验证。

## Capabilities

- `app-tui-transcript`（modify：工具块 MUST 按状态套全行 bg）

## Soft depends / 解锁

- **depends_on** `c453`：共用 expandable 工具块状态机（draft 可并行；本变更不阻塞于其落地）
- **blocks** `c470`：产品 transcript 复用同一三态，避免再手搓
- 与 **c459** 并行：c459 管 Diff 行；本变更管块级 bg

## Out of scope

- bash 模式边框色（见 `bash-mode.md`）
- 用户主题切换 UI（c458）
- 把 Mocha 色值硬编码进 `xylitol-tui` 包默认主题（色值属产品 `DESIGN.md`；demo 本地常量对齐）
