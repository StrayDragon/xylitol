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
  # Diff line / word backgrounds (Mocha).
  # Edit-in-tool path: prefer word_wash_bg(surface, polarity) mix≈0.32.
  # Fixed *-word-bg tokens: optional standalone unified row-bg path.
  diff-added-bg: "#1e2b22"
  diff-removed-bg: "#2b1e24"
  diff-added-word-bg: "#2d4a35"
  diff-removed-word-bg: "#4a2d35"
  surface: "#1e1e2e"
  # Tool block full-row tints (Mocha base + green/red; subtle, pi-like).
  # pending ≈ surface0; success/error = base blended toward green/red.
  tool-pending-bg: "#313244"
  tool-success-bg: "#24352a"
  tool-error-bg: "#352428"
  user-message-bg: "#313244"
  # Inline `$skill` token in user message (A10); mauve, distinct from accent spinner.
  skill-ref: "#cba6f7"
# Light companion (designing / demo opt-in; product MVP stays `colors` dark).
colors_light:
  on-surface: "#4c4f69"
  muted: "#9ca0b0"
  accent: "#1e66f5"
  user: "#8839ef"
  assistant: "#4c4f69"
  tool: "#9ca0b0"
  error: "#d20f39"
  warning: "#df8e1d"
  success: "#40a02b"
  diff-added: "#40a02b"
  diff-removed: "#d20f39"
  diff-context: "#9ca0b0"
  diff-added-bg: "#dde8dc"
  diff-removed-bg: "#e8dce0"
  diff-added-word-bg: "#c5dbc4"
  diff-removed-word-bg: "#dbc5ca"
  surface: "#eff1f5"
  tool-pending-bg: "#ccd0da"
  tool-success-bg: "#dce8d8"
  tool-error-bg: "#e8dce0"
  user-message-bg: "#ccd0da"
  skill-ref: "#8839ef"
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
    textColor: "{colors.accent}"
  tool-success:
    textColor: "{colors.success}"
  tool-error:
    textColor: "{colors.error}"
  user-message:
    textColor: "{colors.on-surface}"
  skill-ref:
    textColor: "{colors.skill-ref}"
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
    backgroundColor: "{colors.diff-added-bg}"
  diff-removed:
    textColor: "{colors.diff-removed}"
    backgroundColor: "{colors.diff-removed-bg}"
  diff-context:
    textColor: "{colors.diff-context}"
  diff-added-word:
    textColor: "{colors.diff-added}"
    backgroundColor: "{colors.diff-added-word-bg}"
  diff-removed-word:
    textColor: "{colors.diff-removed}"
    backgroundColor: "{colors.diff-removed-word-bg}"
---

# Design System — Xylitol Terminal

> 规范：遵循 `common-design-md-zh`（中文正文 + YAML frontmatter tokens，值一律双引号）。
> **Token SSOT**：本文件 frontmatter。
> 组件级意图：仓库顶层 [`designing/`](../../../designing/)（`tui/modules` 的 `intent.md` + YAML 固定态）。
> 人类预览：`just open-designing`（交互设计稿，非产品真值）。
> Agent：[`designing/AGENTS.md`](../../../designing/AGENTS.md)。包 `packages/xylitol-tui` 只提供引擎与通用组件；语义 token / layout / glyph 配置在本面。运行时以本目录代码为准。

## Token 引用

色板查 `colors` / `spacing` / `components`；组件意图与固定态在仓库顶层 `designing/tui/modules/`。YAML `token: muted` 映射本文件同名色。模块 MUST NOT 另立冲突 hex。

## Overview

**少装饰壳、多内容、可复制。** 跑在用户已有终端模拟器里的 coding-agent 界面，不是仪表盘。

对齐 pi interactive 的体感：当前轮进 scrollback，输入贴底，忙碌时一行 status，底部一行极简 footer；**分支回看 / travel / fork 用双 Esc 会话树**（替换 editor 槽）。**不做** Codex 式独立 transcript 浏览面。装饰、侧栏、常驻 debug、多行快捷键条默认都不要。

