---
change_id: c341-drop-umbrella-ratatui-use-core
title: 弃用 umbrella ratatui，直依 ratatui-core + ratatui-crossterm，放弃所有内置 widget
status: proposed
priority: 341
depends_on:
  - c340-add-tui-inline-repl
author: agent
---

# c341-drop-umbrella-ratatui-use-core

## Why

c340 落地 TUI 时依赖 umbrella `ratatui` 0.30.2。经对本地 `../ratatui` 仓库的源码核查 + 对 codex/pi/kimi-code 三个参考项目的 TUI 架构调研，发现 umbrella crate 对 xylitol 的 **inline 模式**是错配，且携带了我们用不上的 widget 包。本变更做两件事：(1) 依赖从 umbrella 切到 `ratatui-core` + `ratatui-crossterm` 直依；(2) 放弃所有 ratatui 内置 widget（当前唯一在用的是 `Paragraph`），改为自建组件渲染。

### 为什么 umbrella + 内置 widget 对 inline 模式是错配

**证据 1 —— ratatui crate-split 架构（本地 `../ratatui` 源码核查）**
- `ratatui-core`（`ratatui/ratatui-core/`，v0.1.2，`#![no_std]`）含我们用到的**全部**核心类型：`terminal/`（`Terminal`/`Frame`/`Viewport::Inline`/`inline.rs:109` 的 `insert_before`/`frame.rs:166` 的 `set_cursor_position`）、`backend.rs`（`Backend` trait + `TestBackend`）、`widgets.rs`（**只有 `Widget`/`StatefulWidget` trait，零内置实现**）、`buffer/`、`layout/`（`Rect`/`Layout`/`Constraint`）、`style/`、`text/`（`Text`/`Line`/`Span`）。
- `ratatui-crossterm`（`ratatui/ratatui-crossterm/`，v0.1.2）只提供 `CrosstermBackend<W: Write>`（`src/lib.rs:160`），默认转发 `crossterm_0_29`。
- umbrella `ratatui` 的 `src/init.rs`（572 行，但每个函数逻辑仅 ~5-10 行，其余是文档注释）只额外提供 `try_init_with_options`/`restore`/`DefaultTerminal` + re-export `ratatui-widgets`（内置组件包）。**我们用到的末三个便利函数 ~15 行可自写**（逻辑 = `enable_raw_mode` + `CrosstermBackend::new(stdout())` + `Terminal::with_options`；restore = `disable_raw_mode` + `LeaveAlternateScreen`，inline 模式跳过 alt screen）。
- `ratatui-core/Cargo.toml` 有 `scrolling-regions` feature（注释原文：*"Use terminal scrolling regions to make some operations less prone to flickering. (i.e. Terminal::insert_before)."*），由 `ratatui-crossterm` 转发——**直击 c340 §7 遗留的 insert_before 闪烁**。

**证据 2 —— `Line` 本身就是 `Widget`，根本不需要 `Paragraph`**
- `ratatui-core/src/text/line.rs:699` — `impl Widget for Line<'_>`；`line.rs:705` — `impl Widget for &Line<'_>`，调用 `self.render_with_alignment(area, buf)`。
- xylitol 当前 `render.rs:17,67-68` 是**唯一**使用内置 widget 的地方：`Paragraph::new(lines)` + `frame.render_widget(para, content_area)`。这完全可由「逐行 `line.render(row_area, buf)`」或直接写 `Buffer` 取代，且 c340 的 `commit_to_scrollback`（`terminal.rs:51-92`）**已经在直接操作 `Buffer`**（`buf[(x,y)].set_char`/`set_style`/`set_symbol`），证明这条路可行且已是项目既定模式。

**证据 3 —— 内置 widget 不擅长「流式 + 部分行更新 + scrollback」场景**
- umbrella 携带的 `ratatui-widgets`（Paragraph/List/Block/Table/…）是为 **alt-screen 全屏重绘**设计的组件模型。xylitol 是 inline 模式：mutable tail + scrollback 两区，tail 每帧重画、scrollback 一次性 commit 后不再碰。`Paragraph` 的 wrap/scroll 语义在这个模型里是噪音。
- **codex（最近的 Rust+ratatui 同类）的做法**：codex **不**用 `Block`/`List`/`Paragraph` 做组合层。它定义自己的 `Renderable` trait（`codex-rs/tui/src/render/renderable.rs:22`，含 `desired_height(width)` + `cursor_pos(area)` + `cursor_style(area)`），ratatui 内置仅作叶子辅助。其输入框 `bottom_pane/textarea.rs:1928` 直接 `buf.set_string`/`buf.set_style` 写 Buffer，**不用任何内置 widget**——这正是 xylitol 要走的方向。
- **pi / kimi-code（成熟的 inline chat TUI）的做法**：两者都是 TS 项目，**完全不用 widget 工具包**，自建 `Component { render(width): string[] }` 契约手写一切（input/editor/loader/messages/dialogs）。kimi-code 的 `components/{messages,chrome,dialogs,editor}/` 按区域分组件的结构是 inline chat TUI 的事实标准布局。

### 战略价值

