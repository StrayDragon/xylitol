# _HANDOFF — TUI 渲染层重写（c399 pi-tui line-array 引擎）

> 交接日期：2026-07-06（阶段 2 完成）
> 分支：`feat/tui-dev`（13 个未 push commit，远端停在 `4c724c1`）
> 变更：`c399-tui-rewrite-pi-render-engine`（active，43/68 tasks 完成，**阶段 1+2 全部就位**）
> 接手者：用 `/llman-sdd-apply c399-tui-rewrite-pi-render-engine` 续做。

---

## 一、背景：为什么从 ratatui 换到 pi-tui

c396（已归档）修了 markdown 样式表（标题分级/引用前缀/有序列表/主题自适应），单测全过。但截图暴露根因：代码块围栏 ``` 原样显示、无语法高亮、无空行。这不是样式问题，是 **ratatui `Viewport::Inline` + `insert_before` 模型的结构性病灶**——StreamBuffer 段落切分（c377 fence-aware）破坏 markdown 结构后喂给独立 `render_markdown` 调用，commit 单元 = 渲染单元 = 段落，跨段落上下文丢失。

用户判断：ratatui inline-viewport 模型不合用，换 `.agents/skills/tui-pro-of-pi-tui/` 的 pi-tui 模式——**retained widget + line-array + differential rendering**。整个对话历史是一个 line-array，每帧对全量 diff，只写变更行（CSI 2026 同步输出）。渲染粒度天然 = 整条消息，跨段落上下文自动保留。

技能文档（terminal-foundations.md）明确：crossterm 在 Rust 上吸收了 pi-tui 最难的 UX 键模型模块（TS 版 ~1400 行 keys.ts + stdin-buffer.ts → Rust 版 crossterm `event::read() → KeyEvent`），所以 MVP ~1.5–3k 行。

---

## 二、已完成（43/68 tasks，~4120 行新代码，127 单测全过；阶段 1+2 完成）

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

---

## 三、未完成（25/68 tasks，阶段 3-5）

### 阶段 3：UX/交互（crossterm 大幅缩减）⏳
- `keybindings.rs`：`KeyId` + `KeybindingsManager`（可配置 + 冲突检测），替换现 hardcoded key 检查。
- 单焦点路由（handleInput: listeners → focus → focused.handle_input → requestRender）。
- input listeners（Ctrl+C abort / Ctrl+D quit / Ctrl+L force redraw）。
- bracketed paste：crossterm `Event::Paste`。
- Overlay 栈最小版（先不做完整 focus-restore 状态机）。
- **crossterm 已吸收**：键模型（KeyEvent）、stdin 分割（event::read 一次一个）、bracketed paste（Event::Paste）——无需移植 keys.ts/stdin-buffer.ts。

### 阶段 4：接入主循环 + 删除 ratatui ⏳（关键转折点）
- `mod.rs::run` 改用新 `Tui` 引擎；保留 Driver 调用 + `spawn_drain` 语义 + `Msg` 通道骨架。
- `app.rs::pending_tail` 改返回 `Vec<StyledLine>`（mutable tail 直接并入 line-array）。
- StreamBuffer 简化（line-array 天然整源上下文，fence-aware drain 可大幅简化或移除）。
- `render.rs::RenderedLine` seam 保留；`to_lines` 改产出 `Vec<StyledLine>`。
- **删除**：`components/` 目录、`terminal.rs`(旧)、`init.rs`(旧 ratatui 部分)、`InlineTerminal`/`commit_to_scrollback`/`draw_tail_frame`/`insert_before`。
- **Cargo.toml**：删 `ratatui-core`/`ratatui-crossterm`/`ratatui-widgets`，`tui` feature 重定义（`unicode-segmentation` 已在 2.3 加入）。
- `engine_ratatui_style_adapter.rs` 删除（syntect_highlight 改产 CellStyle）。

### 阶段 5：回归 + QA + 手动验证 ⏳
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
- `src/app/tui/engine/component.rs` — Component/Container/Focusable trait
- `src/app/tui/engine/tui.rs` — **核心引擎**（do_render + 三策略 + 硬宽度 + IME）
- `src/app/tui/engine/virtual_terminal.rs` — 测试 oracle
- `src/app/tui/widgets/text.rs` — Text/TruncatedText/Spacer
- `src/app/tui/widgets/markdown.rs` — Markdown widget（passthrough + code 高亮 + 扣子）
- `src/app/tui/widgets/input.rs` — **Input widget**（单行 Focusable + grapheme 光标 + 横向滚动 + take_outcome）
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
- `llmanspec/changes/c399-tui-rewrite-pi-render-engine/` — proposal/design/spec/tasks（38/68 完成）
- `llmanspec/specs/app-tui/spec.toon` — 已合并 c396 的 delta（tui61/tui66/tui71/tui78/tui79）

---

## 六、给接手者的建议

1. **先跑测试确认基线**：`just test`（既有 684 + 新增 127 = ~811 全绿）、`just lint`（0 警告）。
2. **从阶段 3 开始**（UX/路由层）——`/llman-sdd-apply c399-tui-rewrite-pi-render-engine`。**阶段 1+2 已全部完成**（引擎六模块 + widget 四模块）。阶段 3 是把引擎和 widget 连起来的交互层。
3. **阶段 3 路由层接 Input widget 的 take_outcome**：`engine/tui.rs::route_event` 是 stub，阶段 3 建单焦点路由时，在 `focused.handle_input()` 之后调 `input.take_outcome()` 把 Submit/Slash/Abort/Quit 传给主循环（参考 pi `tui.ts:827-833`）。Loader widget 的 `advance()` 也在此层接线（host 在 `Msg::Tick` 时调）。
4. **阶段 4 是关键转折点**：接入主循环 + 删 ratatui。建议在阶段 3（UX）完成后单独评估，确保引擎 + widget + 输入交互都就位再动接入。
5. **Markdown 扣子启用顺序**（用户提到）：标题分级 → 加粗/斜体 → 引用 → 链接 → 媒体预览 → 脚注 → 表格列对齐。每个在 `MarkdownTheme` flip 一个开关 + 在 `widgets/markdown.rs` 的 Renderer 加对应事件处理。
6. **token 节省原则不可破**：表格永远 tab/空格对齐（不画 Unicode 边框），代码块只颜色无框——用户明确要求复制干净。
7. **differential render 调试**：tui.rs 有 `last_changed_range`/`last_full_redraw_reason` 测试访问器；若 diff 行为难调，在 do_render 加 `PI_DEBUG_REDRAW` 式日志（技能强调无可见性无法调 diff 引擎）。

---

## 七、commit 历史（11 个未 push）

```
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