情绪：安静、高效、像在普通 REPL 里聊天。用户应能向上翻历史、框选复制，再贴回下一轮提问——**复制友好优先于视觉热闹**。

参考实现锚点：浏览器 [`designing/`](../../../designing/) = **交互设计稿**（固定状态，对照辅助）；生产接线在本目录 `src/app/tui`（运行时真值）。包侧 `agent_demo`（`just demo-tui`）= 引擎交互演示，**允许与产品固定区 / 文案有差异**，**不**充当 designing。

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
| `diff-added` / `diff-removed` / `diff-context` | Diff 行 **fg**（渲染规则见 `diff` 设计模块） |
| `diff-added-bg` / `diff-removed-bg` | Diff 增删行 **整行淡底**（**仅 unified**；铺满行宽；与 tool-*-bg 分离）。**Side-by-side MUST NOT 用行底**（c464 side-by-side 行底禁令） |
| `diff-added-word-bg` / `diff-removed-word-bg` | 独立 unified 行底路径的词级底 token。**Edit 在 rail 块内**改用 `word_wash_bg(surface, polarity)`（surface→红/绿轻量混亮，默认 mix≈**0.32**）；**MUST NOT** 叠 `diff-*-bg` 行底；**勿 reverse** |
| `surface` | 默认底（终端常透明；需要垫底时用） |
| `tool-pending-bg` / `tool-success-bg` / `tool-error-bg` | 历史 wash token（仍保留于 Palette）；**产品默认不**作整行洗底——成败用 accent/success/error **左边轨**（c1830 rail 落点） |
| `user-message-bg` | 历史用户淡底 token；**产品默认不**启用全行淡底（c1830 rail 落点） |
| `skill-ref` | 用户消息内联 `$skill` 高亮（A10；**不是** accent） |

**工具状态背景（吸取 pi）**：成功/失败不要只靠 fg `ok`/`error` 字——用极淡的绿/红 **bg** 铺满工具块行宽（`apply_background_to_line` + 仅重置 `\x1b[49m`），pending 用中性 surface tint。产品侧 **不做 Codex 式 TranscriptView**；历史/分支 UX 优先双 Esc 会话树。

不要为 header / debug / 多角色长标签再扩一套色。选中列表用 **reverse**，不必单独 `selection-bg` 面板底。

包内组件收闭包主题；语义 → SGR 在本目录 theme 层（代码真值；`theme` / `palette` 设计模块做对照）。

## Typography

只有终端等宽 + ANSI 属性：

- **body**：助手 markdown / 用户正文
- **dim**：元数据、footer、工具行
- **bold**：极少用（错误标题、必要强调）
- **reverse**：列表选中（**不要**用于 Diff 词级；词级见 `word_wash_bg` 与 `diff` 设计模块）
- **underline**：可复制 URL 展示时可用

段落间最多一空行。代码块：语法高亮即可，**无边框、无语言标签条、无树线装饰**（`markdown` 设计模块）。标题分级靠色组 + bold（H1 另 underline），**不**用 `#` 前缀；复制友好与 token 权衡见该模块。

Markdown **fg 内联、bg 延后到行宽 padding**（与 pi-tui Markdown 一致），避免背景断在内容末尾。

## Layout

默认栈（对齐 `agent_demo` / 图 2）：

```
loaded_resources  Codex 风启动卡片（见 loaded-resources 设计模块）
content           全宽；当前轮 live 输出 → 引擎 scrollback（非 Codex 式浏览面）
status            0 或 1 行（仅 busy / retry / error）
editor            贴底；上下 muted `─` 边框标出操作区
                  双 Esc 会话树 / 命令面板：替换此槽（showSelector）
footer            1 行 dim（cwd · model · 可选 context%）
```

硬规则：

