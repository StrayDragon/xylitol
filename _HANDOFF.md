# _HANDOFF — TUI 渲染层重写（c399 pi-tui line-array 引擎）

> 交接日期：2026-07-07（阶段 4 完成，待手动验证 + 归档）
> 分支：`feat/tui-dev`（15 个未 push commit，远端停在 `4c724c1`）
> 变更：`c399-tui-rewrite-pi-render-engine`（active，阶段 1-4 完成；剩手动验证 + 归档）
> 接手者：**阶段 5 收尾**——跑真实终端手动验证清单，全过后 `llman sdd validate --strict` + 归档。

---

## ⚠️ 当前状态（2026-07-07）

**阶段 4 已完成**：新引擎已接入主循环，ratatui 完全删除。
- `cargo check --features tui` ✅、`just fmt` ✅、`just lint`（clippy 无告警）✅
- `cargo test --features tui` 758 测全绿（lib 672 + bdd 85 + 1）
- arch_guard 4/4 ✅（TUI 不 import agent/infra）
- `llman sdd validate`（非 strict）通过（剩 warning 是未勾的手动验证项 + overlay 推迟项 + 归档项）

**待办（阶段 5 收尾，需用户）**：
1. **手动验证清单**（tasks.md 阶段 5，需真实终端）：`cargo run --features tui` 后逐项验收。
2. 全过后 `llman sdd validate c399-tui-rewrite-pi-render-engine --strict` + 勾手动项。
3. 归档 c399。

---

## 一、背景：为什么从 ratatui 换到 pi-tui

