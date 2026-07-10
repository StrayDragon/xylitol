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

1. `DiffInput::EditText` + 解析 `^([+-\s])(\s*\d*)\s(.*)$`。
2. Unified 对 EditText 用紧凑 `±{pad} {content}`（pi 风格）。
3. SBS：`format_sbs_cell` 使用 `old_no`/`new_no`；空半栏不伪造行号。
4. 可选 `DiffTheme::highlight_line`（默认 identity）；demo 可接 syntect。
5. `agent_demo` edit 样例；单测覆盖解析与 SBS 行号。
6. 更新 `design/diff-block.md`；`PI_DELTAS` 如有新差异则记一行。

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
