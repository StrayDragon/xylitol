# c399 Design — pi-tui line-array + differential render 引擎

> 范围：完整三模块移植（技能 `tui-pro-of-pi-tui/`），Rust + crossterm。
> 本文件记录关键架构决策与风险权衡。技能文档是行为 SSOT，本文件只记「在 xylitol 里怎么落地 + 偏离技能的地方」。

## 核心架构决策

### D1. 完全删除 ratatui，新建自有轻量样式类型

**验证事实**：ratatui-core 的 `Style/Line/Span` 是纯数据类型（本身不依赖 `Buffer` 运行时），但**它们无法直接序列化成 ANSI 字节流**（无 `to_string()` 产出 `\x1b[...m`）。而 pi-tui line-array 模式的核心是「widget 产出带 ANSI 的字符串行，engine diff 字符串相等」——必须自控 ANSI 序列化。

**决策**：新建 `src/app/tui/engine/style.rs`：
```rust
#[derive(Clone, Copy, Default, PartialEq)]
pub struct CellStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed_out: bool,
    // reset 标志：用于 applyLineResets 的 \x1b[0m
}
pub struct Cell { pub ch: char, pub style: CellStyle }
pub struct StyledLine { pub cells: Vec<Cell> }  // 或 pub spans: Vec<Span>
```
- `StyledLine::to_ansi_string(width) -> String`：把 cells 序列化成 `\x1b[<SGR>m<text>`，每行末尾加 `\x1b[0m\x1b]8;;\x07`（技能「styles do not cross lines」不变量）。
- `Color` 自有枚举（Black/Red/.../Rgb(u8,u8,u8)/Indexed(u8)），~30 行。

**对 c396 的影响**：`markdown_render.rs` 现产出 `ratatui_core::text::Line`，改成产出 `StyledLine`。样式映射：`Modifier::BOLD → bold: true`、`Color::Cyan → fg: Some(Color::Cyan)` 等，~50 行机械替换。`RenderStyle` 字段类型同步改。markdown 解析逻辑（pulldown-cmark Writer 状态机）**不动**，只改产出类型。

**收益**：line-array diff 完全自控，无 ratatui 死代码，类型最小化。

### D2. line-array 模型：scrollback 与 mutable tail 合并

**现状（要替换）**：ratatui `Viewport::Inline(TAIL_HEIGHT)` 固定 mutable 区 + `insert_before` 不可逆 commit scrollback。两区割裂导致跨段落上下文丢失（HANDOFF 根因）。

**新模型**：整个对话历史是 `previous_lines: Vec<StyledLine>`，每帧 `do_render`：
1. widget 树 `render(width)` 产 `new_lines: Vec<StyledLine>`（含全部历史 + 当前流式尾部 + 输入面板）。
2. diff `previous_lines` vs `new_lines`，找 first/last changed，只重写变更行（`\x1b[2K` + line）。
3. append（流式新增）= previous 末尾之后的新行，用 `\r\n` 滚动写入。

**天然解决**：
- c397 渲染粒度：widget 看到完整消息源，markdown 结构完整。
- c398 resize：width change → `fullRender(true)` 全量重绘（技能 Step 5B）。
- 跨段落留白：renderer 看全貌，`needs_newline` 机制可用。

### D3. Component trait 与 widget 产出

```rust
pub trait Component {
    fn render(&self, width: usize) -> Vec<StyledLine>;
    fn handle_input(&mut self, key: &KeyEvent) -> InputResult { InputResult::NotHandled }
    fn invalidate(&mut self);
}
pub trait Focusable: Component {
    fn focused(&self) -> bool;
    fn set_focused(&mut self, v: bool);
}
```
- `Container::render` = children 线性拼接 line 数组（垂直栈）。
- `CURSOR_MARKER`（`\x1b_pi:c\x07`）由 Focusable widget 在 focused 时插入，engine strip + 定位硬件光标（IME）。

**聊天 UI 的 widget 树**：
```
Container (root)
├── Container (history: 已完成的对话行)
│   └── Text/Markdown widgets per message
├── Text (mutable streaming tail, 若在 streaming)
├── Text (status line: spinner + Working/Ready)
└── Input (focusable 输入面板)
```
每帧整树重 render → line-array → diff。流式时 history 区不变（diff 跳过），只尾部变。

### D4. crossterm 吸收的 UX 模块（不移植）