c396（已归档）修了 markdown 样式表（标题分级/引用前缀/有序列表/主题自适应），单测全过。但截图暴露根因：代码块围栏 ``` 原样显示、无语法高亮、无空行。这不是样式问题，是 **ratatui `Viewport::Inline` + `insert_before` 模型的结构性病灶**——StreamBuffer 段落切分（c377 fence-aware）破坏 markdown 结构后喂给独立 `render_markdown` 调用，commit 单元 = 渲染单元 = 段落，跨段落上下文丢失。

用户判断：ratatui inline-viewport 模型不合用，换 `.agents/skills/tui-pro-of-pi-tui/` 的 pi-tui 模式——**retained widget + line-array + differential rendering**。整个对话历史是一个 line-array，每帧对全量 diff，只写变更行（CSI 2026 同步输出）。渲染粒度天然 = 整条消息，跨段落上下文自动保留。

技能文档（terminal-foundations.md）明确：crossterm 在 Rust 上吸收了 pi-tui 最难的 UX 键模型模块（TS 版 ~1400 行 keys.ts + stdin-buffer.ts → Rust 版 crossterm `event::read() → KeyEvent`），所以 MVP ~1.5–3k 行。

---

## 二、已完成（46/68 tasks，~4810 行新代码，156 单测全过；阶段 1+2+3 核心完成）

### SDD 重组（commit `2cdbd89`）
- propose `c399-tui-rewrite-pi-render-engine`（proposal/design/delta spec/tasks 全工件，spec modify tui1/tui12/tui41/tui50/tui70 + add tui82 硬宽度/tui83 resize）。
- 归档 `c396`（样式资产标注转入 c399；design.md 补归档说明）。
- 删除 `c397`/`c398`（渲染粒度解耦在 line-array 天然满足；resize 是 c399 引擎基础能力）。
- `write-tui` SKILL 顶部加重写进行中标注。

### 阶段 1：渲染引擎（六模块，69 单测）✅
按技能 `rendering-engine.md` 实现，路径 `src/app/tui/engine/`：

| 文件 | 行 | 职责 |
|---|---|---|
| `style.rs` | 437 | 自有类型 `CellStyle`/`Color`/`Span`/`StyledLine` + ANSI 序列化（`to_sgr`/`to_ansi`）。**完全脱离 ratatui**。 |
| `width.rs` | 373 | `truncate`/`wrap`/`wrap_plain`（CJK-safe）+ `marker_aware_width`（ANSI 解析，APC 认 BEL 终止）。 |
| `terminal.rs` | 188 | `Terminal` trait + `ProcessTerminal`（crossterm 后端）+ `CapturingTerminal`（测试替身）。 |
| `component.rs` | 219 | `Component`/`Focusable` trait + `Container`（垂直栈）+ `InputResult`。 |
| `tui.rs` | 768 | **核心引擎**：状态（previous_lines/cursor 双轨/viewport）+ `do_render` 管线 + 三策略（首帧不清/全量重绘/diff）+ 硬宽度不变量（两条路径都查，修 audit Finding 2）+ 同步输出 + IME 光标。 |
| `virtual_terminal.rs` | 505 | 测试 oracle：ANSI 流 → cell-grid，端到端断言。 |

**关键设计决策**（c399 design.md D1-D6）：
1. **完全删除 ratatui，自有类型**：ratatui Style 无 ANSI 序列化能力，line-array diff 需自控。
2. **scrollback 与 mutable tail 合并**：单一 line-array，每帧全量 diff，无「commit 不可逆」。
3. **去边框**（pi 风格）：BottomPanel 的 `─` 丢弃。
4. **token 节省原则**：表格 tab 对齐纯文本（不画 Unicode 边框），代码块只 syntect 颜色无框。

### 阶段 2.1+2.2：基础 widget + Markdown（18 单测）✅
路径 `src/app/tui/widgets/`：

| 文件 | 行 | 职责 |
|---|---|---|
| `text.rs` | 160 | `Text`（wrap+cache）/`TruncatedText`（单行截断）/`Spacer`（N 空行）。 |
| `markdown.rs` | 449 | **Markdown widget**：pulldown-cmark 解析，**纯文本透传 + 只有代码块高亮**（见下）。 |
| `input.rs` | 777 | **Input widget**（阶段 2.3 落地，见下）。 |
| `loader.rs` | 243 | **Loader widget**（阶段 2.4 落地，见下）。 |

`engine_ratatui_style_adapter.rs`（82 行）：过渡适配器，ratatui `Style` → `CellStyle`。**阶段 4 删 ratatui 后此模块删除**（syntect 直接产 CellStyle）。

**Markdown widget 策略**（用户 directive，重要）：
- **现阶段**：文本 pulldown-cmark 解析后**纯透传**（去语法字符：`# Title` → "Title"，`**b**` → "b"），**只有 fenced code block 走 syntect 高亮**，其它语法元素无样式。
- **留好扣子**：`MarkdownTheme` 有 8 个 per-element 开关（`enable_heading_grading`/`enable_bold`/`enable_italic`/`enable_quote_prefix`/`enable_link_expand`/`enable_list_marker`/`enable_table_align`），全默认 false（passthrough），flip 即启用该元素样式化——渐进增强。
- **不换 comrak**（用户确认继续用 pulldown-cmark；comrak README 作为「扣子」参考——脚注/wikilink/alerts/CJK emphasis 等未来可探索）。
- 块级元素 End 事件插空行分隔；finish 去尾部空行。

### 阶段 2.3：Input widget（26 单测）✅
路径 `src/app/tui/widgets/input.rs`（777 行）。pi `components/input.ts` 的 Rust 移植。

