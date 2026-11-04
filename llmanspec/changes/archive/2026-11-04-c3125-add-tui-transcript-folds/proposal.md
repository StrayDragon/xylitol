---
depends_on:
  - c3120-update-tui-activity-display
title: TUI 工具参数折叠层与用户长消息折叠
needs_specs_change: true
branch: sdd/c3125-add-tui-transcript-folds
base_branch: main
base_sha: db4d76ab587ceace23256ecd80907536e3df0e36
---

# TUI 工具参数折叠层与用户长消息折叠

## Why

工具块 header 只有一行人类摘要，但没有独立的参数折叠层：bash 长命令会整段换行，展开簇后占满屏幕；MCP args JSON 进 output 也没有上限。用户长消息提交后全文铺开，80 行输入会占 80 行 scrollback，把上一轮内容挤出屏幕。两者都需要「默认缩略 + 点击展开」的端侧折叠，且复用现有 expandable output / fold hit 基础设施。

## 术语

- **工具参数层**：工具块 header 下的一层可折叠正文；bash 存完整命令，MCP 存 pretty args JSON；默认 hidden 或 5 行 preview。
- **用户长消息折叠**：用户消息默认只显示前 N 个视觉行 + 展开提示；点击展开全文。
- **视觉行**：按终端宽度换行后的行；不是逻辑换行。

## What Changes

- `UiEntry::Tool` 增加 `args_full: Option<String>`：`ToolExecutionStart.args` 或历史 ToolCall arguments 填充；bash 取 `command`，MCP 取 pretty JSON，其他工具按需。
- 工具 header 改为单行截断：`▾ Bash $ <首行>` / MCP 只画工具名 / 文件族只画路径；不再 `push_wrapped` 整段。
- 新增 args viewport：默认按 `visible_items.tool_args`（hidden / preview / full）；preview 5 行 head + `... (N more lines, click to expand)`；点击展开/收起；复用 `render_expandable_output`。
- 新增 `FoldTarget::ToolArgs(id)` 与命中表；`ScrollbackFold` 增加 `tool_args_mode` + per-id overrides。
- 用户消息折叠：`visible_items.user_message`（preview / full）；preview 默认 3 个视觉行 + `... (N more lines, click to expand)`；点击展开；新增 `FoldTarget::User(key)`，key 用 entry index + 文本 hash，resume/travel 稳定。
- 缓存指纹纳入 args 展开态与 user 展开态；历史重建填 args_full；不往返 Host。
- 配置扩展：`tui.activity_fold.visible_items.tool_args` 与 `.user_message`；默认 `hidden` / `preview`。

## Capabilities（拟）

- `app-tui-transcript`：工具参数层与用户长消息折叠；新增规则与场景；r1343 保持「header 摘要不得携带完整 args JSON」。
- `runtime-config`：`visible_items.tool_args` / `visible_items.user_message` 字段与默认。
- `app-tui-mouse-actions`：新折叠目标与点击命中；沿用现有折叠表语义。

## Impact

`bridge/mod.rs` / `bridge/session_tree.rs` / `widgets/scrollback/{paint,live,mod,cache,diff}.rs` / `widgets/fold_hit.rs` / config / harness / snapshot。不改 Host 存储格式；args_full 只来自既有 ToolCall 参数。

## 非目标

- 不做工具输出全文无上限展开（diff 仍有 80 行 cap；output 仍有 `[Full output:` 硬截断）。
- 不做用户消息的富文本编辑 / 重发。
- 不做 MCP args 的结构化表单；只做文本折叠。
- 不做跨会话持久化逐块展开态。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
