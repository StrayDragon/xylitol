---
change_id: c355-switch-to-ratatui-core-backend
title: 从 umbrella ratatui 切换到 ratatui-core + ratatui-crossterm 直依
status: proposed
priority: 355
depends_on:
  - c340-add-tui-inline-repl
author: agent
---

# c355-switch-to-ratatui-core-backend

## Why

c340 落地 TUI 时依赖 umbrella `ratatui` crate（0.30.2）。经调研 ratatui crate-split 架构后发现：

- 我们用到的全部核心类型——`Terminal`、`Viewport::Inline`、`insert_before`、`Frame::set_cursor_position`、`TestBackend`、`Widget`/`StatefulWidget` trait、`Buffer`、`Layout`、`Style`、`Text`/`Line`/`Span`——**都在 `ratatui-core`**（源码事实：umbrella `ratatui/src/lib.rs:479` 只是 `pub use ratatui_core::terminal::{...}` re-export）。
- umbrella `ratatui` crate 只额外提供：re-export shim + 几个便利函数（`try_init_with_options`/`restore`/`DefaultTerminal`，这些 ~15 行我们自己写）+ `ratatui-widgets`（内置组件）。
- 多数内置 widget（流式 transcript、带 cursor 的输入框、spinner、slash 弹窗）我们**用不上、得自写**——因为内置 widget 不建模「流式 + 部分行更新 + scrollback」的场景。印证了「自建组件」的架构方向。

继续依赖 umbrella 会无谓携带 `ratatui-widgets` + shim 到依赖树（无功能收益），且 umbrelllla 的便利函数硬编码 `CrosstermBackend<Stdout>`（与 inline viewport 不完全契合）。切换到 `ratatui-core` + `ratatui-crossterm` 直依，信号更诚实（我们自建组件）、依赖更精简。

### 调研证据
- `ratatui-core` 是 `#![no_std]`，含 `terminal/`（Terminal/Frame/Viewport/inline.rs 的 insert_before/frame.rs 的 set_cursor_position）、`backend.rs`（Backend trait + TestBackend）、`widgets.rs`（Widget/StatefulWidget trait）、`buffer/`、`layout/`、`style/`、`text/`、`symbols/`。
- umbrella `ratatui/src/lib.rs:479-521` 证实核心类型全是 `ratatui_core::` 的 re-export；umbrella 独有的只是 `init.rs`（DefaultTerminal/try_init_with_options/restore）+ widget re-exports。
- backend 选择：`ratatui-crossterm`（已用、跨平台、`event::poll/read` 不需 EventStream）优于 `ratatui-termion`（DSR race 本质相同，换它无收益还丢跨平台）。
- c340 实际使用的 ratatui 符号：text/style/Frame/Rect/Terminal/TestBackend/TerminalOptions/Viewport/DefaultTerminal/try_init_with_options/restore——仅末三个是 umbrella-only，且 trivial。

## What Changes

1. **Cargo.toml**：`ratatui` 替换为 `ratatui-core` + `ratatui-crossterm`（+ 启用 `scrolling-regions` 缓解 c340 §7 遗留的 insert_before 闪烁）。
2. **新建本地 init 模块**（如 `src/app/tui/init.rs`）：提供 `DefaultTerminal` 类型别名 + `try_init_with_options` + `restore`，复刻 umbrella 的 `init.rs`（~15 行，去掉 inline 不需要的 alt-screen 部分）。
3. **import 路径机械重命名**（无逻辑变化）：`ratatui::Terminal`→`ratatui_core::terminal::Terminal` 等；`ratatui::DefaultTerminal`/`try_init_with_options`/`restore`→本地 init 模块。
4. **Paragraph 去留**：`Line` 已实现 `Widget` trait（core 就有），`render.rs` 改用 `line.render(area, buf)` 直接渲染，去掉对 `ratatui-widgets::Paragraph` 的依赖。
5. **panic hook**（c340 defer 项）：在 init 模块里加 `set_panic_hook` 确保异常退出也恢复终端。

## Capabilities

- `app-tui`（修改）：声明 TUI 依赖 `ratatui-core` + `ratatui-crossterm`，本地管理 init/restore。

## Impact

- **受影响代码**：`Cargo.toml`、`src/app/tui/{terminal,render,mod}.rs`（import 重命名）、新增 `src/app/tui/init.rs`。
- **风险**：低（纯依赖瘦身 + import 重命名，无行为变化；insert_before/inline/TestBackend 代码路径不变）。

## 反降级护栏（防止本变更被降级为「只换 Cargo.toml 不接 init」）

- [ ] umbrella `ratatui` 依赖 MUST 从 Cargo.toml 移除（非仅新增 core）。
- [ ] 本地 init 模块 MUST 实际提供 `try_init_with_options`/`restore` 并被 `terminal.rs` 调用。
- [ ] `insert_before`/`Viewport::Inline`/`TestBackend::assert_cursor_position` 行为 MUST 与 c340 回归一致（c340 的 8 个渲染测试全过）。
- [ ] `scrolling-regions` feature MUST 启用（缓解 insert_before 闪烁，c340 §7 遗留）。
- [ ] 本变更 MUST NOT 改变 TUI 用户可见行为（纯重构）。
