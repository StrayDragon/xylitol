# ratatui 积木库 调研报告

## 0. 元信息

### 子 crate 版本
| crate | 路径 | 版本 |
|-------|------|------|
| `ratatui-core` | `ratatui-core/Cargo.toml:3` | `0.1.2` |
| `ratatui-crossterm` | `ratatui-crossterm/Cargo.toml:3` | `0.1.2` |
| `ratatui-widgets` | `ratatui-widgets/Cargo.toml:5` | `0.3.2` |

### xylitol 当前依赖 (Cargo.toml:63-64)
```toml
ratatui-core = { version = "0.1.2", default-features = false, features = ["std"], optional = true }
ratatui-crossterm = { version = "0.1.2", default-features = false, features = ["crossterm_0_29", "scrolling-regions"], optional = true }
```
xylitol 依赖 `ratatui-core` + `ratatui-crossterm` 直接（非 umbrella `ratatui` crate），且启用了 `scrolling-regions` feature。**不依赖 `ratatui-widgets`**—— 对应 c341 的设计决策："no built-in widgets"。

### 报告日期
2026-07-02

---

## 1. ratatui-core 能力面

`ratatui-core` 是 `#![no_std]` 的基石 crate（[lib.rs:1](https://github.com/ratatui/ratatui/blob/main/ratatui-core/src/lib.rs#L1)），提供以下模块：

### 1.1 layout (`src/layout.rs`)

核心类型和 API 签名：

| 类型 | 关键 API | 说明 |
|------|----------|------|
| `Rect` | `new(x, y, w, h)`, `.x/.y/.width/.height`, `.left()/.right()/.top()/.bottom()`, `.area()`, `.contains(Position)`, `.intersection()`, `.union()`, `.inner(Margin)`, `.rows()/.columns()/.positions()` | 矩形区域，坐标空间原点在左上角 |
| `Position` | `new(x, y)`, `ORIGIN` | 坐标点 |
| `Size` | `new(w, h)` | 尺寸 |
| `Layout` | `vertical([...])`, `horizontal([...])`, `.areas(area)`, `.split(area)`, `.margin(u16)`, `.spacing(Spacing)` | 布局引擎（基于 Cassowary 约束求解器 via `kasuari`） |
| `Constraint` | `Length(u16)`, `Percentage(u16)`, `Ratio(u32, u32)`, `Fill(u16)`, `Min(u16)`, `Max(u16)` | 布局约束 |
| `Direction` | `Vertical`, `Horizontal` | 布局方向 |
| `Flex` | `Start`, `End`, `Center`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`, `Legacy` | 多余空间分配策略 |
| `Alignment` / `HorizontalAlignment` / `VerticalAlignment` | `Left`, `Center`, `Right` / `Top`, `Center`, `Bottom` | 对齐方式 |
| `Margin` | `new(h, v)` | 边距 |
| `Offset` | `new(x, y)` | 偏移 |
| `Spacing` | `Space(u16)`, `Overlap(u16)` | 间距（可选重叠） |
| `Rows` / `Columns` / `Positions` | 迭代器 | 遍历行/列/所有单元格位置 |

xylitol 可用场景：用 `Layout` 切分对话区、输入框、状态栏。

### 1.2 text (`src/text.rs`)

文本三层结构：

```
Text  (多行)
 └── Line  (单行，由多个 Span 组成)
      └── Span  (单一样式的一段文本)
```

| 类型 | 关键 API | 说明 |
|------|----------|------|
| `Text<'a>` | `raw()`, `styled()`, `from()`, `.lines`, `.style`, `.width()`, `.height()` | 多行文本容器 |
| `Line<'a>` | `raw()`, `styled()`, `from()`, `.spans`, `.style`, `.alignment()`, `.width()`, `.pad_left()/.pad_right()`, `.centered()` | 单行文本。实现了 `Widget` |
| `Span<'a>` | `raw()`, `styled()`, `from()`, `.content`, `.style` | 统一样式的一段文本 |
| `StyledGrapheme` | `.symbol`, `.style` | 单个排版字素 |
| `Masked` | `new(str, ch)` | 掩码文本（密码输入） |
| `ToLine` / `ToText` / `ToSpan` | trait | `&str`/`String` 自动转换 |

`Line` 实现了 `Widget`，可以直接渲染：
```rust
frame.render_widget(Line::raw("Hello"), area);
```

xylitol 应用：xylitol 的 `render.rs` 已直接使用 `Line`/`Span` 渲染用户消息和 AI 回复。`Masked` 可为密码/API key 输入提供现成掩码。

### 1.3 style (`src/style.rs`)

| 类型 | 关键 API | 说明 |
|------|----------|------|
| `Style` | `new()`, `.fg(Color)`, `.bg(Color)`, `.add_modifier(Modifier)`, `.remove_modifier()`, `.patch()` | 样式组合 |
| `Color` | `Reset`, `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `Gray`, `DarkGray`, `LightRed`, ..., `Rgb(u8,u8,u8)`, `Indexed(u8)` | 颜色值 |
| `Modifier` | `BOLD`, `DIM`, `ITALIC`, `UNDERLINED`, `SLOW_BLINK`, `RAPID_BLINK`, `REVERSED`, `HIDDEN`, `CROSSED_OUT` (bitflags) | 文字修饰 |
| `Stylize` trait | `.red()`, `.on_blue()`, `.bold()`, `.italic()`, `.underlined()`, ... | 链式风格简写 |
| `Styled` trait | `.set_style()`, `.style()` | 类型通用样式接口 |
| `Palette` | `material::`, `tailwind::` | 配色方案（material design / tailwind） |

xylitol 场景：主题系统已经用 `Style`/`Color`（见 `theme.rs`）。`Stylize` trait 可大幅简化书写。

### 1.4 symbols (`src/symbols.rs`)

| 子模块 | 内容 | 用途 |
|--------|------|------|
| `border` | `Set`, `PLAIN`, `ROUNDED`, `DOUBLE`, `THICK`, `QUADRANT_OUTSIDE`, `QUADRANT_INSIDE` | 边框字符集 |
| `block` | `FULL`, `SEVEN_EIGHTHS`, ..., `ONE_EIGHTH` | 方块字符（进度条用） |
| `bar` | `ONE_EIGHTH`, ..., `FULL` (竖) | 竖条字符 |
| `line` | `HORIZONTAL`, `VERTICAL`, `TOP_LEFT`, `TOP_RIGHT`, `BOTTOM_LEFT`, `BOTTOM_RIGHT`, `CROSS` | 线条字符 |
| `shade` | `FULL`, `DARK`, `MEDIUM`, `LIGHT` | 阴影字符 |
| `braille` | `BRAILLE_DATA` (256 个盲文点) | 点阵图 |
| `half_block` | `UP`, `DOWN`, `FULL`, `THREE_QUARTERS` | 半块字符 |
| `marker` | `DOT`, `BLOCK`, `BAR`, `HALF_BLOCK`, `BRAILLE`, `DOT_DOT` | 标记样式 |
| `pixel` | `PIXEL_DATA` (256 种像素) | 像素级渲染 |
| `scrollbar` | `Set`, `DOUBLE_VERTICAL`, `DOUBLE_HORIZONTAL` | 滚动条符号 |
| `merge` | `MergeStrategy` | 边框合并策略 |

xylitol 场景：`symbols::border::ROUNDED` 可用于对话框/输入框边框，`symbols::scrollbar` 可用于滚动条。

### 1.5 terminal (`src/terminal.rs`)

| 类型/方法 | 签名 | 说明 |
|-----------|------|------|
| `Terminal<B>` | `new(backend)`, `with_options(backend, opts)`, `draw(f)`, `try_draw(f)`, `flush()`, `swap_buffers()`, `clear()`, `resize()`, `autoresize()`, `size()`, `insert_before()`, `backend()`, `backend_mut()` | 渲染引擎，双缓冲 + diff |
| `Frame<'a>` | `.area()`, `.render_widget(w, area)`, `.render_stateful_widget(w, area, state)`, `.set_cursor_position(pos)`, `.buffer` | 单帧渲染上下文 |
| `CompletedFrame<'a>` | `.buffer`, `.area`, `.count` | `draw()` 返回的已完成帧 |
| `TerminalOptions` | `{ viewport: Viewport }` | 终端选项 |
| `Viewport` | `Fullscreen`, `Inline(u16)`, `Fixed(Rect)` | 视口模式 |

`Terminal::draw` 的完整管线（`src/terminal/render.rs`）：
1. `autoresize()` — 检查终端尺寸变化
2. `get_frame()` — 创建 `Frame`
3. 运行闭包填充 buffer
4. `flush()` — diff 当前 buffer 和上一帧，只写差异到 backend
5. 应用光标状态
6. `swap_buffers()` — 交换双缓冲

### 1.6 buffer (`src/buffer/`)

| 类型/方法 | 签名 | 说明 |
|-----------|------|------|
| `Buffer` | `empty(Rect)`, `filled(Rect, Cell)`, `with_lines([...])`, `.area()`, `.content()`, `.cell(pos)`, `.cell_mut(pos)`, `.set_string(x,y,str,style)`, `.set_stringn(...)`, `.set_line(...)`, `.set_span(...)`, `.set_style(area, style)`, `.resize(rect)`, `.reset()`, `.merge(other)`, `.diff(other)`, `.diff_iter(other)`, `Index<(x,y)>`, `IndexMut<(x,y)>` | 格子缓冲（Vec<Cell> 平铺） |
| `Cell` | `new(str)`, `.symbol()`, `.set_symbol(str)`, `.set_char(ch)`, `.set_style(style)`, `.fg`, `.bg`, `.modifier`, `.cell_width()`, `.reset()`, `.skip`, `.diff_option` | 单个单元格 |
| `CellWidth` trait | `.cell_width()` | 字符占用宽度（1=半宽，2=全宽） |
| `BufferDiff` | 迭代器 `(x, y, &Cell)` | 零分配 diff 迭代器 |
| `CellDiffOption` | `None`, `Skip`, `AlwaysUpdate`, `ForcedWidth(NonZeroU16)` | 单元格差异控制 |

关键方法 `Buffer::with_lines` 接受 `Into<Line>` 的迭代器：
```rust
let buf = Buffer::with_lines(["Hello", "World"]);
```

### 1.7 宏：`assert_buffer_eq!`

位于 `src/buffer/assert.rs`（已 deprecated，建议直接用 `assert_eq!`）：
```rust
// 旧用法（deprecated）
assert_buffer_eq!(actual, expected);

// 推荐用法
assert_eq!(actual, expected);
```

`Buffer` 实现了 `PartialEq`，所以直接 `assert_eq!` 即可。详细的 diff 信息通过 `Buffer::diff()` 方法获得。

---

## 2. ratatui-widgets 现成 Widget 清单

以下所有 widget 都在 `ratatui-widgets/src/lib.rs` 中声明，在 umbrella `ratatui` crate 中重新导出。

| Widget | 用途 | 关键 API | xylitol 场景 |
|--------|------|----------|--------------|
| **Block** | 边框 + 标题 + 内边距容器 | `Block::new()`, `.bordered()`, `.title()`, `.title_top()`, `.title_bottom()`, `.border_style()`, `.border_type()`, `.style()`, `.padding()`, `.inner(area)` | 对话框、输入框、状态栏的外框（当前 xylitol 手写边框，可用此替代） |
| **Paragraph** | 显示可选样式的文本块 | `Paragraph::new(text)`, `.block(block)`, `.style()`, `.alignment()`, `.wrap(Wrap)`, `.scroll((v,h))` | 渲染对话历史、AI 回复文本（当前手写渲染，用这个可省大量代码） |
| **List** | 可选择的列表 | `List::new(items)`, `.block()`, `.highlight_style()`, `.highlight_symbol()`, `.direction()`, `+ ListState` | 对话历史列表、文件选择列表 |
| **Table** | 表格（行列网格） | `Table::new(rows)`, `.header()`, `.footer()`, `.widths([...])`, `.column_spacing()`, `.block()`, `+ TableState` | 结构化数据显示（如有需要） |
| **Scrollbar** | 滚动条 | `Scrollbar::default()`, `.orientation()`, `.symbols()`, `.begin_symbol()`, `.end_symbol()`, `.track_symbol()`, `.thumb_symbol()`, `+ ScrollbarState` | 长对话历史滚动 |
| **Tabs** | 标签页选择栏 | `Tabs::new(titles)`, `.select(index)`, `.highlight_style()`, `.divider()`, `.padding()` | 模式切换（对话/设置/资源） |
| **Gauge** | 水平进度条（百分比标注） | `Gauge::default()`, `.percent()`, `.ratio()`, `.label()`, `.gauge_style()`, `.use_unicode()` | 正在渲染/加载进度 |
| **LineGauge** | 细线进度条 | `LineGauge::default()`, `.ratio()`, `.label()`, `.filled_style()`, `.unfilled_style()` | 紧凑进度指示 |
| **Clear** | 清除区域（用于叠画） | `Clear`（单元结构体） | 对话框弹出前清空背景 |
| **Fill** | 单一符号填充整个区域 | `Fill::new(symbol)`, `.style()` | 背景填充、分割线 |
| **BarChart** | 柱状图 | `BarChart::default()`, `.data(BarGroup)`, `.bar_width()`, `.bar_gap()`, `.group_gap()` | 数据可视化 |
| **Chart** | 折线图/散点图 | `Chart::new(datasets)`, `.x_axis()`, `.y_axis()`, `.hidden_legend_constraints()` | 数据可视化 |
| **Canvas** | 任意形状绘制 | `Canvas::default()`, `.paint(\|ctx\| ...)`，支持 `Line`/`Circle`/`Rectangle`/`Points`/`Map`/`World` | 简单图形图示 |
| **Sparkline** | 迷你趋势线 | `Sparkline::default()`, `.data(&[u64])`, `.max()`, `.direction()`, `.style()` | 性能指标小图 |
| **RatatuiLogo** | 显示 ratatui logo | `RatatuiLogo::default()` | 启动 splash |
| **RatatuiMascot** | 显示 ratatui 吉祥物 | `RatatuiMascot::default()` | 趣味展示 |
| **calendar::Monthly** | 单月日历 | `Monthly::new(date, style)` | 日期选择 |

### 对 xylitol 最具价值的 widget（按优先级）

1. **Paragraph** — 直接替代手写的 `Line::render` + 文本换行逻辑（当前 `render.rs` 手动处理 CJK 换行）
2. **Block** — 替代手写边框（输入框、对话框的 `─│┌┐└┘`）
3. **List + ListState** — 对话历史列表（需要选择/高亮/滚动）
4. **Scrollbar** — 长内容滚动指示
5. **Clear** — 对话框叠画（替代当前 `reset` 循环）
6. **Tabs** — 模式切换 UI

---

## 3. TestBackend —— TUI 自动化测试基石（详细）

### 3.1 它是什么、怎么工作

**位置**：`ratatui-core/src/backend/test.rs`

**本质**：一个纯内存实现的 `Backend` trait（`src/terminal/backend.rs`），内部维护一个 `Buffer` 来模拟终端屏幕，不涉及任何真实的终端 I/O。

```rust
pub struct TestBackend {
    buffer: Buffer,         // 主屏 buffer
    scrollback: Buffer,     // 历史滚动 buffer
    cursor: bool,           // 光标可见性
    pos: (u16, u16),        // 光标位置
}
```

**实现了 Backend trait 的所有方法**（`test.rs:178-360`）：
- `draw()` — 将 cell 更新写入内存 buffer
- `hide_cursor()` / `show_cursor()` — 记录光标状态
- `set_cursor_position()` / `get_cursor_position()` — 读写光标位置
- `clear()` — reset 整个 buffer
- `clear_region()` — 按 ClearType 清除特定区域
- `append_lines()` — 模拟终端换行（支持 scrollback）
- `size()` / `window_size()` — 返回 buffer 尺寸 + 伪造像素尺寸
- `flush()` — 无操作
- `scroll_region_up()` / `scroll_region_down()` — 带 `scrolling-regions` feature

### 3.2 最小示例代码

**直接使用 `TestBackend` + `Terminal` 的完整可运行测试**（从源码测试摘取组装）：

```rust
use ratatui_core::backend::TestBackend;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};
use ratatui_core::widgets::{Widget, Paragraph};
use ratatui_core::text::Line;

#[test]
fn test_inline_rendering_with_test_backend() {
    // 1. 创建 TestBackend（指定宽高）
    let backend = TestBackend::new(20, 10);

    // 2. 在内存中模拟一个 inline viewport
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(4),
        },
    ).unwrap();

    // 3. 渲染一帧
    terminal.draw(|frame| {
        let area = frame.area();
        // 直接用 Line widget 渲染文本
        Line::raw("Hello from xylitol!").render(area, &mut frame.buffer);
    }).unwrap();

    // 4. 断言渲染结果
    let expected = Buffer::with_lines([
        "Hello from xylitol!",  // 这一行渲染了什么
        "                    ",  // viewport 剩下空白
        "                    ",
        "                    ",
    ]);
    // 注意：assert_eq! 要求两个 buffer 的 area 一致
    terminal.backend().assert_buffer_lines([
        "Hello from xylitol!",
        "                    ",
        "                    ",
        "                    ",
    ]);
}
```

**渲染 widget 并断言**（更完整的示例）：

```rust
use ratatui_core::backend::TestBackend;
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style, Stylize};
use ratatui_core::terminal::Terminal;
use ratatui_core::widgets::{Widget, Block, Borders, Paragraph};
use ratatui_core::text::Line;

#[test]
fn test_paragraph_with_block() {
    let backend = TestBackend::new(15, 5);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        let area = frame.area();

        // 渲染一个带边框的段落
        let paragraph = Paragraph::new("Hello")
            .block(Block::bordered().title("Test"));
        frame.render_widget(paragraph, area);
    }).unwrap();

    terminal.backend().assert_buffer_lines([
        "┌Test─────────┐",
        "│Hello        │",
        "│             │",
        "│             │",
        "└─────────────┘",
    ]);
}
```

### 3.3 怎么断言渲染结果（buffer assert）

几种断言方式（源码 `test.rs` 提供的便利方法）：

| 方法 | 签名 | 说明 |
|------|------|------|
| `.assert_buffer(&expected)` | `(&self, &Buffer)` | 全 buffer 对比（调用 `assert_buffer_eq!`） |
| `.assert_buffer_lines([...])` | `(&self, impl IntoIterator<Item=Into<Line>>)` | 用 `Buffer::with_lines` 构造预期并对比 |
| `.assert_scrollback(&expected)` | `(&self, &Buffer)` | 对比 scrollback buffer |
| `.assert_scrollback_lines([...])` | `(&self, impl IntoIterator<Item=Into<Line>>)` | scrollback 行对比 |
| `.assert_scrollback_empty()` | `(&self)` | 断言 scrollback 为空 |
| `.assert_cursor_position(pos)` | `(&mut self, impl Into<Position>)` | 断言光标位置 |

直接 `assert_eq!` 也有效（`Buffer` 实现了 `PartialEq`）：

```rust
assert_eq!(terminal.backend().buffer(), &expected_buffer);
```

**diff 机制**（`src/buffer/diff.rs`）：
`Buffer::diff(&self, other) -> Vec<(u16, u16, &Cell)>` 返回两个 buffer 之间所有不同的单元格，逐个位置列出。支持多宽字符（CJK/emoji）的正确 diff。

### 3.4 怎么模拟多帧渲染序列

**测试一：多帧渲染 + inline insert_before**

```rust
#[test]
fn test_multi_frame_render() {
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions { viewport: Viewport::Inline(4) },
    ).unwrap();

    // 第一帧
    terminal.draw(|frame| {
        Line::raw("Frame 1").render(frame.area(), &mut frame.buffer);
    }).unwrap();
    terminal.backend().assert_buffer_lines([
        "Frame 1   ",
        "          ",
        "          ",
        "          ",
    ]);

    // insert_before 一行
    terminal.insert_before(1, |buf| {
        buf.set_string(0, 0, "Inserted", Style::default());
    }).unwrap();

    // 第二帧
    terminal.draw(|frame| {
        Line::raw("Frame 2").render(frame.area(), &mut frame.buffer);
    }).unwrap();
    terminal.backend().assert_buffer_lines([
        "Inserted  ",
        "Frame 2   ",
        "          ",
        "          ",
    ]);
}
```

**测试二：滚动 + scrollback**

```rust
#[test]
fn test_scrollback() {
    let mut backend = TestBackend::with_lines([
        "aaaaaaaaaa",
        "bbbbbbbbbb",
        "cccccccccc",
        "dddddddddd",
        "eeeeeeeeee",
    ]);

    // 模拟在最后一行追加行（触发 scrollback）
    backend.set_cursor_position((0, 4)).unwrap();
    backend.append_lines(1).unwrap();

    backend.assert_buffer_lines([
        "bbbbbbbbbb",
        "cccccccccc",
        "dddddddddd",
        "eeeeeeeeee",
        "          ",
    ]);
    backend.assert_scrollback_lines(["aaaaaaaaaa"]);
}
```

### 3.5 局限性

| 场景 | 能否用 TestBackend 测试 | 说明 |
|------|------------------------|------|
| 纯渲染（layout / widget / style / text） | ✅ 完全支持 | 核心用例 |
| 多帧 diff 渲染管线 | ✅ 支持 | 双缓冲 + diff 行为 |
| 光标位置/可见性 | ✅ 支持 | `cursor_position()` / `cursor_visible()` |
| Inline viewport + insert_before | ✅ 支持 | `Terminal::with_options(Viewport::Inline(h))` |
| Scrollback 内容检查 | ✅ 支持 | `assert_scrollback_*()` |
| scrolling-regions | ✅ 支持 | `scroll_region_up()` / `scroll_region_down()` |
| 终端 resize | ✅ 支持 | `backend.resize(w, h)` |
| ❌ 真实 crossterm 事件 | ❌ 不支持 | 无法模拟键盘/鼠标事件输入 |
| ❌ 真实终端 escape sequence 输出 | ❌ 不支持 | 只测逻辑渲染结果，不测 ANSI 序列正确性 |
| ❌ 真实 raw mode 切换 | ❌ 不支持 | 不涉及 `enable_raw_mode()` |
| ❌ 真实终端尺寸变化信号 | ❌ 不支持 | 需要手动 `resize()` |
| ❌ 带 true color 支持的终端渲染差异 | ⚠️ 部分 | Cell 级别支持 fg/bg color，但终端实际渲染效果不在 scope 内 |

**核心结论**：TestBackend 测的是"应用正确调用了渲染 API + 每一帧 buffer 内容符合预期"。它不测"终端是否显示了正确的 escape sequences"或"crossterm 事件是否被正确分发"——那是集成测试的范畴。

---

## 4. Inline Viewport

### 4.1 ratatui-core 的 inline 模式 API

位于 `src/terminal/viewport.rs`:

```rust
pub enum Viewport {
    Fullscreen,          // 默认，全屏渲染
    Inline(u16),         // 内联：指定高度，锚定在光标行
    Fixed(Rect),         // 固定区域
}
```

**Inline 的工作原理**（`src/terminal/inline.rs`）：
1. 创建时：读取当前光标位置 `pos.y`，通过 `Backend::append_lines()` 预留高度空间，计算 viewport 的 `Rect { x: 0, y: row, width: terminal_width, height: requested }`
2. 每次 draw：保持 viewport 区域，从 `frame.area()` 开始渲染
3. resize 时：重新计算 viewport 位置（保持光标在 viewport 内相对位置）
4. `insert_before(height, draw_fn)`：在 viewport 上方插入内容（用 `scrolling-regions` feature 可避免重绘 viewport）

**关键方法**：
- `Terminal::with_options(TerminalOptions { viewport: Viewport::Inline(8) })`
- `Terminal::insert_before(height, |buf| { ... })` — 在 viewport 上方插入内容
- `Terminal::autoresize()` — 自动检测终端尺寸变化

### 4.2 与 xylitol InlineTerminal 的关系/差异

**xylitol 当前实现**（`src/app/tui/terminal.rs`）：

```rust
pub struct InlineTerminal {
    term: init::DefaultTerminal,  // 即 Terminal<CrosstermBackend<Stdout>>
}

impl InlineTerminal {
    pub fn enter() -> io::Result<Self> {
        // 先打空行把光标推到底部
        let (_, h) = crossterm::terminal::size()?;
        for _ in 0..h { writeln!(stdout)?; }
        crossterm::terminal::enable_raw_mode()?;
        let term = init::try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(TAIL_HEIGHT),  // 8
        })?;
        Ok(Self { term })
    }

    pub fn draw_tail(&mut self, app: &TuiApp) -> io::Result<()> {
        self.term.draw(|frame| render::draw_tail_frame(frame, app))?;
        Ok(())
    }

    pub fn commit_to_scrollback(&mut self, lines: &[Line]) -> io::Result<()> {
        // 手动 wrap + 手写 cell 提交
        self.term.insert_before(height, |buf| { ... })?;
        Ok(())
    }
}
```

**对比**：

| 方面 | ratatui-core inline API | xylitol InlineTerminal |
|------|------------------------|----------------------|
| 创建 | `Terminal::with_options(Viewport::Inline(N))` | 包装了同一 API + 底部锚定逻辑 |
| 绘制 | `terminal.draw(\|frame\| ...)` | `draw_tail()` 委托给 `draw_tail_frame` |
| 追加内容到 scrollback | `terminal.insert_before(N, \|buf\| ...)` | `commit_to_scrollback()` 手动渲染 |
| 滚动区优化 | 有 `scrolling-regions` feature（xylitol 已启用） | 已利用 |
| 文本换行 | `Paragraph::wrap(Wrap)` 自动处理 | 手写 CJK 感知换行 + cell-by-cell 渲染 |
| **差异核心** | 库提供 `Paragraph` / `Line` widget 处理文本渲染 | xylitol 手写 cell 循环（c341 设计决策） |
| 终端恢复 | `init::restore()` + Drop | `Drop` 中自行 restore + disable_raw_mode |

**关键差异**：xylitol 的 `commit_to_scrollback` 手写了 CJK 感知的文本换行和 cell 渲染，而 ratatui 库的 `Paragraph` widget 已经提供了完整的文本布局能力（包括 CJK 宽字符处理）。

---

## 5. 对 xylitol 的启示

### 5.1 现在就能直接用的积木清单

以下积木不需要额外依赖（已在 `ratatui-core` 和 `ratatui-crossterm` 中）：

**ratatui-core**（已依赖）：
| 积木 | 位置 | 用途 |
|------|------|------|
| `Layout` + `Constraint` | `layout` | 窗口分区（对话区/输入框/状态栏） |
| `Rect` / `Position` / `Size` | `layout` | 区域操作 |
| `Line` / `Span` / `Text` | `text` | 文本渲染（当前已在用） |
| `Style` / `Color` / `Stylize` | `style` | 样式（当前在用）+ 链式简写 |
| `symbols::border` | `symbols` | 边框字符（替代手写 `─│┌┐└┘`） |
| `symbols::scrollbar` | `symbols` | 滚动条符号 |
| `symbols::block` / `shade` | `symbols` | 进度条/分割线 |
| `Buffer::with_lines` / `set_string` / `set_line` | `buffer` | 当前已直接使用 |
| `TestBackend` | `backend::test` | 自动化测试（零终端依赖） |
| `assert_eq!` / `Buffer::diff` | `buffer` | 渲染结果断言 |
| `Terminal::insert_before` | `terminal` | 当前已在用 |
| `Viewport::Inline` | `terminal` | 当前已在用 |
| `Terminal::draw` + `Frame` | `terminal` | 渲染管线 |

**ratatui-crossterm**（已依赖）：
| 积木 | 用途 |
|------|------|
| `CrosstermBackend` | 当前在用 |
| `scrolling-regions` feature | 已启用，提升 `insert_before` 性能 |

**需要额外依赖的积木（`ratatui-widgets`）**：
| 积木 | 当前手写量 | 改用库后的收益 |
|------|-----------|---------------|
| `Paragraph` | 大量：cell-by-cell 渲染 + CJK 换行 + 行布局 | 一行 `Paragraph::new(text).wrap(Wrap::default())` |
| `Block` | 少量：边框字符手动组装 | 一行 `Block::bordered().title(...)` |
| `List` + `ListState` | 中量：滚动/选择逻辑 | `List::new(items).highlight_symbol(">")` + `ListState` |
| `Clear` | 少量：reset 循环 | 一行 `Clear.render(area, buf)` |
| `Scrollbar` | 0（未实现） | 直接可用 |

### 5.2 TestBackend 能怎么搭 xylitol 的 e2e harness

**方案雏形**：`xylitol` 的 e2e harness 可以分为三层：

```
┌─────────────────────────────────────────────┐
│  Layer 3: BDD 场景测试 (rstest-bdd)          │
│  Given("用户启动 xylitol")                    │
│  When("用户输入 '你好' 并回车")               │
│  Then("渲染结果含 '你好' + 正确换行")          │
├─────────────────────────────────────────────┤
│  Layer 2: 渲染单元测试 (TestBackend + assert) │
│  test_render_input_box()                      │
│  test_render_streaming_reply()                │
│  test_render_dialog_overlay()                 │
│  test_scrollback_insert_before()              │
├─────────────────────────────────────────────┤
│  Layer 1: 纯逻辑测试 (无渲染)                  │
│  状态管理 / 事件处理 / 消息转换                │
└─────────────────────────────────────────────┘
```

**Layer 2 的具体实现思路**：

```rust
// tests/render/snapshots.rs
use ratatui_core::backend::TestBackend;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};

/// 工具函数：用 TestBackend 模拟一次 inline 渲染
fn render_to_buffer(app_state: &TuiApp) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions { viewport: Viewport::Inline(8) },
    ).unwrap();
    terminal.draw(|frame| {
        render::draw_tail_frame(frame, app_state);
    }).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn test_empty_input_shows_prompt() {
    let app = TuiApp::default();
    let buf = render_to_buffer(&app);

    // 断言底部输入行有提示符号
    assert_eq!(buf[(0, 7)].symbol(), "❯");
}

#[test]
fn test_streaming_reply_wraps_at_width() {
    // 模拟正在流的回复
    let mut app = TuiApp::default();
    app.start_stream();
    app.set_streaming_chunk("这是一段很长的中文回复，需要跨行显示在终端上。".into());

    let buf = render_to_buffer(&app);
    let expected = Buffer::with_lines([
        // ... 预期的多行渲染结果
    ]);
    assert_eq!(buf, expected);
}
```

**key insights**：
1. `TestBackend` 测试完全脱离终端——可在 CI 中直接 `cargo test`
2. 结合 `insta`（已在 dev-deps）做 snapshot 测试，自动对比渲染输出
3. 使用 `Buffer::with_lines` 直接描述预期的屏幕内容，可读性极高
4. 覆盖 inline viewport + insert_before 组合场景（scrollback 内容验证）

### 5.3 不该自建、该直接用库的

| 自建内容 | 应该用的库 | 原因 |
|---------|-----------|------|
| `commit_to_scrollback` 中的手写 CJK 换行 + cell-by-cell 渲染 | `Paragraph::new(text).wrap(Wrap::default())` + `Line::render` | `Paragraph` 已经正确处理 CJK 宽字符、字素聚类、换行。手写版容易出 off-by-one/宽度计算错误 |
| 手写边框字符 (`┌─┐│└─┘`) | `Block::bordered()` | `Block` 支持多种边框类型（圆角/双线/粗线）、标题定位、内边距 |
| 手写 `buffer.reset()` 循环 | `Clear::render()` | 一行代替嵌套循环，语义更清晰 |
| 手动实现的滚动选择逻辑 | `List` + `ListState` | 已经实现了高亮、自然滚动、选中同步 |
| 进度指示器 | `Gauge` / `LineGauge` / `Sparkline` | 标准美观的进度UI |
| 模式切换 | `Tabs` | 现成的标签选择栏 |
| **测试断言** | `assert_eq!(buf, Buffer::with_lines([...]))` + `Buffer::diff()` | 比手写逐 cell 断言更简洁、diff 输出更友好 |

**c341 设计决策回顾**：xylitol 选择了"不用 `ratatui-widgets`"，理由包括：
- 控制依赖体积
- 避免 widget 版本不稳定
- 手写渲染更灵活

**重新评估**：引入 `ratatui-widgets = "0.3"` 后：
- 依赖增加 ~1 crate（且 widget 之间共享 core 类型，编译增量小）
- 可省去 `commit_to_scrollback` 中 ~50 行手写 CJK 换行代码
- `render.rs` 中大量 cell-by-cell 渲染可简化为 widget 调用
- `Paragraph` 已经过广泛测试（包括 CJK、emoji VS16、零宽字符）
- 如果只需要部分 widget，可以 feature flag 控制（`all-widgets` 是 default 但可禁用）

**建议**：在 `tui` feature 下增加 `ratatui-widgets` 可选依赖，逐步替换手写渲染。优先替换 `Paragraph` 和 `Block`。

---

*本报告基于 ratatui 仓库 commit at 2026-07-01 的代码状态。所有文件行引用指向 `/home/l8ng/Projects/__straydragon__/ratatui/` 下对应路径。*