1. **依赖诚实**：不再无谓携带 `ratatui-widgets`（我们不用一个内置 widget）。信号与「我们自建组件」的架构方向一致。
2. **解锁减闪烁**：`scrolling-regions` feature 在 `ratatui-core` 直连后可直接启用，缓解 c340 §7 遗留的 insert_before 果冻效应（umbrella 路径下也能开，但切 core 时一并解决更干净）。
3. **为 c342/c350 的组件化铺路**：c342 修 c340 遗留、c350 集成 diff_review，都会新增渲染组件。先确立「自建组件 + 直接写 Buffer/用 Line::render」的基线，避免后续组件又被诱导去 import 内置 widget。

## What Changes

### 1. Cargo.toml 依赖替换
```toml
# 删:
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"], optional = true }
# 加:
ratatui-core = { version = "0.1.2", default-features = false, features = ["std"], optional = true }
ratatui-crossterm = { version = "0.1.2", default-features = false, features = ["crossterm_0_29", "scrolling-regions"], optional = true }
```
`tui` feature 改为 `["dep:ratatui-core", "dep:ratatui-crossterm", "dep:crossterm", "dep:unicode-width"]`。

### 2. 新增 `src/app/tui/init.rs`（复刻 umbrella init.rs 的 inline 子集，~15 行）
```rust
pub type DefaultTerminal = ratatui_core::terminal::Terminal<ratatui_crossterm::CrosstermBackend<std::io::Stdout>>;
pub fn try_init_with_options(opts: ratatui_core::terminal::TerminalOptions) -> std::io::Result<DefaultTerminal> {
    let backend = ratatui_crossterm::CrosstermBackend::new(std::io::stdout());
    DefaultTerminal::with_options(backend, opts)
}
pub fn restore() { /* disable_raw_mode + best-effort; inline 无 alt screen */ }
```
（panic hook 留给 c355，不在本变更。）

### 3. import 路径机械重命名（无逻辑变化）
- `ratatui::Terminal`/`Frame`/`Viewport`/`TerminalOptions`/`layout::Rect`/`style::*`/`text::*`/`backend::TestBackend` → `ratatui_core::` 对应路径。
- `ratatui::DefaultTerminal`/`try_init_with_options`/`restore` → 本地 `crate::app::tui::init::`。
- `ratatui::widgets::Paragraph` → **删除**（见下）。

### 4. 放弃 `Paragraph`（render.rs 唯一内置 widget 用量）
`render.rs:67-68` 的 `Paragraph::new(lines)` + `frame.render_widget(para, content_area)` 改为直接逐行渲染：
```rust
for (i, line) in lines.iter().enumerate() {
    let row_area = ratatui_core::layout::Rect { x: content_area.x, y: content_area.y + i as u16, .. };
    line.render(row_area, frame.buffer_mut());  // Line 实现了 Widget
}
```
（`commit_to_scrollback` 已经在直接写 Buffer，不动。）

### 5. 更新文档
- `src/app/tui/AGENTS.md`：声明依赖 = `ratatui-core` + `ratatui-crossterm`，禁止 import 内置 widget（`Paragraph`/`Block`/`List`/…）；组件一律自建。
- `render.rs` 模块注释更新。

## Capabilities

- `app-tui`（修改）：声明依赖切到 `ratatui-core` + `ratatui-crossterm`，本地管理 init/restore；禁止内置 widget。

## Impact

- **受影响代码**：`Cargo.toml`、`src/app/tui/{terminal,render,app,theme,mod}.rs`（import 重命名）、新增 `src/app/tui/init.rs`、`src/app/tui/diff_review/cli.rs`（也有 ratatui import，一并改）、`src/app/tui/AGENTS.md`。
- **受影响规范**：`app-tui`。
- **风险**：低-中。纯依赖瘦身 + import 重命名 + 一处 Paragraph 替换，**无行为变化**（inline/TestBackend/insert_before 代码路径不变）。主要风险在 `ratatui-crossterm` 的 crossterm 版本兼容（锁定 `crossterm_0_29` 与现状一致）和 init.rs 复刻的正确性。c340 的 8 个 `assert_cursor_position` 渲染测试是回归网。

## 反降级护栏（防止本变更被降级为「只换 Cargo.toml 不删 widget」）

- [ ] umbrella `ratatui` 依赖 MUST 从 Cargo.toml 移除（`cargo tree` 无 `ratatui` 0.30.2，只余 `ratatui-core` + `ratatui-crossterm`）。
- [ ] `scrolling-regions` feature MUST 启用（c340 §7 遗留的减闪烁）。
- [ ] 本地 `init.rs` MUST 实际提供 `try_init_with_options`/`restore`/`DefaultTerminal` 并被 `terminal.rs` 调用（非仅新增文件）。
- [ ] `render.rs` MUST 不再 import `ratatui::widgets::Paragraph`（及任何 `widgets::` 内置）；改用 `Line::render` 或直接写 Buffer。
- [ ] `src/app/tui/` 全目录 grep `ratatui::`（umbrella 路径）MUST 为零；只允许 `ratatui_core::`/`ratatui_crossterm::`/本地 `init::`。
- [ ] c340 的 8 个 `assert_cursor_position` 渲染测试 MUST 全过（行为不变）。
- [ ] `insert_before`/`Viewport::Inline`/`TestBackend` 行为 MUST 与 c340 回归一致。
- [ ] `just qa` 绿。
