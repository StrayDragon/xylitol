# c360 Design — 修订 c341 widget 约束 + TUI 渲染 harness

> 涉及放宽既有决策（tui21）+ 渲染实现迁移，按 propose skill 要求写 design。

## 1. 决策修订依据

### 1.1 c341 当初为何禁 widget

c341（`llmanspec/changes/archive/2026-07-01-c341-drop-umbrella-ratatui-use-core/design.md:27`）原文：

> **不**加 `ratatui-widgets`（umbrella 才有；我们弃用所有内置 widget）。

当时的理由（从 spec tui21 statement 推断）：
- 内置 widget 为 alt-screen 全屏重绘设计，不适配 inline mutable-tail + scrollback
- codex/pi/kimi 都手写所有组件，无 widget toolkit
- 控制依赖体积

### 1.2 为什么现在放宽

调研（`docs/tui-research/`）推翻了上述理由中的两条：

| c341 理由 | 调研事实 | 结论 |
|---|---|---|
| widget 为 alt-screen 设计 | `Paragraph::wrap` 在 inline 模式下正常工作（ratatui 官方 `tests/terminal.rs:66` 用 Inline + Paragraph + insert_before）；widget 与屏幕模式正交 | ❌ 理由不成立 |
| 三家都手写 | codex 用 umbrella ratatui 的 `WidgetRef`（`codex-rs/tui/src/render/renderable.rs`）；pi/kimi 因用 Ink/TS 根本没 ratatui widget 可用，"手写"是语言约束不是设计选择 | ❌ 类比不成立 |
| 控制依赖体积 | ratatui-widgets 是独立 crate，core 类型共享，编译增量小；`default-features = false` 可裁剪 | ⚠️ 部分成立，但可接受 |

**真正成立的约束**只剩"避免 CJK filler 显示成可见空格"（terminal.rs:72 注释）——这个用 harness 断言保护即可，不必因此禁所有 widget。

### 1.3 解禁的边界（不是"强制用库"，是"解除强制手写"）

c341 的 tui21 是**一刀切禁令**（MUST NOT 用任何 widget）。本变更新政策的核心是**选型由适配度驱动**，三条合法路径并存：

1. **复用库 widget**（默认倾向）：`ratatui_widgets::{paragraph::{Paragraph,Wrap}, block::Block, list::{List,ListItem,ListState}, clear::Clear}` 子集可用。当库 widget 正确处理 inline + CJK 时优先复用，避免重复造轮子。
2. **基于 ratatui-core 自建**：当库 widget 不符合需求（如特定 CJK filler 行为、自定义交互），MAY 基于 `ratatui_core` 原语（Buffer/Layout/Style/Text/Widget trait）构建自包含 widget。这同样是合法路径，不是退路。
3. **保留现有手写**：已验证可用的简单手写（如 StatusLine）可保留，不必为用库而用库。

**仍禁**：umbrella `ratatui` crate（tui20 不变）。`Scrollbar`/`Tabs`/`Gauge`/`Canvas` 等暂不在选型范围（按需再加，不在本变更）。

**关键**：判断标准是"适配度"而非"是否来自库"。harness（tui41）保证无论选哪条路径，渲染行为都可验收。

## 2. 渲染实现选型：按适配度决定（复用 OR 自建）

### 2.1 当前手写实现（要替换的）

`src/app/tui/terminal.rs:86-119` `commit_to_scrollback` 闭包：
```rust
self.term.insert_before(height, |buf| {
    let area = buf.area;
    for (i, line) in wrapped.iter().enumerate() {
        let y = area.y + i as u16;
        for x in area.x..area.right() { buf[(x, y)].reset(); }   // 清行
        let mut x = area.x;
        for span in &line.spans {
            for ch in span.content.chars() {
                let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                if w == 0 || w > 2 { continue; }
                buf[(x, y)].set_char(ch);
                buf[(x, y)].set_style(span.style);
                x += 1;
                if w == 2 && x < area.right() {                   // CJK 第二列 filler
                    buf[(x, y)].set_symbol("");
                    buf[(x, y)].set_style(span.style);
                    x += 1;
                }
            }
        }
    }
})?;
```

