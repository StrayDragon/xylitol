---
depends_on: []
title: TUI 引擎能力接入：键位冲突、OSC8、性能面
needs_specs_change: true
branch: sdd/c3140-add-tui-engine-quick-wins
base_branch: main
base_sha: 7a1d7fc061bdbc2ee25de68bae95fd3298ba933f
---

# TUI 引擎能力接入：键位冲突、OSC8、性能面

## Why

`xylitol-tui` 已有几项产品端未接的能力：键位冲突探测（`get_conflicts`）、OSC 8 超链接（`hyperlink`）、渲染性能快照（`RenderPerfSnap`）。产品构建了 keybindings manager 但从不查冲突；markdown 链接只印 `label (url)`，终端不可点；没有 `/perf` 自助诊断。另：mermaid 默认 `Final` 隐式生效但零产品测试/文档；`ActionRow` 全仓零消费者。

## What Changes

- 键位冲突提示：TUI 启动 / `/reload` 后查 `get_conflicts()`；有用户覆盖冲突时推一条耐久滚动提示（含 key 与动作 id），不阻塞、不伪造错误。默认表已知冲突（如 ctrl+c）不在用户未覆盖时告警，避免噪声。
- OSC 8 超链接：引擎 markdown 增加 `hyperlink_links` 开关；产品默认开；链接渲染为 OSC8 可点文本，不支持 OSC8 的终端自动忽略。保留 `label (url)` 作为关闭时回退。
- `/perf`：slash 命令读取 `TUI::last_render_perf()`，以滚动提示输出 `do_render_us` / `component_lines` / `paint_lines` / `ao_reprojected` / `ao_vertical_shifted`；不新增常驻面板。
- mermaid 护栏：产品渲染显式设 `MermaidFenceMode::Final`；补一条产品 snapshot（mermaid fence）与文档说明，避免引擎默认漂移无声改产品输出。
- ActionRow 清理：确认仍零消费者后从 `xylitol-tui` 公共 API 删除 `ActionRow` / `ActionChip` / `ActionRowTheme`；若 c3130 决定用它承载 chip 行则改为接线（二选一，apply 时确认）。

## Capabilities（拟）

- `app-tui-input`：键位冲突提示（用户覆盖冲突）。
- `app-tui-transcript`：markdown 链接 OSC8 可点。
- `app-tui-commands`：`/perf` 输出。
- `package-tui-*`：`hyperlink_links` 开关；ActionRow 删除或接线；mermaid 显式 mode。
- `runtime-config`：无新增配置；OSC8 默认开，若需关闭走 `no_color`? 设计取默认开、无配置。

## Impact

`crates/xylitol-app-tui` 启动 / reload / commands / markdown theme；`crates/xylitol-tui` markdown / action_row / lib re-export。不改 Host / 协议 / 折叠状态机。

## 非目标

- 不做终端能力探测 / OSC11 主题自动（DESIGN 明确不做）。
- 不做图片内联渲染（现有粘贴图片走临时文件路径）。
- 不做 Tabs / CancellableLoader / fd 补全（c3145）。
- 不做常驻性能面板 / 快捷键墙。
- 不把默认键位冲突做成阻断错误。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
