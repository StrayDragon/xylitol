---
name: tui-expert-of-codex
description: 'OpenAI Codex CLI 项目 TUI 交互层深度分析。基于 ratatui + crossterm 构建的工业级终端应用架构，涵盖事件循环/EventBroker、帧调度、弹性布局、流式渲染、终端兼容性探测等核心模式。用于指导新 Agent TUI 的实现。In-depth analysis of OpenAI Codex CLI TUI layer built with ratatui + crossterm: event loop/EventBroker, frame scheduling, elastic layout, streaming rendering, terminal capability detection. Use when building Rust-based agent TUIs. Keywords: Rust, ratatui, crossterm, TUI, terminal UI, 终端UI, event loop, EventBroker, frame scheduling, elastic layout, streaming, Codex CLI'
allowed-tools: Read Write Edit Glob Grep Bash WebSearch WebFetch
metadata:
  based_on: "OpenAI Codex CLI (https://github.com/openai/codex)"
  ratatui_version: "0.28"
  analysis_lang: "Rust"
disable-model-invocation: true
---

# tui-expert-of-codex

基于 OpenAI Codex CLI 项目的 TUI/CLI 交互层的深度分析，覆盖从终端初始化到渲染管线的全部设计精髓。

## 核心架构要点

### 事件系统
- **4 源 tokio select!** 事件循环：AppEvent（mpsc 通道）、TuiEvent（crossterm 广播+draw）、ThreadEvent、AppServerEvent
- **EventBroker 模式**：通过 drop EventStream 彻底释放 stdin，支持外部编辑器集成
- **键盘增强**：DISAMBIGUATE_ESCAPE_CODES | REPORT_EVENT_TYPES | REPORT_ALTERNATE_KEYS，带 WSL/tmux 兼容检测

### 渲染管线
- **FrameScheduler Actor**：请求合并+coalescing + 120 FPS 限速
- **crossterm::SynchronizedUpdate**：原子终端重绘，避免中间状态闪烁
- **双渲染路径**：legacy draw()（cursor 启发式）vs draw_with_resize_reflow()（直接 viewport 计算）
- **双区域流模型**：stable region（scrollback）+ mutable tail（自由重渲染）
- **表格 holdback**：pipe table 未完成前行全部留在 tail，finalize() 一次性提交

### 布局系统
- **Renderable trait**：render() + desired_height() + cursor_pos() + cursor_style()
- **FlexRenderable**：Flutter 风格两步算法（非弹性分配 → 弹性按比例分配）
- **ColumnRenderable / RowRenderable / InsetRenderable**

### 终端兼容性
- **100ms 启动探针**：光标位置、默认颜色、键盘增强支持度
- **WSL 跨环境探测**：cmd.exe /c set TERM_PROGRAM
- **CIE76 感知颜色距离**：sRGB→XYZ→Lab，TrueColor/256/16 三级回退
- **自适应背景样式**：根据终端实际背景色动态调整
- **^Z suspend/resume**：两路径恢复（RealignInline / RestoreAlt）

## 使用指引

遇到 TUI 相关设计问题时，先查阅 `references/tui-expert-of-codex.md` 中对标的分析和源码引用。重点关注：

1. **要实现流式输出** → 双区域流模型 + 表格 holdback
2. **要实现键盘处理** → EventBroker + 键盘增强 + paste 处理
3. **要实现布局** → Renderable + FlexRenderable
4. **要处理终端兼容** → 启动探针 + 颜色探测 + WSL 检测
5. **要集成外部编辑器** → with_restored() 8 步模式

## Guardrails

- 不要照搬 Codex CLI 的源码，而是提取可复用的设计模式
- 流式渲染的 stable/tail 分区是核心难点，需要仔细设计新行门控策略
- 终端兼容性检测应尽量提前（启动期），避免运行时动态检测造成闪烁
- EventBroker 模式是 stdin 管理的通用解法，建议所有 Agent TUI 都实现

## References

- 完整分析文档: `references/tui-expert-of-codex.md`
- 源码引用索引在文档 section 10
