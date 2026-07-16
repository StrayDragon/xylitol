# xylitol-tui ↔ pi-tui 刻意差异台账

> **目的**：对照 `../pi/packages/tui`（及 kimi-code 同源）做行为/代码整合时，**不得静默覆盖**本文件列出的 xylitol 决议。
> **定位**：`xylitol-tui` 是源自 pi-tui 的 **独立 fork**（合规见 [`NOTICE`](NOTICE)），将随 xylitol 本体大幅迭代；不是持续 1:1 port。
> **不是**进度板；稳定边界见本包 `AGENTS.md`。
> 新增刻意差异时：**先改代码与测试，再在本表加一行**；回退差异须显式评审。

对齐源路径（历史参考）：`../pi/packages/tui`（npm：`@earendil-works/pi-tui`）。

---

## 如何使用（整合 / 对照时）

1. 从 pi 拉行为或补丁前，先扫本表「不得回退」列。
2. 若 pi 变更触及某行主题，默认 **保留 xylitol 侧**；只有产品明确要求才改决议并更新本表。
3. 纯 bugfix（两边语义一致）可对齐 pi，不必记入本表。
4. 应用面（`src/app/tui/`、`agent_demo` 产品壳）**不在** pi-tui 包内——勿把 coding-agent UI 合进本包。
5. 分叉是常态：缺能力优先按 xylitol 产品需求设计，不必先问「pi 怎么做」。

---

## 刻意差异（normative）

| ID | 主题 | pi-tui | xylitol-tui | 不得回退 |
|---|---|---|---|---|
| D01 | 终端 I/O | 自管 VT / raw 较多 | 优先 crossterm `Command`；仅库未暴露序列才 `write_raw` | 是 |
| D02 | stdin | 自研 `stdin-buffer` | **不移植**；crossterm 解码 `Event` | 是 |
| D03 | 输入模型 | VT 字符串 / 自解析为主 | **硬切** `InputEvent::{Key,Paste}`；禁止 KeyEvent→VT→parse 运行时路径 | 是 |
| D04 | 键匹配 | 字符串 `matchesKey` 等 | 运行时 `matches_key_event` / `KeybindingsManager::matches_event` | 是 |
| D05 | 平台专属输入 | `native-modifiers`、Apple/Windows native | **不移植** | 是 |
| D06 | 调试写盘 | `writeLogPath` | **不移植**（tracing / 应用面日志） | 是 |
| D07 | 根类型 | `TUI extends Container` | `TUI` 持有根列表 + 独立 `Container` 组件 | 是 |
| D08 | Overlay focus-restore | eligible/blocked/resume 完整状态机 | **已对齐**（c575）：`FocusTarget` + eligible/blocked/resume；`dispatch_event` reclaim；NC 可显式 `focus`；host 驱动路径 | 是 |
| D09 | 事件循环 | 库内 `start` 常见 | 产品路径 **host 驱动**；`TUI::start()` **仅 demo** | 是 |
| D10 | 硬件光标 | 可开（env） | 默认 **隐藏**；Editor 反色假光标 | 是 |
| D11 | Image | 完整 Kitty/iTerm + `Image` 组件 | **裁剪**；保留 `is_image_line` + `hyperlink` | 是 |
| D12 | 渲染输出 | `string[]` ANSI | 同 `Vec<String>`；**不**引入 `StyledLine` | 是 |
| D13 | Editor 补全扩展 | provider + 引擎内 `/` 等特判较多 | **`CompletionSource` 注册表**（`completion.rs`）；`/` `@` `$` 等为可插拔 Source（扩展点已落地；业务语义在应用面） | 是 |
| D14 | paste-burst | 无对等模块（或弱） | `PasteBurst` + `Clock`/`MockClock`（确定性时序） | 是 |
| D15 | 测试分层 | vitest + virtual-terminal | 五层 harness + PTY/tmux E2E（`test-tui-harness`） | 是 |
| D16 | InputListener | VT 字符串回调常见 | **`InputEvent` 原生** `add_input_listener`；无 KeyEvent→VT；v1 仅 `Continue`/`Consumed` | 是 |
| D17 | Diff 组件 | `renderDiff` 函数式 | **`Diff` Component + `render_diff_lines`**；`similar` 在包内；主题闭包；对齐 `design/diff-block.md` | 是 |
| D18 | 代码高亮 | 应用层常见 | **`highlight` optional feature**（syntect+two-face）；默认依赖无 syntect；经 `MarkdownTheme.highlight_code` 注入 | 是 |
| D19 | `requestRender(true)` / suspend | force 用 `previousWidth=-1` → **整屏 clear**；外部编辑器 resume 亦 clear | force 用 `previous_width=0`（首帧哨兵）→ **full path 但不 `2J`**；`with_terminal_suspended` **保留** `previous_lines` 差分、不立刻 paint（inline 保留上方 scrollback） | 是 |

