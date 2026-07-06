# c399-tui-rewrite-pi-render-engine — Tasks

> 大型重写变更。分 6 阶段，每阶段独立编译/测试验证后再进下一阶段。
> 技能 SSOT：`.agents/skills/tui-pro-of-pi-tui/`，本文件是 xylitol 落地清单。

## 阶段 0：SDD 重组

- [x] propose c399（本变更工件）。
- [ ] 归档 c396（design.md 标注「代码已落地，完整效果由 c399 承接」）。
- [ ] 删除 c397/c398 目录（需求并入 c399）。
- [ ] 同步 `.agents/skills/write-tui/SKILL.md`（依赖段 ratatui → crossterm + 自有引擎）与 `src/app/tui/AGENTS.md`。
- [ ] `llman sdd validate c399-tui-rewrite-pi-render-engine` 通过。

## 阶段 1：模块 1 渲染引擎（`src/app/tui/engine/`）

### 1.1 自有样式类型（style.rs）
- [x] `CellStyle { fg, bg, bold, italic, underline, dim, crossed_out }` + `Color` 枚举 + `Cell` + `StyledLine`。
- [x] `StyledLine::to_ansi_string(width)` 序列化（SGR 转义 + 行末 `\x1b[0m\x1b]8;;\x07`）。
- [x] 单测：style 相等/合并、ANSI 序列化往返、颜色枚举映射。
- [x] 校验：`cargo test --features tui -- engine::style`。

### 1.2 ANSI-aware width utilities（width.rs）
- [x] `visible_width(line) -> usize`（ANSI/SGR = 0，CJK = 2，combining = 0）。
- [x] `truncate_to_width(line, width, ellipsis)`（不劈开宽字符）。
- [x] `wrap_text_with_ansi(text, width) -> Vec<StyledLine>`（跨行携带 SGR 状态）。
- [x] 单测：CJK width、ANSI 截断、wrap 保色。
- [x] 校验：`cargo test --features tui -- engine::width`。

### 1.3 Terminal 抽象（terminal.rs）
- [x] `Terminal` struct（crossterm 包装：raw mode / size / event::poll+read / cursor / BeginSynchronizedUpdate / clean stop）。
- [x] `start()` / `stop()` 生命周期 + bracketed paste + signal handlers（SIGINT/SIGTERM/SIGHUP）。
- [x] panic hook 保留（复用 c355 install_terminal_restore_hook 思路）。
- [x] 校验：手动 raw mode 进出 + 写一个字符串（集成测试）。
  - CapturingTerminal 测试替身（捕获 write + 喂合成 event）覆盖 write/event/resize。

### 1.4 Component/Container/Focusable（component.rs）
- [x] `Component` trait（render/handle_input/invalidate）+ `Focusable` + `CURSOR_MARKER`。
- [x] `Container`（垂直栈 render = children 拼接）。
- [x] 单测：Container 拼接、invalidate 传播。

### 1.5 Tui 引擎核心（tui.rs）
- [x] 引擎状态（previous_lines/previous_width/height/viewportTop/cursorRow/hardwareCursorRow/focus/overlay/renderRequested/lastRenderAt）。
- [x] `requestRender(force)` + `schedule`（16ms cap coalesce）。
- [x] `doRender` 管线：render tree → composite overlays（stub）→ extract cursor → applyLineResets → 选策略 → positionHardwareCursor → save。
- [x] 三策略：firstRender（不清屏）/ fullRender(true)（清屏+清scrollback）/ 正常 diff（move + `\x1b[2K` + line）。
- [x] 硬宽度不变量（diff 路径 + fullRender 都检查，修 pi-tui audit Finding 2）。
- [x] 同步输出包裹（CSI 2026）。
- [x] viewport/scrollback math（previousViewportTop 跟随末尾、maxLinesRendered、width/height change 触发 fullRender）。
- [x] 虚拟 IME 光标（positionHardwareCursor）。
- [x] `stop()` clean exit（移到内容末尾 + 换行 + restore）。

### 1.6 测试 harness（virtual_terminal.rs）
- [x] 内存 cell-grid + `feed(ansi)` 解析（SGR/cursor/clear/sync）+ assert_row/assert_contains/assert_cursor。
- [x] ANSI 解析器（vte crate 或手写状态机）。
- [x] 单测：diff 不变量（append-only 只写新行、in-place 只写该行、width change 全量、shrink 清孤立）。

**阶段 1 完成标志**：渲染引擎全部就位（style/width/terminal/component/tui/virtual_terminal 六模块，69 单测全过）。引擎可独立验证，待阶段 2 widget + 阶段 4 接入后可用。

## 阶段 2：模块 2 widget 系统（`src/app/tui/widgets/`）

### 2.1 基础 widget
- [x] `Text`（wrap + cache + invalidate）。
- [x] `Spacer`（n 空行）。
- [x] `TruncatedText`（单行截断，状态行用）。
- [x] 单测：wrap 宽度、cache 命中/失效。