**核心设计决策**：
1. **Outcome 通道 = `take_outcome()` 轮询**（参考 pi）。pi 的 Input/Editor 用可变回调字段 `onSubmit?: (value) => void`——widget 在 `handleInput` 内命中 submit 时**主动调 host 回调**（`input.ts:22-23,101-103`，路由 `tui.ts:761-835`）。Rust 里回调会形成 widget↔host 循环引用，惯用等价物是**轮询**：widget 内部记 `pending_outcome: Option<InputOutcome>`，host 在 `handle_input` 后调 `take_outcome()` 取走。语义一致（"handleInput 后取走 submit"），形态不同。`Component::handle_input` 契约不改（仍返回 `InputResult{Handled,NotHandled}`），submit/abort/quit 旁路。
2. **grapheme 级光标**（用户确认）。新增 `unicode-segmentation` 依赖（Cargo.toml + tui feature），光标按 grapheme cluster 边界移动，正确处理 `👨‍👩‍👧` 这类 ZWJ 序列（pi 用 `Intl.Segmenter`，Rust 对应 `UnicodeSegmentation::graphemes`）。**原计划阶段 4 加此依赖，提前到 2.3**，阶段 4 Cargo.toml 任务相应简化。
3. **横向滚动**（pi `input.ts:378-445`）：half-width bias 让 cursor 居中；窗口计算用 grapheme + `unicode-width`；`CURSOR_MARKER`（focused 时）+ reverse-video 假光标。
4. **不在范围**（design.md 已定）：kill-ring/undo/word-navigation/bracketed-paste/Kitty CSI-u 解码——留扣子。

**顺带修的引擎 bug**：`StyledLine::width()` 原用裸 `UnicodeWidthStr::width`，对 `CURSOR_MARKER`（APC 序列 `\x1b_pi:c\x07`）算成 5 宽（中间 `_pi:c` 是可见 ASCII），但终端实际 0 宽。改用 `marker_aware_width`（剥离 ANSI/APC）。这影响引擎硬宽度不变量检查（`tui.rs:198`）和 IME 光标定位——**任何 Focusable widget 发射 marker 都受益**。

**遗留**：`TuiApp` 的 `input`/`input_cursor` 字段 + 旧 `input.rs::handle` 自由函数**不动**（阶段 4 接入时再迁移状态到 widget，阶段 3 建路由层时 `handle` 被路由替代）。

### 阶段 2.4：Loader widget（14 单测）✅
路径 `src/app/tui/widgets/loader.rs`（243 行）。pi `components/loader.ts` 的 Rust 移植。

**核心设计决策**：
1. **host tick 驱动，无内部 timer**。pi Loader 持 `NodeJS.Timeout` + 每 80ms 调 `ui.requestRender()`（"widgets don't schedule renders" 的唯一例外）。c399 改为 widget 暴露 `advance()` 方法，host 主循环在 `Msg::Tick`（已存在，驱动旧 spinner）时调用 + requestRender。语义一致（steady cadence 推进帧），但 widget 保持纯（无 I/O/async/timer）——timer 是 host 关注点。
2. **SPINNER const 自带**（10 帧 braille，与 c380 `components::spinner::SPINNER` 和 pi `DEFAULT_FRAMES` 一致）。阶段 4 删 `components/` 后此 widget 自包含。
3. **spinner 颜色**：`SPINNER_COLOR = Color::Cyan`（对应 c396 `theme::Palette::spinner`）。阶段 4 把 ratatui `Style` 迁到 CellStyle 后，改为走 theme 回调。
4. **render 单行**：`<frame> <message>`，超宽用 `truncate(..., "…")`。pi Loader 前置空行做垂直分隔——c399 Container 垂直栈，调用方加 `Spacer` 即可，widget 只出内容行。

**阶段 2 完成标志**：widget 系统全部就位（text/markdown/input/loader 四模块，阶段 2.1-2.4 共 58 单测）。待阶段 3 UX 路由 + 阶段 4 接入后可用。

### 阶段 3：UX/交互层核心（29 单测）✅
引擎和 widget 的连接层。新建两个 engine 模块 + 改造 Component/Container/Input。**核心完成**（keybindings + 路由 + listeners）；bracketed paste / overlay / commands 对接随阶段 4 主循环接入。

**新增模块**：
| 文件 | 行 | 职责 |
|---|---|---|
| `engine/outcome.rs` | 60 | `UxOutcome { Submit, Slash, Abort, Quit, Idle }`——engine 自有的 UX outcome 枚举（下沉自上层 `input.rs`，避免 engine 反向依赖）。 |
| `engine/keybindings.rs` | 280 | `KeybindingsManager`（KeyId + 默认表 + user overrides + `matches(KeyEvent, id)` + 冲突检测）。crossterm 已归一化协议，不做 pi 三协议解码。 |

