---
change_id: c399-tui-rewrite-pi-render-engine
title: TUI 渲染层重写——ratatui inline-viewport → pi-tui line-array + differential rendering
status: proposed
priority: 399
depends_on: []
author: agent
---

# c399-tui-rewrite-pi-render-engine

## Why

c396 截图暴露：代码块围栏 ``` 原样显示、无语法高亮、无空行。根因是 StreamBuffer 段落切分（c377 fence-aware）破坏 markdown 结构后喂给独立 `render_markdown` 调用——这是 ratatui `insert_before` + 固定 viewport 模型的结构性病灶（`_HANDOFF.md` 第二节论证：commit 单元 = 渲染单元 = 段落，跨段落上下文丢失）。

c396 修了样式表但治不了这个架构病。用户判断：**ratatui inline-viewport 模型本身不合用**，换 `.agents/skills/tui-pro-of-pi-tui/` 的 pi-tui 模式——retained widget + line-array + differential rendering。

pi-tui 模式下，整个对话历史是一个 line-array，每帧对全量 diff，只写变更行（CSI 2026 同步输出）。流式新增 = append（写新行 + 滚动跟随）。**渲染粒度天然 = 整条消息**（c397 想解决的问题自动消失），**resize 接线是引擎基础能力**（c398 需求并入）。

### 为什么是完整移植而非修补

- ratatui 的 `Buffer`/`Viewport::Inline`/`insert_before` 与 pi-tui 的 line-array + diff 是**两种不兼容的渲染范式**（技能 terminal-foundations.md 的两条设计轴）。混合不成立（技能 L321-323：「Two engines coexisting is not coherent」）。
- pi-tui 在 Rust + crossterm 上成本大降：crossterm **吸收了最难的 UX 键模型模块**（TS 版 ~1400 行 keys.ts + stdin-buffer.ts，Rust 版变成 `event::read() → KeyEvent`）。MVP ~1.5–3k 行。

## What Changes

### 模块 1：渲染引擎（`src/app/tui/engine/`）

新建 pi-tui 风格的 line-array + differential render engine（技能 rendering-engine.md）：
- `style.rs` — 自有轻量类型 `CellStyle { fg, bg, bold, italic, ... }` + `Cell` + `StyledLine`，含 ANSI 序列化（`to_ansi_string()`）。**完全删除 ratatui-core**。
- `width.rs` — ANSI-aware width utilities（`visible_width`/`truncate_to_width`/`wrap_text_with_ansi`），用 `unicode-width` + `unicode-segmentation` + 小型 ANSI 状态机。
- `terminal.rs` — `Terminal` 抽象（crossterm 包装：raw mode/size/event/cursor/synchronized output/clean stop + signal handlers）。替换现 `init.rs` + `terminal.rs`。
- `tui.rs` — `Tui` 引擎状态（previous_lines/viewport/cursor 双轨/focus/overlay stack/render scheduling 16ms cap）+ `do_render` 管线 + 三策略（首帧不清/全量重绘/行 diff + `\x1b[2K`）+ 硬宽度不变量 + 同步输出（CSI 2026）+ 虚拟 IME 光标。
- `virtual_terminal.rs` — 内存 cell-grid 测试 harness（喂 ANSI 流，断言 cell 内容/光标）。

### 模块 2：widget 系统（`src/app/tui/widgets/`，替换 `components/`）

- `component.rs` — `Component` trait（`render(width) -> Vec<StyledLine>` + `handle_input` + `invalidate`）+ `Container`（垂直栈）+ `Focusable` + `CURSOR_MARKER`。
- `text.rs`/`spacer.rs` — 基础 widget（wrap + cache + invalidate 协议）。
- `markdown.rs` — **复用 c396 的 `markdown_render` 逻辑**（改成产出 `StyledLine`）+ COLORFGBS 主题。**遵守 token 节省原则**：表格 tab/空格对齐纯文本（不画 Unicode 边框），代码块只 syntect 颜色无框，整体只有颜色 + 结构前缀（标题 `#`、引用 `>`、列表 `1.`/`•`）。
- `input.rs` — 单行 Focusable 输入（横向滚动 + grapheme 光标 + 复用现有 CJK `cursor_x_at` 逻辑）。
- `loader.rs` — spinner（复用 SPINNER const）+ 自调度。

### 模块 3：UX/交互（crossterm 大幅缩减）

- 键模型：直接用 crossterm `KeyEvent`/`KeyCode`（不移植 keys.ts）。
- `keybindings.rs` — `KeyId` + `KeybindingsManager`（可配置 + 冲突检测），替换现 hardcoded key 检查。
- 单焦点路由 + input listeners（Ctrl+C/L 等 app-wide key）。
- Overlay 栈最小版（先不做完整 focus-restore 状态机）。
- bracketed paste：用 crossterm `Event::Paste`。

### 接入（改造现有代码）

- `tui::run` 主循环改用新引擎；保留 Driver 复用契约 + spawn_drain 语义。
- `app.rs` 的 `pending_tail` 改返回 `Vec<StyledLine>`；StreamBuffer 的 fence-aware drain 简化（整源 render 天然保留 markdown 上下文）。
- 删除 `InlineTerminal`/`commit_to_scrollback`/`draw_tail_frame`/`insert_before` 路径。
- 删除 `components/` 整个目录（widget 全部重写进 `widgets/`）。
- `RenderedLine` enum + `xyevent_to_rendered` seam 保留（UI 数据类型不变）。

## Capabilities

- `app-tui`：渲染引擎范式替换（modify tui1/tui12/tui41/tui50/tui70 等 inline-viewport 相关 req；add 新的 line-array/diff/width-contract req）。

## Impact

- **代码**：`src/app/tui/` 大部分重写（现 4713 行）。保留 `app.rs` 状态机主体、`commands.rs`/`input.rs` 分发、`render.rs` 的 `RenderedLine` seam、`markdown_render` 样式逻辑。
- **依赖**：删除 `ratatui-core`/`ratatui-crossterm`/`ratatui-widgets`；保留 `crossterm`（新增 cursor/synchronized output）、`unicode-width`/`syntect`/`two-face`/`pulldown-cmark`；新增 `unicode-segmentation`。
- **测试**：TestBackend 全废，新建 `virtual_terminal` 测试 harness；保留行为断言（CJK/commit/streaming），harness 换；新增 differential render 不变量测试。
- **spec**：大幅 modify app-tui（inline-viewport → line-array），add 新引擎 req。
- **SKILL 文档**：同步 `.agents/skills/write-tui/SKILL.md`（依赖段 ratatui → crossterm + 自有引擎）与 `src/app/tui/AGENTS.md`。
- **风险**：中高。跨整个 TUI 渲染层，但保留 Driver/XyEvent/RenderedLine seam + markdown 样式。分阶段实施，每模块独立验证。

## 变更处理

- **归档 c396**：commit 19b115e 的样式逻辑（标题/引用/列表/主题）在本变更的 Markdown widget 复用。c396 归档时标注「代码已落地，完整效果由 c399 新引擎承接」。
- **删除 c397/c398**：渲染粒度解耦（c397）在 line-array 天然满足；resize（c398）是 c399 引擎基础能力。两者需求并入本变更。

## 不在范围

- 完整 focus-restore 状态机（overlay 最小版先行）
- kill-ring/undo（Input 精简先行）
- 图片 widget（Kitty/iTerm2 graphics protocol）
- ansi-to-tui（自有类型已定，无需 ANSI 解析库）
