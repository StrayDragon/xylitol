---
change_id: c459-enhance-diff-edit-line-syntax
title: "增强 Diff/edit：pi 行号格式 + SBS 行号 + 可选内容语法高亮"
status: purpose-draft
priority: 459
depends_on: ["c451-add-package-tui-diff", "c452-demo-highlight-pipeline"]
author: agent
track: A
---

# c459-enhance-diff-edit-line-syntax

> **status: purpose-draft** — 插队：对齐 pi edit 块观感，并补齐 side-by-side 行号；供 c470 transcript / demo 提前验证。

## Why

### A. pi edit 行号格式

pi coding-agent 的 **edit** 工具结果块（`edit-diff.ts` → `generateDiffString` + `renderDiff`）呈现为：

- 行号与符号粘在一起：`+ 42 content` / `- 11 old` / `  40 context`
- 行级红/绿/dim + 单行对 word-level inverse
- 复制友好、无表线

当前 `package-tui-diff`（c451）已有 unified / side-by-side / word-level，但：

1. 输入偏 `display_diff` gutter（`NNNN NNNN | …`）与 unified；**未一等公民支持 pi edit 的 `±NNNN content` 行格式**。
2. 内容区**无语言语法高亮**（可选经 c452 管线）。

### B. Side-by-side 行号缺口（demo 已暴露）

`render_side_by_side` → `format_sbs_cell(..., _no: Option<u32>, ...)` **丢弃行号参数**，左右栏只有 `±` 无数字。Unified 路径已有 gutter；SBS 必须同等合理：

| 栏 | 行号来源 | 展示意向 |
|---|---|---|
| 左（旧） | `old_no` | 删除/上下文：旧文件行号 |
| 右（新） | `new_no` | 添加/上下文：新文件行号 |
| 空半栏 | — | 空白占位，不伪造行号 |

宽度：按本 hunk / 全文件 max line 做 pad（与 unified `format_gutter` 一致），**MUST NOT** 做成装饰性「行号墙」。

## Purpose

增强 `packages/xylitol-tui` Diff：

1. 解析/渲染 **pi edit 行号格式**；
2. **side-by-side 左右栏均显示合理行号**（与 unified 同源 `old_no`/`new_no`）；
3. 可选经 `highlight` feature 对**内容片段**做语言语法高亮（与 c452 同管线），再叠 diff 行色 / word-level。

`agent_demo` 增加 edit 块样例 + 展开后可验收 SBS 行号。

## What Changes（意向）

1. `DiffInput` 增加 `EditText(String)`（或扩展解析）支持 `^([+-\s])(\s*\d*)\s(.*)$`。
2. 渲染输出对齐 pi：`±{lineNum} {content}`（行号宽度按 max line pad）。
3. **`format_sbs_cell` 使用 `_no`**：每侧 `sign + padded_lineno + content`（或 `lineno + sign`，与 unified 视觉对齐后定一种）；空半栏不加伪行号。
4. 可选 `DiffOptions::syntax_lang` / `highlight_content` 回调：对 content 跑 `highlight_code`（单行），再套 added/removed/context 色；超限回退纯文本（复用 c452 安全上限）。
5. `agent_demo`：可展开 edit 块样例；SBS seed 展开后断言左右行号可见。
6. 更新 `design/diff-block.md` MUST；`PI_DELTAS` 记一行（若有新刻意差异）。

## Capabilities

- `package-tui-diff`（modify）

## Soft depends / 解锁

- 解锁更接近产品的 c470 transcript edit 呈现
- 可与 c453 expandable 策略并行
- 工具块 **bg 状态色**（`tool-*-bg`）属产品 theme / transcript，**不在本变更实现**（见主 `DESIGN.md` Colors；另案接线）

## Out of scope

- 改 `infra` `generate_display_diff` 输出格式（可另变更对齐）
- 产品面 `src/app/tui` 接线（仍走 c470）
- 把 syntect 打进默认非 highlight 构建
- 实现 tool pending/success/error 全行背景（token 已入 DESIGN，实现另案）

## Promote trigger

demo / c470 需要 edit 块行号+内容高亮，或 SBS 行号验收不通过时升格 full。
