---
name: tui-expert-of-gemini-cli
description: '当需要用 React/Ink 构建 Node.js 终端 TUI 应用，或参考 Google Gemini CLI 的 TUI 架构设计（声明式渲染、按键系统、流式处理、虚拟滚动、主题系统）时使用。Use when building terminal UIs with React/Ink in Node.js, or referencing Gemini CLI TUI architecture (declarative rendering, keybinding system, streaming, virtualized list, theme system). Keywords: React, Ink, TUI, terminal UI, 终端UI, CLI, gemini, streaming, virtualized list, keybinding, theme'
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
metadata:
  tech_stack: "React + Ink 6.x + TypeScript + Node.js"
  source_project: "google-gemini/gemini-cli v0.45.0"
  rendering_mode: "declarative (React/Ink)"
  input_system: "custom KeypressContext with Kitty Protocol"
---

# TUI Expert of Gemini CLI

基于 Google Gemini CLI (v0.45.0) 源码深度分析的 TUI 架构知识库。技术栈：React + Ink 6.x + TypeScript。

## 适用场景

- 用 React/Ink 构建终端 TUI 应用
- 设计终端应用的按键系统、流式渲染、虚拟滚动
- 参考 Gemini CLI 的状态管理模式（巨型组件 + Context 分离）
- 实现终端主题系统、终端能力检测与降级
- 处理 stdin raw mode、paste mode、resize 等终端底层问题

## 架构要点速查

| 模块 | 核心文件 | 关键模式 |
|------|---------|---------|
| 状态管理 | AppContainer.tsx (~2867行) | useState x50+ → useMemo → UIStateContext/UIActionsContext |
| 按键系统 | KeypressContext.tsx | Generator 协程缓冲 + 4级优先级分发 + Kitty Protocol |
| 流式处理 | useGeminiStream.ts (~2158行) | AsyncIterable + AbortController + tool scheduler |
| 虚拟滚动 | VirtualizedList.tsx (~764行) | ResizeObserver 测量 + StaticRender 双缓冲 |
| 主题系统 | theme.ts + theme-manager.ts | 4种主题类型 + 自动终端背景检测 + hljs 映射 |
| 渲染配置 | interactiveCli.tsx | Alternate buffer + incremental rendering + render process |

## References index

- 完整架构分析文档：`references/tui-expert-of-gemini-cli.md`

## Guardrails

- AppContainer 巨型组件模式适合快速迭代，但超过 3000 行应考虑拆分为独立 store hook
- stdin resume 必须在 Ink render 之前调用，否则 useInput 静默失败
- 流式处理中的 AbortError 需要全局 unhandled rejection handler 兜底
- Static + VirtualizedList 双缓冲渲染是性能关键，不要把所有内容都放进 VirtualizedList

## Done checklist

- stdin raw mode 在进入 Ink 前正确设置，退出时恢复
- 所有流式请求都有 AbortController 支持取消
- 终端 resize 正确传播到组件，布局动态计算
- 主题系统支持终端背景色自动检测和降级
