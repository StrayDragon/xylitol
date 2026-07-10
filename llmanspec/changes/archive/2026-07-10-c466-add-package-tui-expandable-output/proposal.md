---
change_id: c466-add-package-tui-expandable-output
title: "包 ExpandableOutput：工具详情 max-height 视口 + Ctrl+O"
status: full
priority: 466
depends_on: []
author: agent
track: A
---

# c466-add-package-tui-expandable-output

> 实现已在 `agent_demo` / `ExpandableOutput` 冒烟；本变更补契约并归档。

## Why

长工具/bash 输出若整段展开会冲垮 transcript。pi coding-agent 用 max-height 视口：默认贴尾 N 行 + `... (N earlier lines, ctrl+o to expand)`；Ctrl+O 全文；流式时折叠态仍贴尾。

## Purpose

包提供 `ExpandableOutput` / `render_expandable_output` + wrap-aware `truncate_to_visual_lines`；demo/产品在 Tool 详情层使用；全局 Ctrl+O 切换视口（与 Alt+E 块展开正交；树开时 Ctrl+O 让给 filter cycle）。

## What Changes

1. `packages/xylitol-tui`：`ExpandableOutput`、`ExpandableOutputOptions`（max_preview_lines / Tail|Head / expand_hint）。
2. `utils::truncate_to_visual_lines`（ANSI wrap 预算）。
3. `agent_demo`：Tool 详情走视口；seed 长 bash + StreamingBash；Ctrl+O / Ctrl+Shift+O。
4. 文档：`design/expandable.md` §详情视口、`keybindings.md`。

## Capabilities

- `package-tui-expandable-output`

## Out of scope

- Diff 块视口（另议）
- 产品 `UiRoot` 接线（随 bridge / tool 块）
- 每块独立 expand 态（当前全局 `tools_output_expanded`，对齐 pi）
