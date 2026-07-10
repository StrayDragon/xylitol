---
change_id: c459-enhance-diff-edit-line-syntax
title: "增强 Diff/edit：pi 行号格式 + SBS 行号 + 可选内容语法高亮"
status: full
priority: 459
depends_on: ["c451-add-package-tui-diff", "c452-demo-highlight-pipeline"]
blocks: ["c470-add-app-tui-transcript"]
author: agent
track: A
---

# c459-enhance-diff-edit-line-syntax

## Why

### A. pi edit 行号格式

pi `generateDiffString` / `renderDiff`：`+ 42 content` / `- 11 old` / `  40 context`。

### B. Side-by-side 行号缺口

`format_sbs_cell` 丢弃 `_no`；unified 已有 gutter，SBS 必须同等合理。

### Crate 调研（结论：自研渲染，复用已有 diff/高亮）

| Crate / 项目 | 角色 | 是否采用 |
|---|---|---|
| **`similar`（已依赖）** | 行级 / 词级 diff | **继续用** — 已满足 LinePair + word-level |
| **`syntect` + `two-face`（highlight feature）** | 语法高亮 | **继续用** — 经可选 `highlight_line` 闭包注入内容，不新增依赖 |
| `imara-diff` / `diffy` | 另一套 diff 算法 | **不采用** — 与 similar 重叠，迁移无收益 |
| `deff` / `asd` / `git-ui` | 完整 ratatui Diff TUI 应用 | **不采用** — 绑定 ratatui、非库 API、与 xylitol-tui Component 模型冲突 |

**结论**：无合适的「可嵌入 xylitol-tui、主题闭包、无 ratatui」成熟 Diff UI crate；**自研** EditText 解析 + SBS 行号 + 可选高亮钩子。

## Purpose

增强 `packages/xylitol-tui` Diff：pi edit 行格式、SBS 左右行号、可选内容高亮钩子；`agent_demo` 样例验收。

## What Changes

1. `DiffInput::EditText` + `DiffInput::from_edit_pair` / `generate_edit_text`（pi 兼容）。
2. **默认** `compact_line_numbers: true`：`±{pad}N {content}`，符号与行号同色、内容列对齐（修双 gutter「错位」）。
3. SBS：内容打包列宽；多行 replace hunk（DD…II…）按行 zip 成 L|R（`take_change_hunk`），无需新 crate。
4. 可选 `DiffTheme::highlight_line`（默认 identity）。
5. `agent_demo`：seed 样例 + **模拟 Edit 工具步骤**（整块弹出，非打字机）；Edit 固定 unified。
6. 更新 `design/diff-block.md`。pi 整块 bg/fg 对齐记在 **c462**（不在本变更硬凑）。

## Capabilities

- `package-tui-diff`（modify）

## Soft depends / 解锁

- **blocks** `c470`：transcript edit/diff 呈现应对齐本增强
- 与 **c462**（工具 bg）并行，互不阻塞

## Out of scope

- 改 `infra` `generate_display_diff` 输出格式
- 产品面 `src/app/tui` 接线（c470）
- 把 syntect 打进非 highlight 构建
- 工具块 bg（c462）
