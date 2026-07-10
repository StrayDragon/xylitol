---
version: "alpha"
name: "Xylitol Terminal"
description: "Minimal terminal design system for the xylitol product TUI — clean, copy-friendly, scrollback-native."
colors:
  on-surface: "#cdd6f4"
  muted: "#6c7086"
  accent: "#89b4fa"
  user: "#cba6f7"
  assistant: "#cdd6f4"
  tool: "#6c7086"
  error: "#f38ba8"
  warning: "#f9e2af"
  success: "#a6e3a1"
  diff-added: "#a6e3a1"
  diff-removed: "#f38ba8"
  diff-context: "#6c7086"
  surface: "#1e1e2e"
  # Tool block full-row tints (Mocha base + green/red; subtle, pi-like).
  # pending ≈ surface0; success/error = base blended toward green/red.
  tool-pending-bg: "#313244"
  tool-success-bg: "#24352a"
  tool-error-bg: "#352428"
  user-message-bg: "#313244"
typography:
  body:
    fontFamily: "terminal-monospace"
    fontSize: "1cell"
    fontWeight: "400"
    lineHeight: "1"
  dim:
    fontFamily: "terminal-monospace"
    fontSize: "1cell"
    fontWeight: "400"
    lineHeight: "1"
spacing:
  xs: "0"
  sm: "1"
  status-rows: "1"
  footer-rows: "1"
  diff-side-by-side-min-cols: "100"
rounded:
  none: "0"
components:
  user-prefix:
    textColor: "{colors.user}"
  assistant-body:
    textColor: "{colors.assistant}"
  tool-line:
    textColor: "{colors.tool}"
  tool-pending:
    backgroundColor: "{colors.tool-pending-bg}"
  tool-success:
    backgroundColor: "{colors.tool-success-bg}"
  tool-error:
    backgroundColor: "{colors.tool-error-bg}"
  user-message:
    backgroundColor: "{colors.user-message-bg}"
    textColor: "{colors.on-surface}"
  status-line:
    textColor: "{colors.muted}"
    height: "{spacing.status-rows}"
  footer:
    textColor: "{colors.muted}"
    height: "{spacing.footer-rows}"
  editor-border:
    textColor: "{colors.muted}"
  operation-zone:
    border: "{components.editor-border}"
  diff-added:
    textColor: "{colors.diff-added}"
  diff-removed:
    textColor: "{colors.diff-removed}"
  diff-context:
    textColor: "{colors.diff-context}"
---

# Design System — Xylitol Terminal

> 规范：遵循 `common-design-md-zh`（中文正文 + YAML frontmatter tokens，值一律双引号）。
> **Token SSOT**：本文件 frontmatter。子文档见下方「Token 引用」。
> 组件级 MUST：[`design/`](./design/)（各文件 YAML 只引用本文件，不另立色板）。
> 包 `packages/xylitol-tui` 只提供引擎与通用组件；语义 token / layout / glyph 配置在本面。

## Token 引用（子文档）

`design/*.md` 的 YAML 中形如 `{colors.diff-context}`、`{spacing.status-rows}`、`{components.footer}`、`{typography.body}`、`{rounded.none}` 的表达式：

1. **根路径**：一律解析到 **本文件** `src/app/tui/DESIGN.md` 的 frontmatter 同名路径。
2. 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`，便于 agent / 人一眼找到根源。
3. 子文档 MUST NOT 重新定义与主色板冲突的 hex；组件级只写引用或本组件独有的非色板属性（如 `height`）。
4. 查色 / 查间距：先打开本文件 `colors` / `spacing` / `components`，再打开子文档看该组件用了哪些引用。

## Overview

**少 chrome、多内容、可复制。** 跑在用户已有终端模拟器里的 coding-agent 界面，不是仪表盘。

对齐 pi interactive 的体感：对话进 scrollback，输入贴底，忙碌时一行 status，底部一行极简 footer。装饰、侧栏、常驻 debug、多行快捷键条默认都不要。

情绪：安静、高效、像在普通 REPL 里聊天。用户应能向上翻历史、框选复制，再贴回下一轮提问——**复制友好优先于视觉热闹**。

参考实现锚点：`packages/xylitol-tui` 的 `agent_demo`（图 2 布局）≈ 产品 TUI 目标形态。当前 `src/app/tui` 空场景仅为 host 框架占位，**尚未**按本 DESIGN 实现产品视觉。

## Colors

色板刻意短。默认偏 Catppuccin Mocha，但产品面只映射这些语义：

| Token | 用途 |
|---|---|
| `on-surface` | 助手正文、默认文本 |
| `muted` | status、footer、工具摘要、边框 |
| `accent` | 忙碌 spinner、当前焦点边框（一屏最多一处） |
| `user` | 用户消息前缀（短 glyph） |
| `tool` | 工具一行摘要（dim） |
| `error` / `warning` / `success` | 异常与结果，少用（**前景**） |
| `diff-added` / `diff-removed` / `diff-context` | Diff 行着色（见 [`design/diff-block.md`](./design/diff-block.md)） |
| `surface` | 默认底（终端常透明；需要垫底时用） |
| `tool-pending-bg` / `tool-success-bg` / `tool-error-bg` | 工具块**全行背景**三态（Mocha tint：`#313244` / `#24352a` / `#352428`；对齐 pi 语义，色值本文件 SSOT） |
| `user-message-bg` | 用户消息可选全行背景（对齐 pi `userMessageBg`） |