### 2.2 Markdown widget — 纯透传 + 代码块高亮 + 语法处理器扣子
- [x] Markdown widget：pulldown-cmark 解析，纯文本透传（去语法字符）+ fenced code block 走 syntect 高亮。
- [x] `MarkdownTheme`：per-element 开关（heading_grading/bold/italic/quote_prefix/link_expand/list_marker/table_align），全默认 false（passthrough），flip 即启用该元素处理器——渐进增强扣子。
- [x] 复用 c396 syntect_highlight（经 engine_ratatui_style_adapter 适配 ratatui Style → CellStyle）。
- [x] 块级元素（Paragraph/Heading/BlockQuote/List/Item）End 事件插空行分隔，SoftBreak/HardBreak 换行，finish 去尾部空行。
- [x] 单测：纯文本透传、代码块高亮（fence 消费 + span 带色）、未知语言 plain fallback、标题/粗体/链接 passthrough、段落分隔、CJK wrap、代码块与后续内容空行分隔。

### 2.3 Input widget
- [x] 单行 Focusable（横向滚动 + grapheme 光标 + CURSOR_MARKER）。
- [x] 复用现有 `cursor_x_at` CJK 逻辑 + `InputOutcome`（Submit/Slash/Abort/Quit/Idle）。
- [x] handle_input：crossterm KeyEvent → InputResult + `take_outcome()` 旁路（参考 pi 回调模型）。
- [x] 单测：光标移动（ASCII/CJK/emoji-ZWJ）、CJK 边界、insert at cursor、outcome（Submit/Slash/Abort/Quit）、render 横向滚动（cursor 末尾/中间/开头）、width 不变量。

### 2.4 Loader widget
- [x] spinner（复用 SPINNER const）+ host tick 驱动（`advance()` 方法，无内部 timer——主循环已有 `Msg::Tick`）。

**阶段 2 完成标志**：widget 系统全部就位（text/markdown/input/loader 四模块，阶段 2.1-2.4 共 58 单测）。待阶段 3 UX 路由 + 阶段 4 接入后可用。

## 阶段 3：模块 3 UX/交互

- [x] `keybindings.rs`：KeyId + KeybindingsManager（defaults + overrides + 冲突检测 + matches）。crossterm KeyEvent 已归一化协议，不做 pi 的三协议解码。
- [x] 单焦点路由（`Tui::handle_event`：listeners → root.handle_input（Container 按 focused_index 转发）→ take_outcome → request_render）。
- [x] input listeners（`InputListener` trait + `ListenerResult{Consume,Rewrite,Pass}`；Ctrl+L force redraw 由 host 注册 listener；Ctrl+C/D 是 widget keybinding 不是 listener，参考 pi tui.ts:825）。
- [ ] bracketed paste（crossterm Event::Paste → Input 插入）——推迟到阶段 4 接入时（主循环才见 Event::Paste）。
- [ ] Overlay 栈最小版——**推迟**（聊天 UI 当前不需要 modal；design.md 说最小版先行，留到真正需要 settings dialog 时）。
- [ ] 适配现有 commands.rs——**推迟到阶段 4**（host loop 拿 `UxOutcome::Slash(body)` 调 `commands::dispatch`；engine 已返回纯 outcome，对接在 host loop 不在 engine）。

**阶段 3 完成标志（核心）**：keybindings + 单焦点路由 + input listeners 就位（engine 自洽，UxOutcome 下沉，Container focus 转发）。bracketed paste / overlay / commands 对接随阶段 4 主循环接入落地。

## 阶段 4：接入

- [ ] `mod.rs::run` 改用新 Tui 引擎；保留 Driver 调用 + spawn_drain 语义 + Msg 通道骨架。
- [ ] `app.rs::pending_tail` 改返回 `Vec<StyledLine>`（mutable tail 直接并入 line-array）。
- [ ] StreamBuffer 简化（line-array 天然整源上下文，fence-aware drain 可大幅简化或移除）。
- [ ] `render.rs::RenderedLine` seam 保留；`to_lines` 改产出 `Vec<StyledLine>`。
- [ ] 删除 `components/` 目录 + `terminal.rs`(旧) + `init.rs`(旧 ratatui 部分)。
- [ ] 删除 `InlineTerminal`/`commit_to_scrollback`/`draw_tail_frame`/`insert_before` 路径。
- [ ] Cargo.toml：删 ratatui-core/crossterm-ratatui/ratatui-widgets，tui feature 重定义（unicode-segmentation 已在 2.3 加入）。

## 阶段 5：回归 + QA

- [ ] `just fmt` + `just lint`（clippy 无新告警）。
- [ ] `just test`（全绿；含 virtual_terminal 新测试 + 移植的行为测试）。
- [ ] arch_guard 通过（TUI 不 import crate::agent/infra）。
- [ ] `cargo run --features tui -- --help` 无回归。
- [ ] 手动验证：
  - [ ] 长文档（标题各级/有序无序列表/嵌套引用/代码块/表格）渲染正确。
  - [ ] 流式逐字增量无断裂、留白一致（c397 想解决的）。
  - [ ] resize（窗口缩放 + 字体缩放）自适应、不崩（c398 需求）。
  - [ ] 表格/代码块复制 token 干净（D6 原则）。
  - [ ] COLORFGBS 亮/暗主题切换（c396 资产）。
  - [ ] Ctrl+C abort / Ctrl+D quit / Ctrl+L redraw / 斜杠命令。
- [ ] `llman sdd validate c399-tui-rewrite-pi-render-engine --strict` 通过。
- [ ] 更新 `_HANDOFF.md`（重写完成，第二/三梯队需求由 c399 承接）。
- [ ] 归档 c399。
