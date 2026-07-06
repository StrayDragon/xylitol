---
change_id: c396-tui-markdown-style-fix
title: TUI markdown 正文组件样式修复（标题分级/引用前缀/有序列表/主题自适应）
status: proposed
priority: 396
depends_on: []
author: agent
---

# c396-tui-markdown-style-fix

## Why

c395 自研 renderer 落地后，用户反馈渲染效果仍明显不如 codex「好看舒畅」（见 `_HANDOFF.md` 症状对照表）。差距分两类：

1. **样式表残缺（本次修复）**：标题层级塌平（6 级共用一个粗体、无 `#` 前缀）、引用块无任何标识（纯灰斜体，测试还显式断言「no pipe prefix」）、有序列表数字 marker 完全丢失（`Tag::List(Some(_))` 落入 `_ => {}`）、高亮主题写死 CatppuccinMocha（亮色终端刺眼）。
2. **架构耦合（c397 承载）**：渲染粒度 = commit 粒度 = 段落，跨段落上下文丢失导致留白不一致。本次**不动**架构。

本变更只做第一梯队——**纯样式修复**，单文件 `markdown_render.rs` 为主（加少量 `syntect_highlight.rs` + `app.rs` 接线）。覆盖 HANDOFF 症状表的 6/7 项，立即改善观感，风险低。

依据 `_HANDOFF.md` 第五节建议「不要边改样式边改架构」，架构解耦另立 c397。

## What Changes

### 1. 标题分级渲染（markdown_render.rs）

`RenderStyle` 新增 `h1..h6` 六个样式字段，`current_style()` 按 `HeadingLevel` 返回不同样式，对齐 codex `MarkdownStyles`：
- H1：粗体 + 下划线
- H2：粗体
- H3：粗体 + 斜体
- H4–H6：斜体

`start_block_or_inline(Tag::Heading)` 推入标题前缀 span：`"# " / "## " ... `（codex 风格 `format!("{} ", "#".repeat(level))`），前缀沿用对应级别样式。删除 `Block::Heading(HeadingLevel)` 上 `#[allow(dead_code)]` 注释（level 现在被用到）。

### 2. 引用块加 `> ` 前缀（markdown_render.rs）

修复 spec 与实现不一致的债务：spec **tui66 要求**「blockquote lines MUST render with a visible leading prefix glyph」，但当前实现是纯灰斜体无前缀，且测试 `blockquote_italic_no_pipe` 显式断言「no pipe prefix」——**实现违反了 spec**。

改为：`start_block_or_inline(Tag::BlockQuote)` 时推入 `> ` 前缀 span（codex 风格，绿色或保留 quote 样式）；嵌套引用（`>>`）累加前缀。删除/改写 `blockquote_italic_no_pipe` 测试，改为断言「每行有 `> ` 前缀」。

> 注：spec tui66 原文写的是 `▎` 左竖线（codex 用 `│`、pi 用 `>`、kimi 用 `▏`）。本变更统一采用 codex/pi 共识的 `> ` 文本前缀（最接近 markdown 源语义、跨终端最稳），spec 文案在 archive 时同步更新为 `> `。

### 3. 有序列表数字 marker（markdown_render.rs）

`start_block_or_inline` 处理 `Tag::List(Some(start))`：维护 `list_counter: Option<u64>`，`Tag::Item` 时按 counter 递增输出 `{n}. `（light_blue，对齐 codex `ordered_list_marker`）；`Tag::List(None)` 仍输出 `• `。`Tag::List(Some(start))` 当前落入 `_ => {}` 是 bug，本次修复。

### 4. 代码块留白改为有条件（markdown_render.rs）

c395 follow-up（commit d877b63）为绕开「段落独立渲染看不到前文」加了**无条件**前后空行。本次保留该行为（架构未解耦前仍是必需），但在 `finish()` 兜底处避免连续空行堆积。代码注释更新为「streaming 段落切分的临时缓解，根治见 c397」。

### 5. 高亮主题自适应（syntect_highlight.rs + 接线）

`theme()` 由写死 `CatppuccinMocha` 改为按终端背景明暗二选一：
- **亮色终端**：`CatppuccinLatte`
- **暗色终端**：`CatppuccinMocha`（现状）

明暗探测策略（按可靠性排序，首个命中即采用）：
1. 查询终端背景色：发 `OSC 11` 查询 + 读响应解析 RGB → 计算相对亮度（ITU-R BT.601 加权）判定明暗。**仅当能同步拿到响应时采用**（crossterm 不直接支持 OSC 响应读取，需谨慎；若实现复杂度过高，降级到策略 2）。
2. `COLORFGBS` 环境变量（`"0;15"` 形式，前景;背景，背景 < 7 视为暗）——常见于 macOS Terminal/iTerm/Alacritty/Tmux 传播。
3. `COLORTERM` / `TERM` 启发式（`*-256color` 多为现代暗色终端，但不可靠，仅兜底）。
4. 兜底默认：暗色（CatppuccinMocha，保持现状）。

> **降级承诺**：OSC 11 同步读取在 ratatui inline viewport 下有竞态风险（可能吞掉用户输入字节）。若实测不稳定，本变更退化为「仅 `COLORFGBS` + 兜底暗色」，OSC 探测留 future.md。design.md 记录权衡。

主题选择结果缓存在 `OnceLock`，进程内只探测一次。

## Capabilities

- `app-tui`：markdown renderer 样式表 + 高亮主题（modify tui61/tui66/tui71，add tui78/tui79）。

## Impact

- **代码**：`src/app/tui/components/markdown_render.rs`（主改）、`syntect_highlight.rs`（主题）、`app.rs`（若有接线）。`markdown.rs` seam 签名不变。
- **测试**：`markdown_render.rs` 内联测试 + `markdown.rs` 行为测试需更新（blockquote 断言反转、新增标题分级/有序列表断言）。
- **spec**：modify tui61（标题分级细化）、tui66（`> ` 前缀文案）、tui71（主题自适应）；add tui78（标题分级）、tui79（有序列表 marker）。
- **风险**：低。单文件为主，不动流式管线/viewport。OSC 探测有降级路径。
- **不在范围**：渲染粒度解耦（c397）、stable 区可替换、CustomTerminal fork。
