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
  # Edit-in-tool path: prefer word_wash_bg(tool-*-bg, polarity) mix≈0.32 (see design/diff-block.md).
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
# Light companion (playground / demo opt-in; product MVP stays `colors` dark).
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
    backgroundColor: "{colors.tool-pending-bg}"
  tool-success:
    backgroundColor: "{colors.tool-success-bg}"
  tool-error:
    backgroundColor: "{colors.tool-error-bg}"
  user-message:
    backgroundColor: "{colors.user-message-bg}"
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
> **Token SSOT**：本文件 frontmatter。子文档见下方「Token 引用」。
> 组件级 MUST：[`design/`](./design/)（各文件 YAML 只引用本文件，不另立色板）。
> 人类快速审色 / 固定状态：[`design/playground/`](./design/playground/)（**静态设计图**；agent 默认忽略，见 [`design/AGENTS.md`](./design/AGENTS.md)）。
> 包 `packages/xylitol-tui` 只提供引擎与通用组件；语义 token / layout / glyph 配置在本面。

## Token 引用（子文档）

`design/*.md` 的 YAML 中形如 `{colors.diff-context}`、`{spacing.status-rows}`、`{components.footer}`、`{typography.body}`、`{rounded.none}` 的表达式：

1. **根路径**：一律解析到 **本文件** `src/app/tui/DESIGN.md` 的 frontmatter 同名路径。
2. 子文档 MUST 声明 `tokens_from: "../DESIGN.md"`，便于 agent / 人一眼找到根源。
3. 子文档 MUST NOT 重新定义与主色板冲突的 hex；组件级只写引用或本组件独有的非色板属性（如 `height`）。
4. 查色 / 查间距：先打开本文件 `colors` / `spacing` / `components`，再打开子文档看该组件用了哪些引用。

## Overview

**少装饰壳、多内容、可复制。** 跑在用户已有终端模拟器里的 coding-agent 界面，不是仪表盘。

对齐 pi interactive 的体感：当前轮进 scrollback，输入贴底，忙碌时一行 status，底部一行极简 footer；**分支回看 / travel / fork 用双 Esc 会话树**（替换 editor 槽）。**不做** Codex 式独立 transcript 浏览面。装饰、侧栏、常驻 debug、多行快捷键条默认都不要。

情绪：安静、高效、像在普通 REPL 里聊天。用户应能向上翻历史、框选复制，再贴回下一轮提问——**复制友好优先于视觉热闹**。

参考实现锚点：`agent_demo`（`just demo-tui`）= **动态** playground；浏览器 [`design/playground/`](./design/playground/) = **静态**设计图（固定状态）；生产接线在本目录 `src/app/tui`。当前主路径 c465–c493 已归档；模块为 `layout/` + `widgets/`。

## Track B 落地切片（设计闸）

| Change | 设计焦点 | 文档 |
|---|---|---|
| **c475** layout 壳 | `Palette::dark` 注入；glyph 档；idle **0** status；busy 一行；footer `cwd · model` | [`status`](./design/status.md) · [`footer`](./design/footer.md) · [`glyphs`](./design/glyphs.md) · [`theme-tokens`](./design/theme-tokens.md) |
| **c476** live scrollback | Markdown / Expandable / Diff / tool-bg；对齐 `agent_demo` 形态（非 Codex 浏览面） | [`markdown`](./design/markdown.md) · [`expandable`](./design/expandable.md) · [`diff-block`](./design/diff-block.md) |
| **c480** input | Editor 操作区；`/exit`；steer / follow-up / Alt+Up dequeue / abort / Ctrl+C；双 Esc → **c615 活树**；队列 strip = pi `Steering:`/`Follow-up:`（非 scrollback 墙）；注入后上行 `UiEntry::User` | [`editor`](./design/editor.md) · [`keybindings`](./design/keybindings.md) · [`queue-steer`](./design/queue-steer.md) · [`session-tree`](./design/session-tree.md) |
| **c615** session tree | `XyDriver::session_tree(MessageHistory)` + `travel_session_tree` Enter；`effects::drain_pending` 异步泵 | [`session-tree`](./design/session-tree.md) |
| **c481** history | 同一 TUI session：idle/steer/follow-up 写入 Editor 发送历史；↑/↓ 召回（包 ed05） | [`editor`](./design/editor.md) · [`keybindings`](./design/keybindings.md) |
| **c490** trust | Ask 时 **ChoicePrompt** 换 editor 槽（禁 stdio 数字菜单） | [`trust-prompt`](./design/trust-prompt.md) |
| **c485** vertical slice | **已归档**：合成 harness H1–H9 + 产品 PTY Fake smoke | archive `2026-07-12-c485-…` |
| **c492** bash | **已归档**：`!`/`!!` 边框 + `execute_bash` → scrollback；Ctrl+G stub | [`bash-mode`](./design/bash-mode.md) · archive `2026-07-12-c492-…` |
| **c493** compaction/retry | **已归档**：Compacting / Retry 单行 status；End 恢复 Working | [`compaction-status`](./design/compaction-status.md) · archive `2026-07-12-c493-…` |

