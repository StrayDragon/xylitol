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
- [ ] 引擎状态（previous_lines/previous_width/height/viewportTop/cursorRow/hardwareCursorRow/focus/overlay/renderRequested/lastRenderAt）。
- [ ] `requestRender(force)` + `schedule`（16ms cap coalesce）。
- [ ] `doRender` 管线：render tree → composite overlays（stub）→ extract cursor → applyLineResets → 选策略 → positionHardwareCursor → save。
- [ ] 三策略：firstRender（不清屏）/ fullRender(true)（清屏+清scrollback）/ 正常 diff（move + `\x1b[2K` + line）。
- [ ] 硬宽度不变量（diff 路径 + fullRender 都检查，修 pi-tui audit Finding 2）。
- [ ] 同步输出包裹（CSI 2026）。
- [ ] viewport/scrollback math（previousViewportTop 跟随末尾、maxLinesRendered、width/height change 触发 fullRender）。
- [ ] 虚拟 IME 光标（positionHardwareCursor）。
- [ ] `stop()` clean exit（移到内容末尾 + 换行 + restore）。

### 1.6 测试 harness（virtual_terminal.rs）
- [ ] 内存 cell-grid + `feed(ansi)` 解析（SGR/cursor/clear/sync）+ assert_row/assert_contains/assert_cursor。
- [ ] ANSI 解析器（vte crate 或手写状态机）。
- [ ] 单测：diff 不变量（append-only 只写新行、in-place 只写该行、width change 全量、shrink 清孤立）。

## 阶段 2：模块 2 widget 系统（`src/app/tui/widgets/`）

### 2.1 基础 widget
- [ ] `Text`（wrap + cache + invalidate）。
- [ ] `Spacer`（n 空行）。
- [ ] `TruncatedText`（单行截断，状态行用）。
- [ ] 单测：wrap 宽度、cache 命中/失效。

### 2.2 Markdown widget（复用 c396 样式逻辑）
- [ ] `markdown_render.rs` 产出类型 ratatui `Line` → 自有 `StyledLine`（~50 行样式映射）。
- [ ] `Markdown` widget 包 `render_markdown` + cache by (源签名, width) + invalidate。
- [ ] `MarkdownTheme`（for_user/for_assistant/for_thinking），保留 c396 COLORFGBS 主题自适应。
- [ ] token 节省：表格 tab 对齐、代码块无框、列表无树连接器（延续 c395/c396 原则）。
- [ ] 单测：标题分级、引用 > 前缀、有序列表、代码块高亮（移植 c396 测试）。

### 2.3 Input widget
- [ ] 单行 Focusable（横向滚动 + grapheme 光标 + CURSOR_MARKER）。
- [ ] 复用现有 `cursor_x_at` CJK 逻辑 + `InputOutcome`（Submit/Slash/Abort/Quit/Idle）。
- [ ] handle_input：crossterm KeyEvent → InputOutcome。
- [ ] 单测：光标移动、CJK 边界、insert at cursor。

### 2.4 Loader widget
- [ ] spinner（复用 SPINNER const）+ 自调度（setInterval 等价：tokio interval + requestRender）。

## 阶段 3：模块 3 UX/交互

- [ ] `keybindings.rs`：KeyId + KeybindingsManager（defaults + overrides + 冲突检测 + matches）。
- [ ] 单焦点路由（handleInput: listeners → focus → focused.handle_input → requestRender）。
- [ ] input listeners（Ctrl+C abort / Ctrl+D quit / Ctrl+L force redraw）。
- [ ] bracketed paste（crossterm Event::Paste → Input 插入）。
- [ ] Overlay 栈最小版（showOverlay/hide + preFocus restore，不做完整状态机）。
- [ ] 适配现有 commands.rs（/exit /model /help 分发）。

## 阶段 4：接入

- [ ] `mod.rs::run` 改用新 Tui 引擎；保留 Driver 调用 + spawn_drain 语义 + Msg 通道骨架。
- [ ] `app.rs::pending_tail` 改返回 `Vec<StyledLine>`（mutable tail 直接并入 line-array）。
- [ ] StreamBuffer 简化（line-array 天然整源上下文，fence-aware drain 可大幅简化或移除）。
- [ ] `render.rs::RenderedLine` seam 保留；`to_lines` 改产出 `Vec<StyledLine>`。
- [ ] 删除 `components/` 目录 + `terminal.rs`(旧) + `init.rs`(旧 ratatui 部分)。
- [ ] 删除 `InlineTerminal`/`commit_to_scrollback`/`draw_tail_frame`/`insert_before` 路径。
- [ ] Cargo.toml：删 ratatui-core/crossterm-ratatui/ratatui-widgets，加 unicode-segmentation，tui feature 重定义。

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
