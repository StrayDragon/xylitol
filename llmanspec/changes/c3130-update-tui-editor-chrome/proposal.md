---
depends_on:
  - c3115-add-tui-stage-name-display
  - c3120-update-tui-activity-display
title: TUI 编辑区 chrome 改版与详情下线
needs_specs_change: true
---

# TUI 编辑区 chrome 改版与详情下线

## Why

当前编辑区上边框常驻 `[详情] [侧栏] [重置折叠]` 三个 chip，纯 `─` 边框；「详情」overlay 是会话 id 的唯一屏上入口，但用户已决定下掉它。用户已确认视觉：**B+E+D** —— 圆角 `╭╮╰╯`、阶段名右对齐在上边框、`◨` 与输入同一行右端、折叠 chip 在下边框右端、spinner 固定 `⬢` 颜色脉冲。该改版同时修掉 `[重置折叠]` 常驻与 spec r3033 的漂移。

## 术语

- **阶段名**：c3110/c3115 定义的会话短显示名；本 change 负责它在 chrome 中的最终落位。
- **chrome**：编辑区上边框 / 下边框 / chip / 状态 spinner / footer 的统称；不是新固定区行。
- **折叠 chip**：`[▤ 摘要]` 紧凑入口，循环 c3120 的显示等级；窄档隐藏。
- **颜色脉冲**：spinner 字形不变，accent 在亮/暗两档间呼吸；不是字形轮换。

## What Changes

- 下掉「详情」覆盖弹出框及全部逻辑：`[详情]` chip、`app.session.popup`、`detailsCycle` 的 popup 分支、overlay 渲染 / 命中 / pending / 测试 / spec r2903 / DESIGN 例外。会话 id 仍可在会话树 `app.session.toggleId` 查看。
- 编辑区改为圆角边框：上 `╭ ╮`、下 `╰ ╯`；内容宽不变；DESIGN「无圆角」同步改为「编辑区边框圆角」。
- 阶段名在上边框**右端**，右对齐，不居中；muted；超宽 `…`；无阶段名不留空槽。
- 侧栏改为单字符 `◨`（U+25E8，1 列），放在**输入内容行右端**，与 `❯` 同一行；预留 2 列（glyph + 前导空格）；输入文本在预留列前换行/截断；命中区 2 列；标准档 ≥80 显示，窄档隐藏。
- 折叠 chip `[▤ 摘要]` 放在**下边框右端**；点击循环 quiet→summary→recent→full；标准档显示，窄档隐藏；宽度不足整 chip 不画。
- `[重置折叠]` 仅在存在非默认折叠态或逐块覆盖时绘制；放在下边框、折叠 chip 左侧；宽度不足先丢重置 chip；无覆盖时不占位。
- spinner 固定 `⬢`，颜色在 accent 亮/暗两档间脉冲，100ms；不换字形；若终端把 ambiguous 宽字符渲染成 2 列，回退 `●` 固定 + 颜色脉冲。
- 布局预算：圆角 > 上边框标题（截断）> 下边框折叠 chip > 下边框重置 chip（条件）> 内容行 `◨`（2 列）；不得增高编辑区、不得换行、不得另加固定区行。
- 所有可点元素只靠指针可达；chip / `◨` 命中以屏上实际绘制为准。

## Capabilities（拟）

- `app-tui-fixed-zone`：r1224 chip 行 / 阶段名 / 圆角；r2903 删除详情弹出框；r2904 侧栏改 `◨` 内容行；r3031 移除「会话详情弹层显隐」；r3033 重写 dock chip。
- `app-tui-mouse-actions`：r3032 移除 `app.session.popup` / `app.session.detailsCycle`；r3033 新 chip 语义；折叠 chip / `◨` 命中。
- `crates/xylitol-app-tui/src/DESIGN.md`：chip 行、圆角、spinner、侧栏字形、详情例外删除、窄档 alt+u。

## Impact

`layout/root/{footer_chips,render,mod}.rs`、`host/mod.rs`、`keybindings.rs`、`tests.rs`、`DESIGN.md`、specs。视觉变化集中在本端；不改 Host / 协议 / 折叠状态机。

## 非目标

- 不做折叠等级 / 子项可见性（c3120）；本 change 只提供 chip 入口。
- 不做阶段名生成 / 存储（c3110/c3115）；只负责落位。
- 不做侧栏内容 / 宽度档逻辑（保留现状）。
- 不做新的固定区行；所有元素仍在上边框 / 内容行 / 下边框 / 状态行 / footer。
- 不做 Codex 式 TranscriptView。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。视觉已定稿：**B+E+D**（rounded + `◨` 内容行 + pulse spinner），折叠 chip 文案 A（`[▤ 摘要]`）。