**问题**：① 零测试；② 不处理 grapheme cluster（emoji ZWJ）；③ 手写 CJK filler 是 terminal.rs:72 注释承认的 workaround。

### 2.2 选型评估流程（复用 OR 自建）

`commit_to_scrollback` 的替换**不预设结论**，按以下顺序评估：

1. **先固化当前行为**：harness 写测试覆盖当前手写实现的 CJK commit（双宽占两列、filler 行为），作为基准。
2. **试复用 Paragraph::wrap**：用 `Paragraph::new(text).wrap(Wrap{trim:false}).render(buf.area, buf)`（insert_before 闭包签名是 `FnOnce(&mut Buffer)` 无 Frame，直接调 `Widget::render` 匹配）。跑步骤 1 的断言。
3. **看结果决定**：
   - 若 Paragraph 的 CJK 处理与基准一致或更优（emoji grapheme 更好）→ **复用 Paragraph**，删除手写 cell 循环。
   - 若 Paragraph 有偏差（如 filler cell 内容不同导致终端渲染异常）→ **基于 ratatui-core 自建**一个最小的 inline 文本 widget，吸收当前手写的 filler 逻辑但用 `Buffer::set_line` 等更高层 API。跑步骤 1 断言。

无论走哪条路，harness 断言保护行为不变。这是"安全网"的核心价值。

### 2.3 CJK filler 行为保护（评估基准）

步骤 1 固化的断言：commit 一行中文（如"你好"）到 scrollback，断言 buffer 里双宽字符占两列。注意：断言用 `assert_buffer_lines`（比较渲染后的文本行）而非逐 cell 对比 symbol——避免把 filler 实现细节（手写版第二列 symbol 为空字符串）绑进测试。这正是"减少写死单测"的体现：测行为，不测实现。

### 2.4 wrap_to_width 的处置

`render.rs:162-191` `wrap_to_width` 在选型完成后**可能多余**（若复用 Paragraph::wrap，它自带 reflow）。**选型完成后评估**：若新实现完全覆盖换行需求，删除 wrap_to_width + 其 4 个单测（被 harness 取代）；若自建 widget 仍需预换行，保留。

## 3. 测试 harness 设计

### 3.1 层次（参考 ratatui 报告 §5.2）

本变更只建**渲染层 harness**（Layer 2），不碰 BDD（Layer 3）和纯逻辑（Layer 1 已有）：

```
Layer 2（本变更）: TestBackend + assert_buffer_lines
  - render_inline(app, w, h) -> Terminal<TestBackend>  (Viewport::Inline)
  - commit_and_assert(app, lines, expected)            (insert_before 路径)
Layer 1（已有）: app.rs 逻辑断言（喂 XyEvent 断言 Vec）
```

### 3.2 关键 helper（扩展 render.rs:284 现有 render_term）

```rust
fn render_inline(app: &TuiApp, width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    let mut term = Terminal::with_options(
        backend,
        TerminalOptions { viewport: Viewport::Inline(height) },
    ).unwrap();
    term.draw(|f| draw_tail_frame(f, app)).unwrap();
    term
}

fn commit_and_assert(app: &TuiApp, lines: Vec<Line>, expected: &[&str]) {
    let mut term = render_inline(app, 40, 10);
    term.insert_before(lines.len() as u16, |buf| {
        // 用 Paragraph（块2后）或当前手写逻辑（块1先固化）
        Paragraph::new(Text::from(lines)).wrap(Wrap{trim:false}).render(buf.area, buf);
    }).unwrap();
    term.draw(|f| draw_tail_frame(f, app)).unwrap();  // insert_before 后必须重绘
    term.backend().assert_buffer_lines(expected);
}
```

### 3.3 覆盖矩阵

| 测试 | 覆盖路径 | 断言 |
|---|---|---|
| 空 TuiApp tail | draw_tail_frame | 基线渲染 + cursor 位置 |
| 长中文流式 chunk | tail + wrap | 宽度感知换行（对应 wrap_to_width 行为） |
| commit 中文行 | insert_before + CJK filler | buffer 双宽字符占两列 |
| TurnEnd flush | tail 清空 | tail 无残留（回归 tui31，用 harness 重写） |