### Next wave（设计闸 · c625+；实现分 change）

| Change | 设计焦点 | 文档 |
|---|---|---|
| **c625** design/playground | 固定下一屏形状：`/model` 列表槽、树 power、真 `$EDITOR`、footer context%、abort 反馈；**不做** Settings/Plate 运行时改配置 | 本表 · [`playground/`](./design/playground/) |
| **c630** `/model` | 替换 editor 槽的 **fuzzy 模型列表**（对齐 pi）；**移除** 无参 cycle | [`models-picker`](./design/models-picker.md) · [`keybindings`](./design/keybindings.md) |
| **c1115** `/theme` | 产品 slash 切内建 `dark`/`light`（无参 Themes 槽；有参/`toggle`；busy 拒绝）；**不**默认开 theme auto；**不**抄 demo Ctrl+P | [`theme-tokens`](./design/theme-tokens.md) · [`keybindings`](./design/keybindings.md) |
| **c635–c645** 树 power | 产品 filter → fold → fork（demo 已有；逐个接线） | [`session-tree`](./design/session-tree.md) · [`keybindings`](./design/keybindings.md) |
| **c650** 真 `$EDITOR` | Ctrl+G：TTY 真编辑器；harness 仍 stub | [`bash-mode`](./design/bash-mode.md) |
| **c1035** footer token usage | 带 provenance 的 `used N`/`~N`/`?`；travel 刷新 | [`footer`](./design/footer.md) |
| **c660–c665** abort 质量 | 工具/bash 进程树取消 + status 反馈（非 computer-use） | [`status`](./design/status.md) · [`errors`](./design/errors.md) |

**明确不做（本波）**：Settings / Plate 槽（配置继续 YAML+JSON Schema，无运行时改配置 UX）；computer-use 扩展；Codex TranscriptView。

产品默认 **`Palette::dark()`**；**MUST NOT** 默认开 theme auto / OSC11。用户可经 **`/theme`** 切换内建色板（**c1115**；见 [`theme-tokens`](./design/theme-tokens.md)）。

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
| `diff-added` / `diff-removed` / `diff-context` | Diff 行 **fg**（见 [`design/diff-block.md`](./design/diff-block.md)） |
| `diff-added-bg` / `diff-removed-bg` | Diff 增删行 **整行淡底**（**仅 unified**；铺满行宽；与 tool-*-bg 分离）。**Side-by-side MUST NOT 用行底**（c464） |
| `diff-added-word-bg` / `diff-removed-word-bg` | 独立 unified 行底路径的词级底 token。**Edit 嵌在 `tool-*-bg` 时**改用 `word_wash_bg(block, polarity)`（块底→红/绿轻量混亮，默认 mix≈**0.32**）；复位到块/行底，**勿 reverse** |
| `surface` | 默认底（终端常透明；需要垫底时用） |
| `tool-pending-bg` / `tool-success-bg` / `tool-error-bg` | 工具块**全行背景**三态（Mocha tint：`#313244` / `#24352a` / `#352428`；对齐 pi 语义，色值本文件 SSOT） |
| `user-message-bg` | 用户消息可选全行背景（对齐 pi `userMessageBg`） |
| `skill-ref` | 用户消息内联 `$skill` 高亮（A10；提交注入见 c1130；**不是** accent） |

**工具状态背景（吸取 pi）**：成功/失败不要只靠 fg `ok`/`error` 字——用极淡的绿/红 **bg** 铺满工具块行宽（`apply_background_to_line` + 仅重置 `\x1b[49m`），pending 用中性 surface tint。**demo 已验证（c462）**。产品侧 **不做 Codex 式 TranscriptView**（原 c470 已移除）；历史/分支 UX 优先双 Esc 会话树（c454→c456→**c615** 活树）。

不要为 header / debug / 多角色长标签再扩一套色。选中列表用 **reverse**，不必单独 `selection-bg` 面板底。

包内组件收闭包主题；语义 → SGR 在本目录 theme 层（[`design/theme-tokens.md`](./design/theme-tokens.md)）。

## Typography

只有终端等宽 + ANSI 属性：

- **body**：助手 markdown / 用户正文
- **dim**：元数据、footer、工具行
- **bold**：极少用（错误标题、必要强调）
- **reverse**：列表选中（**不要**用于 Diff 词级；词级见 `word_wash_bg` / [`design/diff-block.md`](./design/diff-block.md)）
- **underline**：可复制 URL 展示时可用