1. Viewport 贴尾：`previous_viewport_top = max(0, max(height, n) - height)`。
2. **无双栏**；无常驻 Workspace / Plan / Files 侧栏。
3. **不截断历史**冒充滚动；**分支 travel / 回看**走会话树，不走 Codex 式 transcript 浏览器。
4. **无常驻 debug strip**；调试信息走 `/debug`、日志或临时一行，不占 3 行底栏。
5. **无常驻键墙 / debug header**；允许 **Codex 风边框启动卡片**（简约 meta；见 `loaded-resources` 设计模块）——名称 **MUST 全量展示**（换行），**MUST NOT** `...` 截断。会话名/路径仍并进 footer。
6. 会话树 / 命令面板 / 设置：**替换 editor 槽**，不要 blit 到内容顶部。
7. 居中 `show_overlay` **默认不用**；交互优先 editor 槽（树 / Ask / 板）。仅极短确认可选用 overlay。
8. **保留 editor 上下边框**作为操作区边界（图 2）；不要为了「更扁」去掉这层分区提示。

`spacing.*` 单位是 cell / 行。宽屏 Diff 阈值见 `spacing.diff-side-by-side-min-cols`。

## Elevation & Depth

无阴影、无卡片。层次靠：短前缀、dim/bold、reverse、editor 操作区边框，以及工具块的 **tint 背景**（pending/success/error）。不要双线框墙或装饰性 Unicode 表格线。

## Shapes

无圆角。Editor 使用 muted 上下 `─`（`Editor` 组件已有 `border_color`）。`rounded.none = "0"`（TUI 无像素圆角；边框风格为 plain `─`）。

## Components

组件级 MUST 与固定态在 [`designing/tui/modules/`](../../../designing/tui/modules/)——索引见 [`generated/AGENT-INDEX.md`](../../../designing/generated/AGENT-INDEX.md)，覆盖：session-tree · session-resume · models · transcript · activity-fold · expandable · status · toast-notice · layout · editor · loaded-resources · mcp-cue · footer · diff · markdown · errors · compaction · bash · trust-prompt · queue-steer · theme · palette · ask · tool。本目录不另维护组件子文档；子文档 token 表达式 → 本文件（见「Token 引用」）。

产品默认 **`Palette::dark()`**；**MUST NOT** 默认开 theme auto / OSC11。用户可经 **`/theme`** 切换内建色板（`theme` 设计模块）。

**明确不做**：Settings / Plate 槽（配置继续 YAML+JSON Schema，无运行时改配置 UX）；computer-use 扩展；Codex TranscriptView。

## Do's and Don'ts

- Do 像普通终端会话：向上翻、复制、再问。
- Do 分支回看 / travel / fork 走双 Esc 会话树（对齐 pi）。
- Do 默认隐藏硬件光标；一屏一个 accent。
- Do 选择器（含会话树）替换 editor 槽，保证贴底可见。
- Do 忙碌才出 status；idle 让出垂直空间给对话。
- Do 用 editor 边框标出操作区（对齐 agent_demo）。
- Do glyph 走应用配置，不探测字体。
- Do thinking/tool 可展开（策略见 expandable；thinking flush，tool 用左边轨）。
- Do 工具块用 **status 左边轨**（accent/success/error，经 `paint_left_rail_line`）表达 pending/success/error；**MUST NOT** 默认整行 `tool-*-bg` 洗底。
- Do YAML frontmatter 中所有 token 值使用双引号（`common-design-md-zh`）。
- Do 设计模块用 `{colors.*}` 引用本文件，并声明 `tokens_from`。
- Don't 做 Codex 式独立 transcript 浏览面 / 专用 TranscriptView 主 UX。
- Don't 常驻 Plan / Tools / Files / 快捷键墙。
- Don't 双栏、卡片、圆角、多字体、阴影。
- Don't blit 弹层到内容绝对顶部。
- Don't 截断历史冒充滚动。
- Don't 为「好看」增加无法复制或复制后无意义的装饰字符（含 Markdown 盒线表、代码 fence 墙、标题 `#` 前缀；引用 `│ ` gutter 为例外，见 `markdown` 设计模块）。
- Don't 在设计模块另立冲突色板 hex。
- Don't 在未对齐本 DESIGN 前把 `src/app/tui` 空场景当成产品视觉完成态。
