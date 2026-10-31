---
depends_on: []
title: 待办栏改为单行摘要与按需展开
---

# 待办栏改为单行摘要与按需展开

## Why

现行待办栏在队列条与通知条之间画两条满宽 `─` 轮廓，把滚动区与编辑区横向切开，在 TUI 观感上显得割裂。三区各带一个可点的折叠三角（`▾ n doing` / `▸ n pending` / `▸ n completed`）与 `[ ]` / `[~]` / `[x]` 勾选字形：用户要先学三角位置、再点两下才能看到内容，而且三个翼头即使全空也照画。

宽档虽然已分三栏，但 100 列以下仍需竖叠三区，高度随翼数线性增长，把编辑区顶上去。

产品定位是「开箱即用」：待办应该像 Crush 的 pills 与 Gemini CLI 的 todo 一样，平时只占一行告诉你「在做什么」，想看全表时再点开。

## What Changes

### 1. 默认收起为单行摘要

- 摘要形如 `▸ 待办 2/5 · <当前 doing 正文>`；`2/5` 是已完成/总数，`· <正文>` 只在有 in_progress 时出现。
- 点摘要行任意位置展开，再点收起。
- 展开状态只在本会话内有效，不写持久化。

### 2. 展开态是扁平的分组列表

- 按 doing → pending → completed 分组，组内保持 SSOT 表序。
- 取消三区翼头、取消逐翼折叠、取消上下 `─` 轮廓。

### 3. 状态靠前导字符与亮度三档

| 状态 | 前导 | 取色 |
|---|---|---|
| doing | `→` | `on_surface` + bold |
| pending | `·` | `muted` |
| completed | `✓` | `tool` + dim |

取色只经 `Palette` 语义 token，浅色 / 深色主题均适用；ASCII 字形档回落 `>` / `.` / `x`。

### 4. 溢出表达改到栏内

- 可见行至多 6 行（含摘要行）；超出时末行 `… 还有 N 项 · 滚轮查看`，栏内滚轮滚动。
- 取消 `↑ N more` / `↓ N more` 边框提示。

### 5. 删除逐翼折叠状态

- `todo_doing_open` / `todo_past_open` / `todo_pending_open` 与 `FoldTarget::Todo*` 三个变体、翼头命中区一并删除。

## Further Notes

- 调研依据：Claude Code 任务列表按 `Ctrl+T` 开关且最多显示五条；Gemini CLI 待办在输入框上方实时更新、当前任务高亮、`Ctrl+T` 看全表；Crush 用 pills 摘要 `To-Do 2/5` 加当前任务名；Codex CLI 的 `update_plan` 列表会空闲消失，社区已在要求常驻或显式显隐。四家共同点：常驻只占一行摘要，全表靠显式动作展开。
- 本 change 不改 todo 工具入参、不改 SSOT 形态、不改 AgentStatusBar 投影。