段落间最多一空行。代码块：语法高亮即可，**无边框、无语言标签条、无树线装饰**（[`design/markdown.md`](./design/markdown.md)）。标题分级靠色组 + bold/underline，**不**用 `#` 前缀；复制友好与 token 权衡见该文档。

Markdown **fg 内联、bg 延后到行宽 padding**（与 pi-tui Markdown 一致），避免背景断在内容末尾。

## Layout

默认栈（对齐 `agent_demo` / 图 2）：

```
loaded_resources  Codex 风启动卡片（见 loaded-resources.md）
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
5. **无常驻键墙 / debug header**；允许 **Codex 风边框启动卡片**（简约 meta；见 [`design/loaded-resources.md`](./design/loaded-resources.md)）——名称 **MUST 全量展示**（换行），**MUST NOT** `...` 截断。会话名/路径仍并进 footer。
6. 会话树 / 命令面板 / 设置：**替换 editor 槽**，不要 blit 到内容顶部。
7. 居中 `show_overlay` **默认不用**；交互优先 editor 槽（树 / Ask / 板）。仅极短确认可选用 overlay（见 [`overlay.md`](./design/overlay.md)）。
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
| [`design/session-tree.md`](./design/session-tree.md) | **优先**：双 Esc 会话树（travel/fork） |
| [`design/models-picker.md`](./design/models-picker.md) | `/model` fuzzy 列表（替换 editor 槽；对齐 pi） |
| [`design/transcript.md`](./design/transcript.md) | live 输出进 scrollback（非 Codex 浏览面） |
| [`design/expandable.md`](./design/expandable.md) | thinking / tool 可展开（demo 优先） |
| [`design/status.md`](./design/status.md) | busy 一行 |
| [`design/editor.md`](./design/editor.md) | 操作区 |
| [`design/skill-ref.md`](./design/skill-ref.md) | `$skill` 用户消息内高亮（A10；demo 先验） |
| [`design/loaded-resources.md`](./design/loaded-resources.md) | 启动品牌 + Skills/MCP 换行清单（c1135） |
| [`design/footer.md`](./design/footer.md) | 一行 dim |
| [`design/overlay.md`](./design/overlay.md) | 默认不用；优先槽内；playground 静图已撤 |
| [`design/diff-block.md`](./design/diff-block.md) | Diff 渲染 |
| [`design/glyphs.md`](./design/glyphs.md) | unicode / ascii 档 |
| [`design/theme-tokens.md`](./design/theme-tokens.md) | 语义 → SGR；Palette；产品 `/theme` slash（c1115）；Ask/ChoicePrompt 见 playground（c565） |
| [`design/keybindings.md`](./design/keybindings.md) | 已决议键位 |
| [`design/markdown.md`](./design/markdown.md) | 复制友好 / token 效率 markdown（`c530-update-package-tui-markdown`） |
| [`design/errors.md`](./design/errors.md) | 错误呈现 |

已落地（c493）：[`compaction-status`](./design/compaction-status.md)。bash（c492）与 trust（c490）已落地。

**已落地 layout 子规范**（c480 起）：[`queue-steer`](./design/queue-steer.md) · [`status`](./design/status.md) — 写产品 host 时以这两份为准，**不要**抄 demo scrollback `[steer]` 墙。

## Do's and Don'ts

- Do 像普通终端会话：向上翻、复制、再问。
- Do 分支回看 / travel / fork 走双 Esc 会话树（对齐 pi）。
- Do 默认隐藏硬件光标；一屏一个 accent。
- Do 选择器（含会话树）替换 editor 槽，保证贴底可见。
- Do 忙碌才出 status；idle 让出垂直空间给对话。
- Do 用 editor 边框标出操作区（对齐 agent_demo）。
- Do glyph 走应用配置，不探测字体。
- Do thinking/tool 可展开（策略见 expandable；demo 优先于产品 Expandable 栈）。
- Do 工具块用 `tool-*-bg` 表达 pending/success/error（吸取 pi）。
- Do YAML frontmatter 中所有 token 值使用双引号（`common-design-md-zh`）。
- Do 子文档用 `{colors.*}` 引用本文件，并声明 `tokens_from`。
- Don't 做 Codex 式独立 transcript 浏览面 / 专用 TranscriptView 主 UX。
- Don't 常驻 Plan / Tools / Files / 快捷键墙。
- Don't 双栏、卡片、圆角、多字体、阴影。
- Don't blit 弹层到内容绝对顶部。
- Don't 截断历史冒充滚动。
- Don't 为「好看」增加无法复制或复制后无意义的装饰字符（含 Markdown 盒线表、代码 fence 墙、标题 `#` 前缀；引用 `│ ` gutter 为例外，见 [`design/markdown.md`](./design/markdown.md)）。
- Don't 在子文档另立冲突色板 hex。
- Don't 在未对齐本 DESIGN 前把 `src/app/tui` 空场景当成产品视觉完成态。
