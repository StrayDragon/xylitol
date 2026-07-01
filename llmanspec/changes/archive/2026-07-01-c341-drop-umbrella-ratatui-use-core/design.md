# c341 Design — 弃用 umbrella ratatui，直依 ratatui-core + ratatui-crossterm

本变更是纯依赖瘦身 + import 重命名 + 一处 widget 替换，**无行为变化**。设计目标：让依赖与「inline 模式 + 自建组件」的架构方向一致，并启用 scrolling-regions 减闪烁。

## 1. 依赖替换（Cargo.toml）

### 现状（c340 落地后）
```toml
ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"], optional = true }
# tui feature:
tui = ["dep:ratatui", "dep:crossterm", "dep:unicode-width"]
```

### 目标
```toml
ratatui-core = { version = "0.1.2", default-features = false, features = ["std"], optional = true }
ratatui-crossterm = { version = "0.1.2", default-features = false, features = ["crossterm_0_29", "scrolling-regions"], optional = true }
# tui feature:
tui = ["dep:ratatui-core", "dep:ratatui-crossterm", "dep:crossterm", "dep:unicode-width"]
```

**版本锁定事实**：本地 `../ratatui` 仓库 `ratatui-core/Cargo.toml:2-3` = `0.1.2`，`ratatui-crossterm/Cargo.toml:2-3` = `0.1.2`，umbrella `ratatui 0.30.2` 正是 re-export 这两个版本。crossterm 锁 `crossterm_0_29`（与现状 `crossterm = "0.29.0"` 一致）。

**feature 选择**：
- `ratatui-core`：`std`（我们非 no_std）；**不**加 `underline-color`（不用下划线色，减依赖）。
- `ratatui-crossterm`：`crossterm_0_29`（默认）+ `scrolling-regions`（转发给 core，减 insert_before 闪烁）。
- **不**加 `ratatui-widgets`（umbrella 才有；我们弃用所有内置 widget）。

## 2. 新增 `src/app/tui/init.rs`

复刻 umbrella `ratatui/src/init.rs` 中 inline 模式需要的部分（umbrella init.rs 共 572 行，但每个函数逻辑 ~5-10 行，其余是文档；我们只要 3 个符号）：

```rust
//! 本地终端初始化（替代 umbrella ratatui::init）。
//! 只覆盖 inline 模式所需；alt-screen 路径不在此（spec tui1 禁止 alt screen）。

use std::io::{self, Stdout};

use ratatui_core::terminal::{Terminal, TerminalOptions};
use ratatui_crossterm::CrosstermBackend;

/// inline 模式终端（crossterm + stdout）。
pub type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

/// 进入 inline viewport（调用方负责 enable_raw_mode）。
pub fn try_init_with_options(opts: TerminalOptions) -> io::Result<DefaultTerminal> {
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::with_options(backend, opts)
}

/// 恢复终端（inline 模式：只关 raw mode，无 alt screen 退出）。
/// panic hook 在 c355；本函数是 Drop 路径的兜底。
pub fn restore() {
    let _ = crossterm::terminal::disable_raw_mode();
}
```

**注意**：umbrella `restore` 还会 `LeaveAlternateScreen`，但 inline 模式从不 `EnterAlternateScreen`，故省略（spec tui1）。panic hook（`set_panic_hook`）留给 c355，不在本变更。

## 3. import 重命名映射表（机械替换，无逻辑变化）

| 现状（umbrella） | 目标 |
|---|---|
| `ratatui::Terminal` | `ratatui_core::terminal::Terminal` |
| `ratatui::Frame` | `ratatui_core::terminal::Frame` |
| `ratatui::Viewport` | `ratatui_core::terminal::Viewport` |
| `ratatui::TerminalOptions` | `ratatui_core::terminal::TerminalOptions` |
| `ratatui::layout::Rect` | `ratatui_core::layout::Rect` |
| `ratatui::style::{Style,Color,Modifier}` | `ratatui_core::style::{Style,Color,Modifier}` |
| `ratatui::text::{Line,Span,Text}` | `ratatui_core::text::{Line,Span,Text}` |
| `ratatui::backend::TestBackend` | `ratatui_core::backend::TestBackend` |
| `ratatui::DefaultTerminal` | `crate::app::tui::init::DefaultTerminal` |
| `ratatui::try_init_with_options` | `crate::app::tui::init::try_init_with_options` |
| `ratatui::restore` | `crate::app::tui::init::restore` |
| `ratatui::widgets::Paragraph` | **删除**（见 §4） |

**受影响文件**（grep `ratatui::` 确认）：`terminal.rs`、`render.rs`、`app.rs`、`theme.rs`、`mod.rs`、`diff_review/cli.rs`。

## 4. 放弃 Paragraph（render.rs 唯一内置 widget 用量）

### 现状（render.rs:61-68）
```rust
let content_area = ratatui::layout::Rect { x, y, width, height };
let para = Paragraph::new(lines);
frame.render_widget(para, content_area);
```

### 目标
`Line` 实现了 `Widget`（`ratatui-core/src/text/line.rs:699`），逐行渲染：
```rust
let content_area = ratatui_core::layout::Rect { x, y, width, height };
let buf = frame.buffer_mut();
for (i, line) in lines.iter().enumerate() {
    let row_y = content_area.y + i as u16;
    if row_y >= content_area.bottom() { break; }
    let row_area = ratatui_core::layout::Rect { x: content_area.x, y: row_y, width: content_area.width, height: 1 };
    line.render(row_area, buf);
}
```

**为什么不用 `frame.render_widget(line, area)`**：`Frame::render_widget` 签名要 `Widget`（by value，`self`），`&Line` 也实现了 `Widget`（`line.rs:705`），但逐行直接 `line.render(row_area, buf)` 更显式、与 `commit_to_scrollback` 的直接 Buffer 操作风格一致。

**commit_to_scrollback 不动**：它已经在直接写 Buffer（`terminal.rs:56-89`，处理 CJK filler），是正确的自建组件模式，保留。

## 5. 风险与缓解

### R1：ratatui-crossterm 与 crossterm 版本不兼容【低】
`ratatui-crossterm` 默认转发 `crossterm_0_29`，与现状 `crossterm = "0.29.0"` 一致。
**缓解**：`cargo tree -d` 确认无 crossterm 双版本。

### R2：init.rs 复刻行为与 umbrella 不一致【低】
umbrella `try_init_with_options` 内部还有 cursor query 等。
**缓解**：c340 的 8 个 `assert_cursor_position` 测试是回归网；真终端验收跑一次。inline 模式核心 = `enable_raw_mode` + `Terminal::with_options(Inline)`，复刻这几步即可。

### R3：scrolling-regions 在某些终端无效【低】
非所有终端支持 scrolling region（DECSTBM）。
**缓解**：ratatui 有 fallback（clear-and-redraw），开启只是「在支持的终端更好」，无负面。c340 §7 已记录这是已知优化。

### R4：diff_review/cli.rs 也有 ratatui import【低】
`diff_review/cli.rs:29` import umbrella ratatui。diff_review 是孤立 demo（c350 处理），但本变更要让它继续编译。
**缓解**：一并做 import 重命名（机械），不改 diff_review 逻辑。c350 删除 diff_review 时这些 import 自然消失。

## 6. 不做的事（防 scope creep）

- ❌ panic hook — c355 职责。
- ❌ 重写组件结构（messages/chrome/dialogs 分目录）— c342/c350 按需；本变更只换地基不重组件。
- ❌ 改 diff_review 逻辑 — 只改 import 让它编译，c350 删除它。
- ❌ 改事件循环 / 修 c340 遗留 — c342 职责。
