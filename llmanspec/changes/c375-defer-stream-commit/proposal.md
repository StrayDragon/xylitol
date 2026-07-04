---
change_id: c375-defer-stream-commit
title: 流式 markdown 延迟到 TurnEnd 整段 commit（代码块高亮生效）
status: proposed
priority: 375
depends_on: []
author: agent
---

# c375-defer-stream-commit

## Why

c370 vendor 了 ratatui-markdown + syntect 代码高亮，渲染逻辑正确（TestBackend 验证：`fn` 紫色、字符串绿色、CatppuccinMocha 配色）。但用户终端实测代码块**单色无高亮**。

**根因**（源码级确认）：流式按行 commit，markdown 高亮需要完整代码块上下文。
- `app.rs:208` 的 `drain_complete_lines()` 每遇 `\n` 把**单行**包成 `RenderedLine::AssistantText(line)` commit
- 每行单独 `render_markdown("fn main() {")`——无 ` ```rs ` 围栏，vendored parser 当普通段落，高亮不触发

**ratatui inline 约束**（ratatui-core 源码确认）：`insert_before` 写入终端物理 scrollback 后**不可回溯修改**。codex 式「流式纯文本→结束重渲替换」需自实现 CustomTerminal + scroll region + cell 数据层 splice，架构改动过大。

本变更采用更简单的方案：**延迟到 TurnEnd 整段 commit**。流式期间整段在 mutable 区累积显示（保留逐字效果），TurnEnd 时整段一次性 `render_markdown` + commit（围栏上下文完整，高亮触发）。

## What Changes

### 1. TuiApp 数据流改造（app.rs）

- 激活 `finalized` 字段（当前 dead）：TextDelta 累积整段，**不再逐行 commit**
- TextDelta 分支：只累积，返回空 Vec（不 commit）
- TurnEnd 分支：整段作为单个 `AssistantText(full_text)` 返回（触发高亮 commit），然后 clear
- ThinkingDelta 同理：累积到 `thinking_finalized`，TurnEnd 整段 commit 为 ThinkingText
- pending_tail 改返回整段（mutable 区显示完整流式文本）

### 2. TAIL_HEIGHT 动态化（terminal.rs）

TAIL_HEIGHT = `(终端高度 / 2).max(6)`。mutable 区显示半屏流式内容，平衡 scrollback 可见性。

## Capabilities

- `app-tui`（修改）：新增 tui75（assistant 文本延迟到 TurnEnd 整段 commit）；修订 tui50（complete lines commit incrementally 语义变化）。

## Impact

- **受影响代码**：`src/app/tui/app.rs`（handle_xy_event + pending_tail）、`src/app/tui/terminal.rs`（TAIL_HEIGHT）
- **风险**：中。流式渲染架构变更，影响每条 TextDelta 的处理路径。mutable 区长回复显示不全（drop 顶部，靠 TAIL_HEIGHT=终端/2 缓解）。

## 不在本变更范围

- codex 式「流式期间也高亮」（source-backed cell + 全量重渲 + reflow，过大）
- 表格 holdback
- OSC8 可点击链接

## 调研证据

- ratatui-core `terminal/inline.rs:130-215`：insert_before 写临时 Buffer → backend.draw → append_lines 物理滚动，无 in-memory 副本可回写。
- codex `streaming/controller.rs`：流式全程高亮 + 两区域（stable/tail），靠 CustomTerminal + insert_history（scroll region 手动操作）实现，依赖 cell 数据层 splice 重排——xylitol 原生 ratatui inline 无此能力。
- xylitol `mutable_line.rs:45-52` 已支持多行 wrap + 超出 drop 顶部；`tail.rs:57` 已把 mutable 区设为「总高 - panel - status」全部剩余——延迟 commit 的显示基础已就绪。