**核心设计决策**：
1. **UxOutcome 下沉到 engine**（用户决策）。engine 不能反向依赖上层 `input.rs::InputOutcome`（arch_guard + 依赖方向）。新建 `engine/outcome.rs`，widgets/input.rs 改用它。旧 `InputOutcome` 保留（阶段 4 删）。两枚举形状一致，阶段 4 host loop 翻译层直接对译。
2. **Container focus 寻址 = `focused_index`**（避免自引用借用）。pi 的 `focusedComponent` 是引用句柄（TS 引用语义），Rust 所有权模型无法让 Tui 同时拥有 root + 借用 child。Container 持 `focused_index: Option<usize>`，`handle_input`/`set_focused`/`take_outcome` 按 index 转发。与 component.rs 注释一致。
3. **`set_focused` 提升到 Component trait**（默认 no-op），Focusable 变标记 trait。Rust trait object 限制：`Box<dyn Component>` 无法跨 trait 调 `Focusable::set_focused`，所以合并到 Component（Container/Input 覆盖转发）。
4. **Ctrl+C/D 是 widget keybinding，不是 input listener**（pi 事实）。pi `tui.ts:825` 注释 + `interactive-mode.ts:2503`：ctrl+c→`app.clear`、ctrl+d→`app.exit` 是 widget action。input listeners 只做 consume/rewrite 级全局拦截（Ctrl+L force redraw）。keybindings.rs 让这些键可配置，Input widget 的 handle_input 改查 `kb.matches(key, "app.clear")`。
5. **`Tui::handle_event`**（替换自由函数 route_event）：listeners → root.handle_input（Container 转发）→ take_outcome → request_render。返回 `Option<UxOutcome>`，host loop 翻译（不持 Driver）。

**推迟到阶段 4**：bracketed paste（主循环才见 Event::Paste）、overlay（聊天 UI 不需要 modal）、commands 对接（host loop 拿 Slash 调 `commands::dispatch`）。

### 阶段 4：接入主循环 + 删除 ratatui ✅
单次大重构（合并原计划 4a/4b/4c，避免 ratatui/tui 双轨中间态）。**ratatui 完全删除**，新引擎全权接管。

**核心设计决策**：
1. **host ↔ engine 桥 = `Rc<RefCell<T>>` + `SharedComponent` shim**（关键）。引擎拥有 `root: Box<dyn Component>`，但 `set_root` 是 `#[cfg(test)]`、`root` 是 trait object 无法 downcast 到 `Container::add`——**引擎没有生产 API 让 host 增长渲染树**。技能明确「host 自己拥有 transcript surface」(`ux.md:285-301`)。解法：transcript/input/loader 三个 widget 各包 `Rc<RefCell<>>`，再包一层 `SharedComponent<T: Component>`（impl Component 转发到 borrow）塞进 root Container；host 保留 `Rc` 克隆直接 mutate，引擎 render 时 borrow。两者不重叠（host 在 select! 分支 mutate，引擎在 try_render 时 borrow）。
2. **主循环 = spawn_blocking poll + tokio::select! 三路**（用户决策方案 A）。键盘任务用独立 `event::poll/read`（spawn_blocking），与 `ProcessTerminal::write` 到 stdout 不冲突——新引擎**不做 DSR 光标查询**（旧 ratatui inline 才有那个 stdin 争用），所以更安全。select! 在键盘 Msg/XyEvent/Tick 三路。保留 `spawn_drain` 语义。
3. **`RenderedLine` seam 保留**（UI 数据类型仍是好设计）。`to_lines(width)` 改产 `Vec<StyledLine>`：markdown 变体调新 `widgets::markdown::render_markdown`（passthrough + 代码块高亮）；单行变体用 `theme::Palette` 的 `CellStyle`。`xyevent_to_rendered` seam 不变。
4. **StreamBuffer 保留**（不简化）。line-array 不改变逐字流式的 pending/committed 边界语义；fence-aware drain 保证代码块完整 commit 让高亮正确——仍是必需。
5. **theme.rs 迁到 CellStyle**。`Palette` 方法从返回 ratatui `Style` 改返回 `engine::style::CellStyle`。颜色枚举一一对应。
6. **syntect_highlight 迁移**。`StyleSegment.style` 从 `ratatui_core::style::Style` 改 `engine::style::CellStyle`；`highlight()` 直接产 CellStyle，**删除** `engine_ratatui_style_adapter.rs`。文件从 `components/` 迁到 `src/app/tui/syntect_highlight.rs`。
7. **pending_tail 双轨**。`app.rs::TuiApp::pending_tail()` 仍返回 `(&str, MutableKind)`；host 的 `pending_tail_rows(app, width)` helper 把它 wrap 成 `Vec<StyledLine>` 喂给 `TranscriptWidget::set_pending`——line-array 模型下语义等价（mutable tail 每帧覆盖 pending 区）。

