# Coding Agent / CLI TUI 设计景观调研（2026）

> **用途**：为 coding agent / LLM CLI 的 TUI（或近 TUI）设计提供外部一手景观参考。
> **范围**：Claude Code、Codex CLI、Aider、Goose、OpenCode、Crush、Amp、Gemini CLI；经典终端参照；Warp/Fig 仅对比注记。
> **非目标**：不对照 xylitol / pi / 本仓库；不含实现计划。
> **本仓后续**：引擎对照 [`xylitol-tui-capability-hooks-vs-landscape-2026.md`](./xylitol-tui-capability-hooks-vs-landscape-2026.md)；产品方向 [`../roadmaps/TUI重制.md`](../roadmaps/TUI重制.md)。

## 1. 一句话结论

赛道已分化为两条复制哲学：**富 TUI（alternate buffer + 应用内选区）** 与 **scrollback-native（终端历史即 transcript）**；三者目标（习惯终端操作、易复制、复制省 token）没有产品能同时满分，主流做法是 **键位/命令提供「语义复制」出口**（`/copy`、`[` 写 scrollback、raw 模式），并把 **元数据沉到 footer/status**，正文尽量像日志流。

## 2. 产品矩阵

| 产品 | 形态 | 布局范式 | 键位习惯族 | 复制友好度证据 | 装饰密度 | 一手来源 |
|------|------|----------|------------|----------------|----------|----------|
| **Claude Code** | TTY；可选 fullscreen（alt buffer） | 底固定输入 + 上 transcript；`/focus` 极简；status 任务清单 | Readline + 可配 Vim；`/` `@` `!`；context 化 keybindings | fullscreen：`[` 写 native scrollback；`c` 复制 raw MD；警告鼠标选区是硬换行渲染 | 中–高（消息背景、可折叠 tool） | [interactive-mode](https://code.claude.com/docs/en/interactive-mode)、[fullscreen](https://code.claude.com/docs/en/fullscreen)、[keybindings](https://code.claude.com/docs/en/keybindings) |
| **Codex CLI** | TTY | 主 transcript + 可配 statusline/footer；composer 分区 | Emacs 默认 + **完整 Vim 子 context**；`tui.keymap` + `/keymap` | `/copy`/`Ctrl+O` 复制 markdown；`/raw` 减格式便于终端选区；OSC 52 | 中 | [CLI reference](https://developers.openai.com/codex/cli/reference)、[tui_keymap.rs](https://github.com/openai/codex/blob/main/codex-rs/config/src/tui_keymap.rs) |
| **Aider** | **近 TTY**（prompt-toolkit 流式输出） | 单栏滚动 transcript + 底 prompt；无面板墙 | 默认 **Emacs**；`--vim`；`/` 命令 | **`/copy`**、**`/copy-context`**；依赖终端选区（无 alt buffer 文档） | **低**（流式文本为主） | [commands](https://aider.chat/docs/usage/commands.html)、[copypaste](https://aider.chat/docs/usage/copypaste.html) |
| **Goose** | TTY 会话 | 线性输出 + 底输入；`/r` 展开 tool | `Ctrl+J` 换行；`Ctrl+R` 历史；slash 为主 | **未找到**专用复制键/命令；靠终端 scrollback | 低–中；**`/t ansi`** 最简高对比 | [CLI commands](https://goose-docs.ai/docs/guides/goose-cli-commands/) |
| **OpenCode** | 全功能 TUI | 可 **sidebar**、command palette、session 导航 | **`ctrl+x` leader** + 大量 `input_*` readline 绑定 | **`messages_copy`** (`<leader>y`)；`mouse:false` 保留终端选区；`/export` | 中–高（可配 diff、thinking） | [TUI](https://opencode.ai/docs/tui/)、[keybinds](https://opencode.ai/docs/keybinds/) |
| **Crush** | Bubble Tea TUI | 单会话流 + 权限条；Charm 风格 | `Enter`/`Ctrl+D` 发送；`Ctrl+M` 模型 | **未找到**复制 API 文档 | **高**（markdown 流、glamour） | [quickstart](https://charmbracelet-crush.mintlify.app/quickstart)、[introduction](https://charmbracelet-crush.mintlify.app/introduction) |
| **Amp** | TTY + 可选 thread sidebar | transcript + **`Ctrl+O` 命令面板**；footer 元数据 | 面板驱动；`Ctrl+J` 换行；可配 `amp.keymap` | `amp.terminal.copyOnSelect`；无 `/copy` 一等公民 | 中 | [Owner's Manual](https://ampcode.com/manual)、[appendix](https://ampcode.com/manual/appendix) |
| **Gemini CLI** | Ink TUI；**`ui.useAlternateBuffer` 默认 false** | footer 可隐藏项；`Tab+Tab` minimal/full | VS Code 式 `keybindings.json`；可选 Vim | **`/copy`**；alt buffer 下 **F9 copy mode**；`Ctrl+S` 切 mouse | 可配 **`compactToolOutput`**、隐藏 banner/context | [keyboard-shortcuts](https://geminicli.com/docs/reference/keyboard-shortcuts/)、[configuration](https://geminicli.com/docs/reference/configuration/)、[commands `/copy`](https://geminicli.com/docs/reference/commands/) |
| **OpenAI `openai` CLI** | 非 agent 对话 TUI | REST 子命令 | N/A | N/A | N/A | [openai-cli README](https://github.com/openai/openai-cli/blob/main/README.md) |
| **ChatGPT 官方 TTY** | — | — | — | **未找到一手证据** | — | — |

**近 TTY / 非纯 TTY 注记**：**Warp / Fig** 为 GPU/块级终端，非 alternate-buffer 语义；agent 若跑其中仍受块选区与嵌入 UI 约束，本文不展开。

## 3. 布局与信息层级（跨产品）

| 主题 | 可区分做法 | 代表 |
|------|------------|------|
| **固定输入 + 上滚 transcript** | 底 prompt 不随输出跳动（fullscreen 明确承诺） | Claude [fullscreen](https://code.claude.com/docs/en/fullscreen) |
| **元数据下沉** | model/context/git/token 放 **footer/statusline**，非每条消息徽章 | Codex `/statusline`；Gemini `ui.footer.items` |
| **工具输出折叠** | 默认一行摘要，点击/`Tab` 展开 | Claude `Ctrl+O` transcript；Goose `/r`；Gemini `compactToolOutput` |
| **侧栏非默认** | 线程/文件树用 chord 切换，非三栏常驻 | Amp `Ctrl+\`；OpenCode `<leader>b` |
| **反卡片墙** | `/focus`、minimal UI、`ansi` 主题、无用户名 | Claude `/focus`；Goose `/t ansi`；OpenCode `username` 可关；Gemini `Tab+Tab` |

## 4. 键位与操作习惯族（跨产品）

| 习惯族 | 共性绑定 | 产品差异 |
|--------|----------|----------|
| **Readline / Emacs** | `Ctrl+A/E/K/U`、`Ctrl+R` 历史 | 全员 composer；OpenCode/Gemini 文档化最全 |
| **Vim** | 可选 `Esc`+`hjkl`，NORMAL 切历史 | Claude `/config`；Codex 独立 `vim_*` context；Aider `--vim`；Gemini `/vim` |
| **Slash 发现** | `/help` `/clear` `/model` | Goose/Aider 重度依赖；Claude 含 skills/MCP |
| **Leader 层** | 避免占 `Ctrl+*` | OpenCode `ctrl+x`；Amp `<leader>` 可配 |
| **提交 vs 换行** | `Enter` 提交；`Shift+Enter`/`Ctrl+J`/`\`+Enter | 均文档化 tmux `extended-keys`（Amp [appendix](https://ampcode.com/manual/appendix)） |
| **外部编辑器** | `Ctrl+G` / `/editor` | Claude、Codex、OpenCode、Gemini、Aider `Ctrl-X Ctrl-E` |
| **Shell 直通** | `!` 或 `$` | Claude `!`；Amp `$`/`$$`；OpenCode `!cmd` |

## 5. 复制与 token 清洁度

**根本张力**：富 TUI 用 **ANSI/硬换行/装饰** 绘图 → 终端拖选常 **污染剪贴板**；alt buffer 又 **切断 scrollback**。

| 策略 | 机制 | 产品 |
|------|------|------|
| **语义复制（推荐）** | 复制 **源 markdown/纯文本**，非屏幕栅格 | Claude `c`；Codex/Amp/Gemini `/copy`；Aider `/copy`；OpenCode `<leader>y` |
| **交还 scrollback** | 把会话 **dump 到 native buffer** | Claude transcript `[`（[fullscreen](https://code.claude.com/docs/en/fullscreen)） |
| **减装饰渲染** | raw/compact 模式 | Codex `/raw`（[reference](https://developers.openai.com/codex/cli/reference)） |
| **鼠标策略** | 应用内 copy-on-select vs `Shift+拖选` vs `mouse:false` | Claude/Amp copy-on-select 可关；OpenCode `mouse`；Gemini F9 copy mode |
| **近 TTY 默认** | 输出进 scrollback，选区=所见（仍可能有 ANSI） | Aider、Goose（默认） |
| **远程剪贴板** | OSC 52 | Codex PR、Gemini `/copy` 文档 |

**bat/less 参照**：`bat --style=plain` / `--decorations=never` 分离 **阅读装饰** 与 **可复制平面**（[bat README](https://github.com/sharkdp/bat/blob/master/README.md)）；less 式搜索被 Claude transcript `/` 借鉴。

### 三项目标对照（摘要）

| 目标 | 做得较好 | 缺口 |
|------|----------|------|
| 不改变终端习惯 | Aider/Goose scrollback；Gemini 默认不用 alt buffer | Claude/Codex fullscreen 需学 `[`、`/raw` |
| 易复制片段 | `/copy` 族；Claude `[`；OpenCode `mouse:false` | 历史消息寻址弱（Codex issue #24073） |
| 复制省 token | 语义复制 markdown | 鼠标拖选仍易带装饰；Crush/Goose 无一等复制 |

## 6. 视觉差异化谱系

```
极简 scrollback ──────────────────────────────► 富 chrome / 卡片感
  Aider · Goose(ansi)    Gemini(minimal)   Claude(focus)   OpenCode   Crush
                              │                  │
                         footer 元数据      消息背景色(fullscreen)
```

**反卡片墙一手证据**：Claude `/focus` 仅保留末条 prompt + tool 摘要 + 回复（[fullscreen](https://code.claude.com/docs/en/fullscreen)）；Goose `ansi`「most visually distinct… brighter colors」而非边框（[goose-cli-commands](https://goose-docs.ai/docs/guides/goose-cli-commands.md)）；Gemini `ui.hideBanner`/`hideContextSummary`/`compactToolOutput`（[configuration](https://geminicli.com/docs/reference/configuration/)）；OpenCode 可隐藏 username、thinking（[TUI](https://opencode.ai/docs/tui/)）。

**偏「终端工具」线索**：前缀化行（`!` bash、`>` 用户输入隐式）、footer 单行状态、折叠 tool 一行摘要、弱角色徽章（多数不把 User/Assistant 做成大标签墙）。

## 7. 经典终端参照的可迁移原则

| 参照 | 原则 | 对 agent TUI |
|------|------|----------------|
| **tmux** | 状态在 status bar，内容区纯 | footer 承载 model/git/token |
| **nvim** | 固定 command 行 + 可滚动 buffer | 底 prompt + 上 transcript |
| **less** | `/` 搜索、`j/k` 导航 | Claude transcript 模式 |
| **bat** | `plain` vs `default` 装饰开关 | `/raw`、`compactToolOutput` |
| **k9s** | TableView + 边框皮肤可关 `logoless` | 内容表格化、chrome 可主题化（[k9s README](https://github.com/derailed/k9s)） |
| **lazygit** | 多 pane 但 `?` 上下文帮助 | 单焦点 + `?`/palette 发现键位（[lazygit README](https://github.com/jesseduffield/lazygit)） |

## 8. 开放问题（通用 TUI 引擎库能力钩子）

未定案，仅能力清单。本仓对照：[`xylitol-tui-capability-hooks-vs-landscape-2026.md`](./xylitol-tui-capability-hooks-vs-landscape-2026.md)；产品方向：[`../roadmaps/TUI重制.md`](../roadmaps/TUI重制.md)。

1. **渲染模式双轨**：scrollback stream vs alternate buffer；运行时切换与状态保持。
2. **语义复制管线**：`copy_as_markdown(turn_id)`、bounded history、rollback 同步（Codex 模式）。
3. **Scrollback 交还**：`dump_to_scrollback(expand_tools)` 一次性写入。
4. **装饰档位**：`raw | compact | rich` 影响绘制与选区。
5. **鼠标/选区策略**：capture、`copyOnSelect`、`shift+select` passthrough、`mouse:false`。
6. **上下文化 keymap**：global/composer/transcript/vim/approval 分层 + leader。
7. **Footer/statusline 插件槽**：有序字段、交互配置持久化。
8. **Tool 呈现**：折叠一行摘要 + 展开；元数据不进正文流。
9. **远程剪贴板**：native + OSC 52 + `/dev/tty` fallback。
10. **外部编辑器 / export**：`$EDITOR` 阻塞编辑、`/export` markdown。

## 参考来源

- Anthropic Claude Code: https://code.claude.com/docs/en/interactive-mode · https://code.claude.com/docs/en/fullscreen · https://code.claude.com/docs/en/keybindings
- OpenAI Codex: https://developers.openai.com/codex/cli/reference · https://github.com/openai/codex/tree/main/codex-rs
- Aider: https://aider.chat/docs/usage/commands.html · https://aider.chat/docs/usage/copypaste.html
- Goose: https://goose-docs.ai/docs/guides/goose-cli-commands/ · https://goose-docs.ai/docs/guides/acp-clients/（实验 TUI 已移除）
- OpenCode: https://opencode.ai/docs/tui/ · https://opencode.ai/docs/keybinds/ · `anomalyco/opencode` `specs/tui-package.md`
- Crush: https://charmbracelet-crush.mintlify.app/quickstart · https://github.com/charmbracelet/crush
- Amp: https://ampcode.com/manual · https://ampcode.com/manual/appendix
- Gemini CLI: https://geminicli.com/docs/reference/keyboard-shortcuts/ · https://geminicli.com/docs/reference/configuration/ · https://geminicli.com/docs/reference/commands/
- bat: https://github.com/sharkdp/bat · k9s: https://github.com/derailed/k9s · lazygit: https://github.com/jesseduffield/lazygit