---

## 应用面 / demo 约定（不进 pi 包，整合时勿误删）

| 主题 | 约定 |
|---|---|
| 产品壳 | transcript / slash 语义 / session → `src/app/tui/` 或 `agent_demo`，**不**进本包 |
| `agent_demo` 快捷键 | 应用级：`Ctrl+P/S` 槽替换；`Ctrl+T` thinking；**`Alt+E` tools**（避 `Ctrl+E`=cursorLineEnd）；**`Alt+G` glyphs**（避 `Ctrl+G`=外部编辑器）；`Ctrl+O` tools viewport；**Ctrl+C** 清编辑器/空则退；**Esc** 流中 abort（经 InputListener）；UI 旁注用 `(Ctrl+T)` 括号完整和弦 |
| 外部 `$EDITOR` | 包只提供 `TUI::with_terminal_suspended`（stop/start/refresh_size + soft `request_render`；**保留** `previous_lines` 差分、**不**立刻 `do_render`、**不**整屏 `2J`——对齐 inline）；spawn/`$VISUAL`/`$EDITOR`/tempfile 在 demo 或 `src/app/tui`，**不**进本包 |
| 原型优先 | 真实 `src/app/tui` 所需 UX/UI 交互，优先在 `agent_demo` 验证后再接线产品面 |
| 工具 bg 三态 | 产品 theme：`tool-pending-bg` / `tool-success-bg` / `tool-error-bg`（`DESIGN.md`）；对齐 pi coding-agent，**不**进 pi-tui 包 |
| Diff 行号 | unified 双 gutter + EditText 紧凑 `±N` + SBS 左右行号（c459） |

---

## 变更记录（短）

| 日期 | 变更 |
|---|---|
| 2026-07-09 | 建表；纳入 D01–D15；记录 `CompletionSource`（D13）与 demo Alt+E/G 键位 |
| 2026-07-10 | D16 InputListener（c455）；demo Ctrl+C/Esc 经 listener |
| 2026-07-10 | demo Ctrl+G 真 `$EDITOR`（`with_terminal_suspended`）；harness 仍 stub；边界写入 `bash-mode.md` |
| 2026-07-10 | c459 Diff 行号；**c462** tool-bg 三态（已归档，非 draft） |
| 2026-07-11 | 轨 P 合入：Markdown（package c530）· plate（c535）· Diff 边角 · Completion `$` 扩展点 · Expandable · playground sync · Tree 边角 · **ChoicePrompt（c565）** · **Palette/`/theme`（c570）**；D08 仍延后（c575） |
| 2026-07-11 | 定位声明：独立 fork（非持续 1:1 port）；合规 `NOTICE` |
| 2026-07-12 | **c575**：D08 overlay focus-restore（eligible/blocked/resume + dispatch reclaim）已落地 |
| 2026-07-14 | **D19**：Ctrl+G resume 不整屏 clear；`set_text` 光标默认 End（对齐 pi editor） |