**删除**（~2700 行旧代码 + ~82 测）：
- `src/app/tui/components/` 整个目录（bottom_panel/input_prompt/markdown/markdown_render/mod/mutable_line/spinner/status_line/tail/transcript_line；syntect_highlight 迁出后删）。
- `src/app/tui/terminal.rs`（旧 InlineTerminal）、`src/app/tui/input.rs`（旧 `handle` + `InputOutcome`，被 `UxOutcome` + `widgets::input` 取代）。
- `src/app/tui/engine_ratatui_style_adapter.rs`。
- `init.rs` 的 ratatui 部分（`DefaultTerminal`/`try_init_with_options`/`restore`；raw mode 生命周期由 `ProcessTerminal` 接管；保留 panic hook）。
- `app.rs` 的 input 字段 + 方法（`push_char`/`cursor_*`/`backspace`/`take_input`/`input_buffer`/`input_cursor` 等）——input 状态搬到 `widgets::input::Input`。
- Cargo.toml：删 `ratatui-core`/`ratatui-crossterm`/`ratatui-widgets`；`tui` feature 重定义（`crossterm`/`unicode-width`/`unicode-segmentation`/`syntect`/`two-face`/`pulldown-cmark`）。

**新增**：
- `src/app/tui/transcript.rs`：`TranscriptWidget`（host 拥有的会话累积器，`lines` finalized + `pending` mutable tail；5 单测）。
- `src/app/tui/mod.rs::SharedComponent<T>`：`Rc<RefCell<T>>` → `Component` 的转发 shim。
- `src/app/tui/mod.rs::HostAction` + `handle_term_event` + `apply_host_action` + `apply_xy_event`：UxOutcome 翻译逻辑抽函数（可测，不持 Driver 的纯路由部分分离）。
- `widgets/input.rs::insert_paste`：bracketed paste（Event::Paste → grapheme 级插入）。

