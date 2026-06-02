---
name: tui-expert-of-crush
description: '当需要用 Go + Bubble Tea v2 构建终端 TUI 应用，或参考 Charmbracelet Crush 的 TUI 架构设计（Elm Architecture 事件循环、Ultraviolet ScreenBuffer 矩形布局、lipgloss 样式、集中式单模型架构）时使用。Use when building terminal UIs with Go + Bubble Tea v2 / Ultraviolet / lipgloss, or referencing Crush TUI architecture (Elm Architecture event loop, ScreenBuffer rect layout, centralized single-model design). Keywords: Go, Golang, Bubble Tea, bubbletea, Charmbracelet, Ultraviolet, lipgloss, TUI, terminal UI, 终端UI, Elm Architecture, ScreenBuffer, Crush'
allowed-tools: Read Write Edit Glob Grep Bash
disable-model-invocation: true
metadata:
  tech_stack: "Go + Bubble Tea v2 + Ultraviolet + lipgloss v2"
  source_project: "Charmbracelet Crush"
  rendering_mode: "Ultraviolet ScreenBuffer + lipgloss"
---

# tui-expert-of-crush

> 从 Crush（Charmbracelet 出品的终端 AI 编程助手）源码中提炼的 TUI 架构设计精髓。
> 基于 Go + Bubble Tea v2 + Ultraviolet 技术栈。

## 架构概览

Crush 采用**集中式单模型架构**——只有一个 Bubble Tea Model（`UI` struct），子组件不参与标准 Elm 消息循环，而是暴露命令式方法由主模型直接调用。渲染使用**混合模式**：顶层通过 Ultraviolet `ScreenBuffer` 做矩形布局，子组件返回字符串再绘入对应区域。

### 技术栈

| 层级 | 技术 | 作用 |
|------|------|------|
| TUI 框架 | `charm.land/bubbletea/v2` | Elm Architecture 事件循环 |
| 渲染缓冲 | `github.com/charmbracelet/ultraviolet` (uv) | ScreenBuffer + 矩形布局 |
| 终端样式 | `charm.land/lipgloss/v2` | 声明式样式 |
| Markdown | `charm.land/glamour/v2` | 终端 Markdown 渲染 |
| ANSI 操作 | `github.com/charmbracelet/x/ansi` | 安全字符串切割/宽度 |
| 图片协议 | Kitty Graphics Protocol | 终端内联图片 |
| 测试快照 | `charm.land/catwalk` | Golden File 回归测试 |

## 使用指引

遇到 Go TUI 相关设计问题时，查阅 [完整架构分析](references/REFERENCE.md)，包含以下章节：

| 主题 | 何时阅读 |
|------|---------|
| §1 总体架构 | 理解 Crush 的整体设计模式和数据流 |
| §2 事件循环与输入处理 | 设计键盘/鼠标输入分发、焦点管理 |
| §3 渲染管线 | 实现 Ultraviolet ScreenBuffer 矩形布局和渲染优化 |
| §4 组件/视图系统 | 设计组件接口、Overlay/Dialog 系统 |
| §5 状态管理与数据流 | 设计集中式状态、Common 共享上下文 |
| §6 异步任务与 UI 反馈 | 处理 LLM 流式输出、Spinner/进度、SSE |
| §7 样式与主题 | 使用 lipgloss 构建主题系统 |
| §8 关键技巧与避坑 | 避免常见陷阱 |
| §9 可复用实现蓝图 | 直接参考的代码骨架和实现清单 |
| §10 源码引用索引 | 按文件查找源码位置 |

## Guardrails

- 本 skill 基于 Charmbracelet Crush 源码分析，非通用 Bubble Tea 教程
- 代码示例中的源码路径引用的是 Crush 仓库结构
- 与其他 `tui-expert-of-*` skill 互补：本 skill 专注 Go 栈，其他覆盖 Rust/TypeScript/SolidJS