技能 ux.md 的 Step 1-2（stdin 分割 + 跨协议键模型）在 TS 版是 ~1400 行（keys.ts + stdin-buffer.ts）。**Rust + crossterm 下这些变成零**：
- crossterm `event::read()` 一次产出一个 `Event`（KeyEvent/Resize/Paste），天然分割。
- `KeyEvent { code, modifiers, kind }` 天然协议无关（Kitty/modifyOtherKeys/legacy 都归一）。
- bracketed paste = `Event::Paste(String)`。
- 同步输出 = `BeginSynchronizedUpdate`/`EndSynchronizedUpdate`。

**保留的 UX 工作**：keybinding registry（`KeyId` + `KeybindingsManager`，可配置 + 冲突检测）+ 单焦点路由 + input listeners + overlay 栈最小版。

### D5. 去边框（pi 风格）

BottomPanel 的 `─` 边框 + `ratatui_widgets::Block` 丢弃。输入区用空行分隔 + 可选 dim 背景。更简洁，与 pi/codex 观感一致。

### D6. token 节省的 markdown 渲染原则

**用户约束**：markdown 渲染要「只有颜色 + token 节省」，避免复制时产生噪声。具体：
- **表格**：tab/空格对齐的纯文本（复制即干净 markdown 源），**不画** `╔═╤═╗` Unicode 边框。
- **代码块**：只 syntect 颜色高亮，**不画** `╭───╮` 框线/背景/语言标签。
- **列表**：`1.`/`•` 前缀，**不画** `├─└─` 树连接器。
- **引用**：`>` 前缀（c396 已定）。
- **标题**：`#` 前缀 + 颜色/粗细分级（c396 已定）。

这条原则延续 c395 的「不画 box-drawing」精神，并在 Markdown widget 里强制。

## 测试策略（TestBackend 全废）

### virtual_terminal harness（新建）
`src/app/tui/engine/virtual_terminal.rs`：内存 cell-grid（`Vec<Vec<Cell>>` + cursor pos）。
- `feed(&mut self, ansi: &str)`：解析 ANSI 流（SGR/cursor/clear/sync），更新 grid。
- `assert_row(y, expected)` / `assert_contains(substr)` / `assert_cursor(x, y)`。
- 对应 pi-tui 的 `@xterm/headless` VirtualTerminal。

需一个轻量 ANSI 解析器（喂给 virtual_terminal，与 widget 的 ANSI 序列化互逆）。可选 `vte` crate 或手写 ~100 行状态机。

### 保留的行为断言
现有 `row_text(buf, y, width)` helper 模式保留，改读 virtual_terminal 的 grid。CJK 换行、commit 内容、streaming tail 的断言逻辑不变。

### 新增 differential render 不变量测试（对标 porting-checklist smoke matrix）
- append-only：加行只写新行，无 scrollback 污染。
- in-place edit：单行变化只重写该行。
- width change：全量重绘 + 重新换行。
- shrink：clearOnShrink 清孤立行。
- CJK width：truncate 不劈开宽字符。
- ANSI truncate：颜色不泄漏到截断后。

## 实施风险与缓解

| 风险 | 缓解 |
|---|---|
| differential render 调试难 | 实现技能的 `PI_DEBUG_REDRAW`/`PI_TUI_DEBUG` 等日志 hook（diff 理由 + firstChanged + previous/newLines 转储），无可见性无法调 diff |
| cursor 双轨（cursorRow vs hardwareCursorRow）合并 bug | 严格按技能 Step 2 分字段，单测覆盖 IME 定位 |
| ANSI 序列化/解析不对称 | virtual_terminal 的解析与 style.rs 的序列化互为逆运算，property test 对拍 |
| widget invalidate 协议遗漏 | 引擎在 theme change / requestRender(true) 时统一调 invalidate，单测覆盖缓存命中 |
| 流式 widget 树每帧重 render 性能 | Container 层级缓存（history 区内容签名不变则复用）；Markdown widget 缓存 by (源签名, width) |

## 与 codex 的关系（不照搬）

HANDOFF 提到的 codex `StreamCore`（raw_source 累积 + recompute_streaming_render + commit-tick 动画 + finalize canonicalize）是 **retained-on-cell-grid（ratatui 同范式）** 上的优化。本变更切到 **retained-on-line-array（pi 范式）**，codex 的 commit-tick 动画/finalize/StableRegion 不需要（line-array 每帧全量 diff 天然处理）。借鉴 codex 的「流式 == 整体渲染」不变量测试思想（controller_loose_vs_tight），但实现路径不同。

## 不在范围

- 完整 overlay focus-restore 状态机（最小版先行，技能 Step 6 minimal port）
- kill-ring/undo（Input 精简先行，技能 Step 5 编辑原语按需补）
- 图片 widget（Kitty/iTerm2 graphics protocol）
- Editor widget 的完整能力（autocomplete/paste markers/jump-to-char）——聊天输入用单行 Input 够，多行 Editor 按需
