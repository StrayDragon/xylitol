---
depends_on: []
title: TUI 活动折叠等级预设与子项可见性
needs_specs_change: true
---

# TUI 活动折叠等级预设与子项可见性

## Why

用户选择「summary + 流式一行 Thinking」作为默认，但不同场景需要不同密度：回看长会话要全部折叠，盯当前轮要最近一轮展开，排查要全展开。当前只有 `keep_recent_turns` / `stream_collapse` 等零散字段，没有一等「显示等级」；流式 thinking 又被 spec r1372/r1364 锁定为默认展开，导致首个工具意图出现时整块从 N 行折成 1 行，视觉一跳。用户还要求保留像 opencode 那样的**逐子项可见性**覆盖（thinking / tool header / output / diff …），预设只是默认包。

## 术语

- **显示等级**：quiet / summary / recent / full 四档预设，决定已结束轮折叠、当前轮 thinking 与工具块的默认密度。
- **子项可见性**：thinking / tool_header / tool_output / diff 等逐项模式（hidden / header|summary / preview / full）；显式配置或运行时开关覆盖预设。
- **流式 thinking 一行**：流式期间只画一行 `▸ Thinking · <最新一行截断>`，高度稳定；结束后变 `▸ Thought Ns`。

## What Changes

- 新增 `tui.activity_fold.display_level`：quiet / summary / recent / full，默认 `summary`。
- 新增 `tui.activity_fold.visible_items`：`thinking`（hidden / header / full）、`tool_header`（hidden / summary / full）、`tool_output`（hidden / preview / full）、`diff`（hidden / preview / full）；c3125 再扩 `tool_args` / `user_message`。
- 预设默认：
  - quiet：keep_recent_turns=0、collapse_floor=L3、thinking=header、tool_header=summary、tool_output=hidden、diff=hidden。
  - summary（默认）：keep_recent_turns=2、collapse_floor=L3、thinking=header、tool_header=summary、tool_output=preview、diff=preview。
  - recent：keep_recent_turns=1、collapse_floor=L3、thinking=full、tool_header=summary、tool_output=preview、diff=preview，轮末展开最近簇。
  - full：keep_recent_turns=∞ / auto 关闭、thinking=full、tool_header=full、tool_output=full、diff=full，全部簇展开。
- 优先级：显式 `stream_collapse` / `keep_recent_turns` / `visible_items` 覆盖预设；运行时开关写逐项覆盖；`/fold reset` 清覆盖回到预设。
- 流式 thinking 默认改 `header`（summary / quiet）：一行预览，高度稳定；recent / full 为 `full`；用户可点开/折。
- 新增动作 `app.activity.displayLevel`（默认无和弦，进 r3032 目录）；`/fold` slash：`/fold quiet|summary|recent|full`、`/fold reset`、`/fold <item> <mode>`。
- 修复 `enabled=false` 与 `paints_envelope_header` 的语义漂移：明确 full 档的展开方式；不再让 `collapse_floor=L2` 时信封头全消失。
- 折叠仍是端侧纯呈现：状态留客户端，不往返 Host，不要求 Host 持久化。

## Capabilities（拟）

- `app-tui-transcript`：显示等级 / 子项可见性 / 流式 thinking 一行；改 r1356、r1372、r1364，新增预设与可见性规则。
- `runtime-config`：`display_level` / `visible_items` 字段与默认；改 r1739。
- `app-tui-mouse-actions`：`app.activity.displayLevel` 进 r3032；chip 见 c3130。
- `app-tui-commands`：`/fold` 语义。

## Impact

`ActivityFoldSettings` / `ActivityFoldState` / `ScrollbackFold` / `widgets/scrollback/{mod,live,paint}.rs` / config 类型 / 命令解析 / 键位目录 / harness 与 BDD。不改 Host 折叠持久化；不改 tool/args 渲染（c3125）。

## 非目标

- 不做工具 args 明细层与用户长消息折叠（c3125）。
- 不做输入区 chip / 字形 / 圆角 / spinner（c3130）。
- 不做阶段名（c3110 / c3115）。
- 不做跨会话持久化显示等级（端侧会话级即可；配置文件是默认值）。
- 不改变折叠命中表与点击语义（沿用 `FoldTarget`）。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