**测试变化**：840 → 758 测（删 ~82 ratatui TestBackend 绑定的旧测：render.rs 的 commit_harness/cursor_tests + components/* 的 widget 测）。行为覆盖转移：新 engine/widgets 的 156 测（virtual_terminal diff 不变量 + width CJK wrap + markdown 代码块高亮 + input grapheme 光标 + loader）已等价覆盖核心行为；markdown 扣子样式（c396 heading/bold/quote）是**有意降级**（c399 design：passthrough 默认，扣子留渐进增强），非回归。

---

## 三、未完成（阶段 5 收尾，需用户手动验证）

### 阶段 4 余项 ⏳（无——已完成，见上）
- `just fmt`/`lint`/`test`/`qa` 全绿。
- arch_guard 通过（TUI 不 import crate::agent/infra）。
- **手动验证清单**（给用户的验收指引）：
  - 长文档（标题/列表/引用/代码块/表格）渲染正确。
  - 流式逐字增量无断裂、留白一致。
  - resize（窗口缩放 + 字体缩放）自适应、不崩。
  - 表格/代码块复制 token 干净（D6 原则）。
  - COLORFGBS 亮/暗主题切换。
  - Ctrl+C abort / Ctrl+D quit / Ctrl+L redraw / 斜杠命令。

---

## 四、关键约束与复用契约（不可破坏）

### 必须保留（重写不能动）
- **`Driver` trait**（`run/abort` + `EventStream`）—— 一字不改。
- **`XyEvent` 枚举** —— 无关渲染。
- **`RenderedLine` enum + `xyevent_to_rendered` seam** —— UI 数据类型，line-array 正需要。
- **`commands.rs`/`input.rs` 的 `CommandOutcome`/`InputOutcome`** —— 纯分发。
- **arch_guard**（TUI 不 import `crate::agent`/`crate::infra`，只经 `crate::app::core::driver::Driver`）。

### 复用资产（不要重写）
- `components/syntect_highlight.rs`：`highlight` + `StyleSegment`（c396，阶段 4 改产 CellStyle）。
- `components/markdown_render.rs`：**c396 的样式逻辑作参考**（标题分级/引用前缀/有序列表），但新 Markdown widget 已在 `widgets/markdown.rs` 重写（pulldown-cmark 事件驱动，passthrough 模式）。阶段 4 删旧 `components/markdown_render.rs`。
- `components/spinner.rs`：`SPINNER` const。
- `theme.rs`：`Palette`（阶段 4 适配新 CellStyle）。

---

## 五、关键代码索引

### 新引擎（c399，主体）
- `src/app/tui/engine/style.rs` — CellStyle/Color/Span/StyledLine + ANSI 序列化
- `src/app/tui/engine/width.rs` — truncate/wrap/marker_aware_width
- `src/app/tui/engine/terminal.rs` — Terminal trait + ProcessTerminal + CapturingTerminal
- `src/app/tui/engine/component.rs` — Component/Container/Focusable trait（**阶段 3**：Container 加 focused_index 转发，set_focused 提升到 Component，Focusable 变标记）
- `src/app/tui/engine/outcome.rs` — **UxOutcome**（engine 自有 UX outcome，下沉自 input.rs）
- `src/app/tui/engine/keybindings.rs` — **KeybindingsManager**（KeyId + 默认表 + matches + 冲突检测）
- `src/app/tui/engine/tui.rs` — **核心引擎**（do_render + 三策略 + 硬宽度 + IME + **阶段 3** `handle_event` 路由 + input listeners）
- `src/app/tui/engine/virtual_terminal.rs` — 测试 oracle
- `src/app/tui/widgets/text.rs` — Text/TruncatedText/Spacer
- `src/app/tui/widgets/markdown.rs` — Markdown widget（passthrough + code 高亮 + 扣子）
- `src/app/tui/widgets/input.rs` — **Input widget**（单行 Focusable + grapheme 光标 + 横向滚动 + take_outcome + **阶段 3** keybindings 查询替代硬编码）
- `src/app/tui/widgets/loader.rs` — **Loader widget**（spinner + advance + host tick 驱动）
- `src/app/tui/engine/style.rs` — CellStyle/Color/Span/StyledLine + ANSI 序列化（`width()` 用 `marker_aware_width`，2.3 修复）
- `src/app/tui/engine_ratatui_style_adapter.rs` — 过渡适配器（阶段 4 删）

### 旧代码（阶段 4 删除，现共存）
- `src/app/tui/terminal.rs` — InlineTerminal（Viewport::Inline + insert_before）
- `src/app/tui/init.rs` — ratatui 初始化
- `src/app/tui/components/` — 旧 widget（ratatui Widget trait）
- `src/app/tui/render.rs` — RenderedLine seam（保留）+ draw_tail_frame（删）

### 参考技能
- `.agents/skills/tui-pro-of-pi-tui/` — pi-tui 三模块 SSOT（engine/widgets/ux + references/）
- `.agents/skills/write-tui/SKILL.md` — 顶部有重写进行中标注

### SDD 工件
- `llmanspec/changes/c399-tui-rewrite-pi-render-engine/` — proposal/design/spec/tasks（46/68 完成）
- `llmanspec/specs/app-tui/spec.toon` — 已合并 c396 的 delta（tui61/tui66/tui71/tui78/tui79）

---

## 六、给接手者的建议

1. **先跑测试确认基线**：`just test`（既有 684 + 新增 156 = ~840 全绿）、`just lint`（0 警告）、arch_guard 4/4。
2. **从阶段 4 开始**（接入主循环 + 删 ratatui）——`/llman-sdd-apply c399-tui-rewrite-pi-render-engine`。**阶段 1+2+3 核心已全部完成**（引擎 + widget + UX 路由层）。
3. **阶段 4 接入要点**：`mod.rs::run` 改用 `Tui::handle_event` 替代旧 `handle_key` + `InputOutcome`；host loop 拿 `UxOutcome::{Submit,Slash,Abort,Quit}` 翻译（形状与旧 `InputOutcome` 一致，直接对译）；`commands::dispatch` 在 host loop 调（engine 不持 Driver）；Loader `advance()` 在 `Msg::Tick` 调；`app.rs::pending_tail` 改返回 `Vec<StyledLine>`；删 ratatui 依赖 + `components/` 目录。
4. **UxOutcome 翻译层**（阶段 4）：`engine::outcome::UxOutcome` 与旧 `input::InputOutcome` 形状一致。host loop match `handle_event` 返回值，Submit/Slash 走原 `InputOutcome::Submit/Slash` 路径（driver.run / commands::dispatch），Abort/Quit 走原路径。旧 `InputOutcome` + `input.rs::handle` 接入后删。
5. **Markdown 扣子启用顺序**（用户提到）：标题分级 → 加粗/斜体 → 引用 → 链接 → 媒体预览 → 脚注 → 表格列对齐。每个在 `MarkdownTheme` flip 一个开关 + 在 `widgets/markdown.rs` 的 Renderer 加对应事件处理。
6. **token 节省原则不可破**：表格永远 tab/空格对齐（不画 Unicode 边框），代码块只颜色无框——用户明确要求复制干净。
7. **differential render 调试**：tui.rs 有 `last_changed_range`/`last_full_redraw_reason` 测试访问器；若 diff 行为难调，在 do_render 加 `PI_DEBUG_REDRAW` 式日志（技能强调无可见性无法调 diff 引擎）。

---

## 七、commit 历史（14 个未 push）

```
57d2e2d feat(tui): c399 阶段 3 — UX 层（keybindings + 单焦点路由 + input listeners）
320332a feat(tui): c399 阶段 2.4 — Loader widget（spinner + host tick 驱动，阶段 2 完成）
1529f95 feat(tui): c399 阶段 2.3 — Input widget（单行 Focusable + grapheme 光标 + 横向滚动）
0507751 docs: update _HANDOFF — c399 pi-tui 重写进度交接（38/68 tasks，阶段 1 完成）
7939c44 feat(tui): c399 阶段 2.1+2.2 — 基础 widget + Markdown（纯透传 + 代码块高亮）
4ac31fb feat(tui): c399 阶段 1.6 — virtual_terminal 测试 harness（阶段 1 完成）
2fe9c56 feat(tui): c399 阶段 1.5 — engine tui.rs（differential render 核心引擎）
0504288 feat(tui): c399 阶段 1.4 — engine component.rs（Component/Container/Focusable）
e5659c2 feat(tui): c399 阶段 1.3 — engine terminal.rs（crossterm Terminal 抽象）
5dd0f19 feat(tui): c399 阶段 1.2 — engine width.rs（StyledLine 截断/换行）
c7ecb7d feat(tui): c399 阶段 1.1 — engine style.rs 自有样式类型 + ANSI 序列化
2cdbd89 docs(sdd): propose c399 TUI pi-render-engine rewrite; archive c396; drop c397/c398
971a379 agentdev: update skills（环境侧，非本任务）
19b115e feat(tui): markdown 样式分级修复 (c396 第一梯队)
a0f762a docs(sdd): propose c396/c397/c398 — TUI markdown rendering改进规划
```
