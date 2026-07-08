---
version: alpha
name: Xylitol Terminal
description: Terminal-emulator design system for xylitol-tui and the product TUI surface.
colors:
  surface: "#1e1e2e"
  on-surface: "#cdd6f4"
  muted: "#6c7086"
  primary: "#89b4fa"
  secondary: "#a6adc8"
  accent: "#94e2d5"
  success: "#a6e3a1"
  warning: "#f9e2af"
  error: "#f38ba8"
  user: "#f9e2af"
  assistant: "#a6e3a1"
  tool: "#89b4fa"
  border: "#45475a"
  selection-bg: "#45475a"
  selection-fg: "#cdd6f4"
typography:
  body:
    fontFamily: terminal-monospace
    fontSize: 1cell
    fontWeight: 400
    lineHeight: 1
  label:
    fontFamily: terminal-monospace
    fontSize: 1cell
    fontWeight: 500
    lineHeight: 1
  emphasis:
    fontFamily: terminal-monospace
    fontSize: 1cell
    fontWeight: 700
    lineHeight: 1
spacing:
  xs: 0
  sm: 1
  md: 2
  lg: 4
  gutter: 2
  debug-strip-rows: 3
  status-rows: 1
rounded:
  none: 0
components:
  transcript-user:
    textColor: "{colors.user}"
    typography: "{typography.emphasis}"
  transcript-assistant:
    textColor: "{colors.assistant}"
    typography: "{typography.body}"
  transcript-tool:
    textColor: "{colors.tool}"
    typography: "{typography.label}"
  status-line:
    textColor: "{colors.muted}"
    height: "{spacing.status-rows}"
  editor-border:
    textColor: "{colors.primary}"
  debug-strip:
    textColor: "{colors.muted}"
    height: "{spacing.debug-strip-rows}"
  overlay-panel:
    backgroundColor: "{colors.selection-bg}"
    textColor: "{colors.selection-fg}"
---

# Design System — Xylitol Terminal

## Overview

面向 **终端模拟器** 的 coding-agent TUI：单列、高信息密度、低装饰。视觉语言依赖 ANSI 颜色与属性（bold / dim / reverse / underline），不依赖 Web 字体、圆角卡片或阴影。

情绪：冷静、可滚动、可复制。对话历史进入终端 scrollback；输入与状态永远贴在可视区底部。

默认调色板参考 Catppuccin Mocha 语义映射；产品面可用闭包主题覆盖，token 名保持稳定。

## Colors

- **Surface** (`#1e1e2e`)：逻辑背景（多数终端由模拟器绘制；TUI 不强制清成 alt-screen）。
- **On-surface** (`#cdd6f4`)：主文本。
- **Muted** (`#6c7086`)：状态、footer、debug、分隔线。
- **Primary** (`#89b4fa`)：焦点边框、链接感强调、工具名。
- **User / Assistant / Tool**：角色前缀色，保持可区分且不过饱和。
- **Error / Warning / Success**：校验与结果态。
- **Selection**：列表选中用 reverse 或 `selection-bg`，避免依赖真鼠标选区。

实现：包内组件收 **闭包主题**；语义 token → SGR 的映射在 `src/app/tui/`（产品面）。

## Typography

终端无自定义 fontFamily。层级只用属性：

- **Body**：常规文本、assistant markdown 主体。
- **Emphasis**：bold 角色名、标题。
- **Label**：dim 元数据、debug、快捷键提示。
- **Code**：markdown fence 内 syntect 着色；无边框、无语言标签条。

行高恒为 1 cell。禁止用空行堆「呼吸感」超过结构需要（段落间最多一空行）。

## Layout

单列垂直栈（对齐 pi interactive）：

```
header (可选, ≤2 行)
transcript (全宽, 全量历史 → 引擎滚入 scrollback)
status (固定 1 行)
editor (输入区, 贴底可视)
debug / widgets-below (固定行数, 默认 3)
footer (快捷键, 1 行)
```

硬规则：

1. **Viewport 贴尾**：`previous_viewport_top = max(0, max(height, n) - height)`。
2. **禁止双栏抢 transcript 宽度**（dashboard 侧栏不是默认）。
3. **禁止应用层截断历史冒充滚动**——旧消息必须进入 line-array。
4. **固定底栏高度**：status / debug / footer 行数稳定，避免流式时输入框抖动。
5. Overlay 居中叠在内容上，不改底层栈结构。

`spacing.*` 单位是 **cell / 行**，不是 px。

## Elevation & Depth

无阴影。层次靠：

- 角色色前缀
- dim vs bold
- reverse 选中
- overlay 反色/高对比面板
- 分隔线用 muted `-` 重复到宽（可选；能省则省）

## Shapes

终端无圆角。边框用 ASCII/ANSI 线（Editor 上下边）或纯空行分隔。`rounded.none = 0` 表示刻意不模拟圆角。

## Components

- **Transcript**：全宽消息流；User / Assistant / Tool / System 前缀；markdown 经共享渲染器。
- **StatusLine**：固定 1 行；idle=`Ready`；busy=spinner+短标签；不放 turn 计数/模型名（模型可放 footer 或 slash）。
- **Editor**：多行草稿；反色假光标；默认 **隐藏硬件光标**（IME 仍可相对定位）。
- **DebugStrip**：输入框下固定 N 行摘要（plan / 最近 tool / 文件数）；可 slash 关闭。
- **SelectList / SettingsList**：默认以 **editor-slot 替换**（pi `showSelector`）出现在输入区位置，保证 transcript 滚入 scrollback 后仍在可视底；真正的浮动 `show_overlay` 留给居中确认框等场景。选中 reverse；描述列 dim。
- **Overlay**：居中叠层（确认/扩展）；Esc 关闭；由 `OverlayHandle` 控制。命令面板/设置优先用 editor-slot，不要 blit 到内容绝对顶部。
- **Loader**：仅在 status 忙碌时出现；idle 不得残留 spinner 帧。

## Do's and Don'ts

- Do 保持单列 + scrollback，让终端模拟器原生上下滚看历史。
- Do 用语义色 sparingly：一屏一个主强调色（primary / 当前焦点边框）。
- Do 默认隐藏硬件光标；假光标表达编辑位置。
- Do 固定 status/debug 行数，防止流式布局抖动。
- Don't 用双栏 Workspace 挤占 transcript（demo 已移除；产品默认同）。
- Don't blit 选择弹层到内容绝对顶部（transcript 一长就滚出视口）；命令/设置用 editor-slot 替换。
- Don't 在应用层只渲染「最近 N 条」冒充滚动。
- Don't 引入卡片阴影、圆角、多字体栈等 Web 范式。
- Don't 混用「有时 show 硬件光标、有时 hide」而不经显式设置。
- Don't 在第一屏堆 stats / 多块营销式元数据；终端第一屏 = 对话 + 输入。