**工具状态背景（吸取 pi）**：成功/失败不要只靠 fg `ok`/`error` 字——用极淡的绿/红 **bg** 铺满工具块行宽（`apply_background_to_line` + 仅重置 `\x1b[49m`），pending 用中性 surface tint。**demo 已验证（c462）**；产品 transcript 接线见 c470。

不要为 header / debug / 多角色长标签再扩一套色。选中列表用 **reverse**，不必单独 `selection-bg` 面板底。

包内组件收闭包主题；语义 → SGR 在本目录 theme 层（[`design/theme-tokens.md`](./design/theme-tokens.md)）。

## Typography

只有终端等宽 + ANSI 属性：

- **body**：助手 markdown / 用户正文
- **dim**：元数据、footer、工具行
- **bold**：极少用（错误标题、必要强调）
- **reverse**：列表选中；Diff 行内变更（word-level）
- **underline**：可复制 URL 展示时可用

段落间最多一空行。代码块：语法高亮即可，**无边框、无语言标签条、无树线装饰**（[`design/markdown.md`](./design/markdown.md)）。

Markdown **fg 内联、bg 延后到行宽 padding**（与 pi-tui Markdown 一致），避免背景断在内容末尾。

## Layout

默认栈（对齐 `agent_demo` / 图 2）：

```
transcript     全宽；全量历史 → 引擎滚入 scrollback
status         0 或 1 行（仅 busy / retry / error）
editor         贴底；上下 muted `─` 边框标出操作区
               选择器打开时替换此槽（showSelector）
footer         1 行 dim（cwd · model · 可选 context%）
```

硬规则：

1. Viewport 贴尾：`previous_viewport_top = max(0, max(height, n) - height)`。
2. **无双栏**；无常驻 Workspace / Plan / Files 侧栏。
3. **不截断历史**冒充滚动。
4. **无常驻 debug strip**；调试信息走 `/debug`、日志或临时一行，不占 3 行底栏。
5. **无常驻多行 header**；需要会话名/路径时并进 footer，或 quiet 启动后省略。
6. 命令面板 / 设置：**替换 editor 槽**，不要 blit 到内容顶部。
7. 居中 `show_overlay` 只用于确认框等短交互。
8. **保留 editor 上下边框**作为操作区边界（图 2）；不要为了「更扁」去掉这层分区提示。

`spacing.*` 单位是 cell / 行。宽屏 Diff 阈值见 `spacing.diff-side-by-side-min-cols`。

## Elevation & Depth

无阴影、无卡片。层次靠：短前缀、dim/bold、reverse、editor 操作区边框，以及工具块的 **tint 背景**（pending/success/error）。不要双线框墙或装饰性 Unicode 表格线。

## Shapes

无圆角。Editor 使用 muted 上下 `─`（`Editor` 组件已有 `border_color`）。`rounded.none = "0"`（TUI 无像素圆角；边框风格为 plain `─`）。

## Components

组件级 MUST 见 [`design/`](./design/) 索引表。实现与 demo 引用子文档，勿仅依赖口头约定。子文档 token 表达式 → 本文件（见「Token 引用」）。

| 文档 | 内容 |
|---|---|
| [`design/transcript.md`](./design/transcript.md) | 消息呈现 |
| [`design/expandable.md`](./design/expandable.md) | thinking / tool 可展开 |
| [`design/status.md`](./design/status.md) | busy 一行 |
| [`design/editor.md`](./design/editor.md) | 操作区 |
| [`design/footer.md`](./design/footer.md) | 一行 dim |
| [`design/overlay.md`](./design/overlay.md) | 短确认 |
| [`design/diff-block.md`](./design/diff-block.md) | Diff 渲染 |
| [`design/glyphs.md`](./design/glyphs.md) | unicode / ascii 档 |
| [`design/theme-tokens.md`](./design/theme-tokens.md) | 语义 → SGR |
| [`design/keybindings.md`](./design/keybindings.md) | 已决议键位 |
| [`design/markdown.md`](./design/markdown.md) | 复制友好 markdown |
| [`design/errors.md`](./design/errors.md) | 错误呈现 |

后置草稿：[`session-tree`](./design/session-tree.md) · [`bash-mode`](./design/bash-mode.md) · [`queue-steer`](./design/queue-steer.md) · [`trust-prompt`](./design/trust-prompt.md) · [`compaction-status`](./design/compaction-status.md)

## Do's and Don'ts

- Do 像普通终端会话：向上翻、复制、再问。
- Do 默认隐藏硬件光标；一屏一个 accent。
- Do 选择器替换 editor 槽，保证贴底可见。
- Do 忙碌才出 status；idle 让出垂直空间给对话。
- Do 用 editor 边框标出操作区（对齐 agent_demo）。
- Do glyph 走应用配置，不探测字体。
- Do thinking/tool 可展开（策略见 expandable）。
- Do 工具块用 `tool-*-bg` 表达 pending/success/error（吸取 pi）。
- Do YAML frontmatter 中所有 token 值使用双引号（`common-design-md-zh`）。
- Do 子文档用 `{colors.*}` 引用本文件，并声明 `tokens_from`。
- Don't 常驻 Plan / Tools / Files / 快捷键墙。
- Don't 双栏、卡片、圆角、多字体、阴影。
- Don't blit 弹层到内容绝对顶部。
- Don't 截断历史冒充滚动。
- Don't 为「好看」增加无法复制或复制后无意义的装饰字符。
- Don't 在子文档另立冲突色板 hex。
- Don't 在未对齐本 DESIGN 前把 `src/app/tui` 空场景当成产品视觉完成态。
