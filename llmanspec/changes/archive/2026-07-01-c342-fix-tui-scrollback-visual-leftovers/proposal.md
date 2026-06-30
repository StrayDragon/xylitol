---
change_id: c342-fix-tui-scrollback-visual-leftovers
title: 修复 c340 TUI 的 4 项已知遗留（用户消息不上行 / TurnEnd 留尾区 / 输入框未置底视觉）
status: proposed
priority: 342
depends_on:
  - c340-add-tui-inline-repl
author: agent
---

# c342-fix-tui-scrollback-visual-leftovers

## Why

c340 落地 TUI 后，真终端验收阶段在 `design.md §7 已知遗留` 记录了 4 项视觉/行为问题。它们不阻塞核心 REPL 流程，但影响基础可用性——尤其「用户消息不上行」和「TurnEnd 回复留尾区」让对话历史不完整、视觉混乱。本变更逐个修复，让 c340 的 inline REPL 达到「可用」基线。

### 4 项遗留及归属（关键：其中 1 项已转交 c341）

| # | c340 §7 遗留 | 归属 | 说明 |
|---|---|---|---|
| 1 | 输入框「未置底」视觉错觉 | **c342** | `Viewport::Inline(3)` 上方留白给人没到底的错觉 |
| 2 | insert_before 闪烁（果冻效应） | **c341**（已转交） | c341 启用 `scrolling-regions` feature 缓解；c342 不重复处理 |
| 3 | 用户消息不上行 | **c342** | Enter 提交后用户输入文字未提交到 scrollback |
| 4 | TurnEnd 后回复留尾区 | **c342** | 回复末行留在 tail，下次提交才上行 |

> **与 c341 的关系**：c341（依赖瘦身）和 c342（遗留修复）都依赖 c340，彼此**独立**可并行。c341 解决闪烁（遗留 #2），c342 解决其余 3 项（#1/#3/#4）。若 c341 先落地，c342 受益于减闪烁但不依赖它；若 c342 先落地，闪烁问题仍在但其他 3 项已修。

### 调研证据（代码事实）

**遗留 #3（用户消息不上行）——代码事实**
- `src/app/tui/mod.rs:154-177`（c340 `InputOutcome::Submit` 分支）：`take_input()` 后只 `driver.run(&prompt)` + `start_stream()`，**从未把用户输入的 prompt 文本 commit 到 scrollback**。
- 对标：pi / kimi-code 的 user-message 组件（`kimi-code/apps/kimi-code/src/tui/components/messages/user-message.ts`）在提交时立即把用户输入渲染为一条用户消息上行到对话流。codex 的 `history_cell` 同理。所有成熟 chat TUI 都让用户看到自己发了什么。
- c340 design §7 原文：「Enter 提交后用户输入文字未提交到 scrollback。曾尝试加 commit 但回退。」——说明 c340 试过但遇到问题（推测是 commit 时机/样式），本变更需查清回退原因再正确实现。

**遗留 #4（TurnEnd 后回复留尾区）——代码事实**
- `src/app/tui/mod.rs:209-221`（`Msg::Xy` 分支 TurnEnd 处理）：`handle_xy_event` 对 `TurnEnd` 返回剩余 pending 行（`app.rs:124-133`），这些行被 `commit_to_scrollback`，但 **tail 区的 `current_streaming_line`（`app.pending`）此时已清空，而 tail 仍被 `draw_tail` 重画**——如果上一帧 tail 里还有流式文本残留，TurnEnd 这一帧把它清了但视觉上可能有「最后一行闪在 tail 然后消失再从上方出现」的撕裂。
- 根因推测：`pending` 在 `handle_xy_event(TurnEnd)` 里清空（`app.rs:127-130` drain 到 out），但 tail 的 `current_streaming_line()` 读的就是 `pending`——清空后 tail 立刻空了，而 commit 是同一帧。时序上是「tail 清空 + commit 上行」应同帧完成，但若 draw 顺序不对会撕裂。

**遗留 #1（输入框未置底视觉错觉）——代码事实**
- `src/app/tui/terminal.rs:21`：`const TAIL_HEIGHT: u16 = 3`。`Viewport::Inline(3)` 占终端底部 3 行，但 `render.rs:60-66` 的 `content_area` 是**底部对齐**（`y = area.y + area.height - n`）。当只渲染 1-2 行时，tail 区顶部有空白行，给人「没到底」的错觉。
- 对标：pi/kimi-code 的 footer 用背景填充或分隔线让输入区视觉边界清晰。

## What Changes

### 修复 #3：用户消息上行
在 `mod.rs` 的 `Submit` 分支，提交 prompt 前（或后）把用户输入 commit 到 scrollback，样式区分（如 `❯ ` 前缀 + 用户色，或 dim 样式）：
```rust
InputOutcome::Submit(prompt) => {
    // 用户消息上行（修复遗留 #3）
    let user_line = Line::styled(format!("❯ {prompt}"), theme::palette().user_prompt());
    term.commit_to_scrollback(&[user_line])?;
    // ... 原有 driver.run / start_stream ...
}
```
需在 `theme.rs` 加 `user_prompt()` token（或复用既有）。

### 修复 #4：TurnEnd 同帧清尾 + 上行
确保 TurnEnd 时 `pending` 的剩余文本与 tail 清空在同一次 draw 完成。核心：`handle_xy_event(TurnEnd)` 返回的行立即 commit，且该帧 `draw_tail` 看到的 `current_streaming_line` 已为空（`pending` 已 drain）——验证当前时序是否已满足，若满足则问题在别处（如未额外 draw 一次）；若不满足则调整 `end_stream` 与 commit 的顺序。

### 修复 #1：输入框视觉置底
两个选项（实施时评估）：
- **A（背景填充）**：tail 区未用行用 input_bg 填充，让输入区视觉上是连续色块。
- **B（分隔线）**：tail 区顶部画一条 dim 分隔线。
对标 kimi-code footer（背景块）倾向 A。

### 不做（防 scope creep）
- ❌ 闪烁修复（#2）—— c341 的 scrolling-regions。
- ❌ 重写组件结构 —— c342 只修遗留，不重组件目录（c350 按需）。
- ❌ 多行编辑器 —— c340 已 defer，不在 c342。

## Capabilities

- `app-tui`（修改）：用户消息上行、TurnEnd 同帧一致性、输入框视觉置底。

## Impact

- **受影响代码**：`src/app/tui/mod.rs`（Submit/TurnEnd 分支）、`src/app/tui/render.rs`（tail 填充）、`src/app/tui/theme.rs`（user_prompt token）、可能 `src/app/tui/app.rs`（end_stream 时序）。
- **受影响规范**：`app-tui`。
- **风险**：低。都是在既有结构上的小修补，有 c340 的渲染测试做回归网，再加新测试覆盖修复行为。

## 反降级护栏（防止本变更被降级为「只改一处不验收」）

- [ ] 用户提交 prompt 后，用户输入文本 MUST 立即出现在 scrollback（修复 #3）——有测试或手动验收记录。
- [ ] TurnEnd 时回复末行 MUST 同帧上行到 scrollback，tail 不残留上一轮文本（修复 #4）——有测试或手动验收记录。
- [ ] 输入框视觉 MUST 无「悬空」错觉（修复 #1）——手动验收记录。
- [ ] c340 既有 8 个 cursor_tests + input/app 测试 MUST 全过（无回归）。
- [ ] 本变更 MUST NOT 触碰闪烁修复（#2 归 c341）。
- [ ] `just qa` 绿。