## 4. RenderedLine —— 业务/UI 边界 seam（tui42 载体）

参考 codex `HistoryCell`（20+ 变体，`codex-rs/tui/src/history_cell/`）和 pi 消息顺序（thinking/text/toolCall）。但本变更引入它的**首要目的不是"消息分类"，而是建立业务流与 UI 层的隔离边界**（tui42）。

### 4.1 当前耦合（要消除的）

```
XyEvent（domain 事件）
   ↓ 直接传入
render::commit_lines_for(event: &XyEvent)  ← 渲染层 match XyEvent 变体
app::handle_xy_event(event) -> Vec<Line>   ← 业务处理 + 渲染行生产 混在一起
```

问题：渲染层知道 `XyEvent::ToolExecutionEnd`/`XyEvent::ModelSelect` 等业务事件结构，业务词汇泄漏进 UI；改 UI 会牵动业务，反之亦然。

### 4.2 目标边界（RenderedLine 作为 seam）

```
XyEvent（domain 事件）
   ↓ 单一翻译点（seam，在 app.rs）
RenderedLine（UI 专用数据）
   ↓
渲染层（render.rs）只消费 RenderedLine，不知道 XyEvent 存在
```

```rust
/// UI 层的专用数据类型 —— 业务流在此终止，渲染层只消费这个。
/// XyEvent → RenderedLine 的翻译是单向、唯一的 seam（tui42）。
pub enum RenderedLine<'a> {
    UserInput(&'a str),
    AssistantText(&'a str),
    ToolSummary { name: &'a str, result: &'a str },
    Status(&'a str),
}
```

### 4.3 落地改动

- `render::commit_lines_for` 签名从 `(event: &XyEvent)` 改为消费 `&[RenderedLine]`（或 `Vec<RenderedLine>`），不再 match XyEvent。
- `app::handle_xy_event` 拆分：业务状态更新（streaming/pending）留在 app，XyEvent→RenderedLine 的翻译集中到 seam 函数（如 `translate_event(event) -> Vec<RenderedLine>`）。
- 渲染层（`draw_tail_frame`/`commit_to_scrollback`）只接收 `RenderedLine`/`TuiApp`（TuiApp 的公开 API 也不暴露 XyEvent）。

### 4.4 设计权衡

用 enum 而非 trait object——编译期穷尽匹配，适合固定的少量消息类型；未来加 thinking/diff 只是加变体。enum 是值类型，无生命周期/分配开销（`'a` 借用），适合渲染热路径。**不**实现 thinking/diff 变体（延后），但 seam 一旦建立，后续加变体不影响渲染层稳定性。

## 5. 不做的事（防 scope creep）

- ❌ 不动 c330/c335/c345/c350（功能提案，用户明确延后）
- ❌ 不建 fake Driver（独立工作，本变更只测渲染层）
- ❌ 不实现 thinking/diff/overlay/对话框（P1-P2，后续变更）
- ❌ 不切 alt-screen（inline 是三家共识）
- ❌ 不强求删除所有手写渲染——只替换 commit 路径（最脆弱+零测试），StatusLine 等简单的保留

## 6. 风险

### R1：Paragraph 的 CJK filler 与手写版行为不一致【中】
手写版用空 symbol 占第二列；Paragraph 用 unicode-width 算宽度后自动占两列。两者终端渲染应一致，但 buffer cell 内容可能不同（手写版第二列 symbol 为空字符串）。
**缓解**：harness 断言用 `assert_buffer_lines`（比较渲染后的文本行）而非逐 cell 对比，避免绑死 filler 实现细节。这正是"减少写死单测"的体现。

### R2：ratatui-widgets 版本与 core 不匹配【低】
ratatui-widgets 0.3.2 与 ratatui-core 0.1.2 在同一 workspace（umbrella 0.30.2 即此组合），版本兼容。
**缓解**：Cargo.toml 锁 `ratatui-widgets = "0.3"`，CI 跑全套测试。

### R3：迁移后视觉回归【中】
替换 commit 路径可能引入肉眼不可见的渲染差异。
**缓解**：① harness 断言保护；② 真终端冒烟（tasks 块4 手动验收）；③ 分两步——先建 harness 固化当前行为，再替换实现。
