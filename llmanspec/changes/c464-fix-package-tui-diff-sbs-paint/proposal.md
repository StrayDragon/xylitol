---
change_id: c464-fix-package-tui-diff-sbs-paint
title: "package-tui-diff：SBS 去整行红/绿底，避免与 tool-*-bg 抢语义"
status: purpose-draft
priority: 464
depends_on: ["c459-enhance-diff-edit-line-syntax", "c462-demo-tool-status-bg"]
blocks: ["c456-demo-session-nav-keys", "c491-add-app-tui-session-tree"]
author: agent
track: A
---

# c464-fix-package-tui-diff-sbs-paint

> **status: purpose-draft**（插队；**先不 apply/归档**，等确认方案后再升格）

## Why

产品语义里 **背景 tint = 执行态**（`tool-pending/success/error-bg`，图 3 header 绿底）。
当前 **side-by-side Diff** 对增删行再铺整行红/绿底（图 3 body），与执行态 bg 同属「红/绿墙」，用户会把「改动极性」误读成「成功/失败」。

对照 pi（`coding-agent/.../diff.ts`）：**只有 unified**；增删用 **fg**（`toolDiffAdded/Removed`）+ 紧凑 `±N`；词级用 **inverse**；**无** SBS 整行红绿底。Tool 块另有极淡整块 `tool*Bg`。

图 1（pi edit 视口）展示的是：语法高亮 + gutter 行号 + 增删极性标记；其行底若存在，也不应与 xylitol 的 **tool 执行态 bg** 抢同一语义通道。

**Unified 路径用户认可，本变更不改。**

## Purpose

仅调整 **side-by-side** 着色策略：去掉（或默认关闭）整行 `added_line_bg` / `removed_line_bg`；保留 fg + gutter；词级改用 inverse 或仅 word 级 tint。同步 DESIGN / `diff-block.md` / demo theme，使 **bg = 执行态**、**fg/gutter = diff 极性**。

## What Changes（意向）

1. `render_side_by_side`：增删半栏 **MUST NOT** 默认套全行 `diff-*-bg`（identity / 跳过 `paint_kind_line_bg`）。
2. Unified：`added_line_bg` / `removed_line_bg` **保持现状**（用户确认不改）。
3. SBS 词级：优先 `theme.inverse` 或仅 `word_change_*`，避免再叠整行底。
4. `demo_diff_theme()` / DESIGN：写明 SBS vs unified 的 bg 策略分叉；强调与 `tool-*-bg` 语义分离。
5. harness：SBS 断言 **无** 大面积 truecolor 行底（或仅 header 仍可有 tool tint）；unified 回归绿。

## Alternatives（待确认）

| 方案 | 说明 |
|---|---|
| **A（推荐）** | SBS 默认关行底；fg + `±N` gutter；词级 inverse |
| B | SBS 保留极淡行底，但色相/明度与 `tool-*-bg` 明确错开（仍有「双绿」风险） |
| C | 产品默认强制 unified；SBS 仅实验开关（改动面更大） |

## Capabilities

- `package-tui-diff`（modify）
- 文档：`src/app/tui/design/diff-block.md` / `DESIGN.md`

## Out of scope

- Unified 行底/词级重做
- Session Tree 增强（另见下方笔记 / c456）
- 把 pi 的整块 tool bg 铺回 Diff 正文（我们已选 header-only）

## 参考

- pi Diff：`../pi/packages/coding-agent/src/modes/interactive/components/diff.ts`
- pi Tool bg：`../pi/.../tool-execution.ts`
- xylitol：`packages/xylitol-tui/src/components/diff.rs`（`paint_kind_line_bg` / SBS）
