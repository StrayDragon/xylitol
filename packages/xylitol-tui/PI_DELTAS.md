# xylitol-tui ↔ pi-tui 刻意差异台账

> **目的**：对照 `../pi/packages/tui`（及 kimi-code 同源）做行为/代码整合时，**不得静默覆盖**本文件列出的 xylitol 决议。
> **不是**进度板；进度见根 `_HANDOFF.md`。稳定边界见本包 `AGENTS.md`。
> 新增刻意差异时：**先改代码与测试，再在本表加一行**；回退差异须显式评审。

对齐源路径：`../pi/packages/tui`（npm：`@earendil-works/pi-tui`）。

---

## 如何使用（整合 / 对照时）

1. 从 pi 拉行为或补丁前，先扫本表「不得回退」列。
2. 若 pi 变更触及某行主题，默认 **保留 xylitol 侧**；只有产品明确要求才改决议并更新本表。
3. 纯 bugfix（两边语义一致）可对齐 pi，不必记入本表。
4. 应用面（`src/app/tui/`、`agent_demo` 产品壳）**不在** pi-tui 包内——勿把 coding-agent UI 合进本包。

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
| D08 | Overlay focus-restore | eligible/blocked/resume 完整状态机 | 最小 hide/focus；完整 restore **延后**（c445 `future.md`） | 是（延后项除外） |
| D09 | 事件循环 | 库内 `start` 常见 | 产品路径 **host 驱动**；`TUI::start()` **仅 demo** | 是 |
| D10 | 硬件光标 | 可开（env） | 默认 **隐藏**；Editor 反色假光标 | 是 |
| D11 | Image | 完整 Kitty/iTerm + `Image` 组件 | **裁剪**；保留 `is_image_line` + `hyperlink` | 是 |
| D12 | 渲染输出 | `string[]` ANSI | 同 `Vec<String>`；**不**引入 `StyledLine` | 是 |
| D13 | Editor 补全扩展 | provider + 引擎内 `/` 等特判较多 | **`CompletionSource` 注册表**（`completion.rs`）；`/` `@` 为可插拔 Source；未来 `$`/`^` 同范式 | 是 |
| D14 | paste-burst | 无对等模块（或弱） | `PasteBurst` + `Clock`/`MockClock`（确定性时序） | 是 |
| D15 | 测试分层 | vitest + virtual-terminal | 五层 harness + PTY/tmux E2E（`test-tui-harness`） | 是 |
| D16 | InputListener | VT 字符串回调常见 | **`InputEvent` 原生** `add_input_listener`；无 KeyEvent→VT；v1 仅 `Continue`/`Consumed` | 是 |
| D17 | Diff 组件 | `renderDiff` 函数式 | **`Diff` Component + `render_diff_lines`**；`similar` 在包内；主题闭包；对齐 `design/diff-block.md` | 是 |

---

## 应用面 / demo 约定（不进 pi 包，整合时勿误删）

| 主题 | 约定 |
|---|---|
| 产品壳 | transcript / slash 语义 / session → `src/app/tui/` 或 `agent_demo`，**不**进本包 |
| `agent_demo` 快捷键 | 应用级：`Ctrl+P/S` 槽替换；`Ctrl+T` thinking；**`Alt+E` tools**（避 `Ctrl+E`=cursorLineEnd）；**`Alt+G` glyphs**（避 `Ctrl+G`=未来外部 editor）；`Ctrl+O` step；**Ctrl+C** 清编辑器/空则退；**Esc** 流中 abort（经 InputListener） |
| 原型优先 | 真实 `src/app/tui` 所需 UX/UI 交互，优先在 `agent_demo` 验证后再接线产品面（见根 `_HANDOFF.md`） |

---

## 变更记录（短）

| 日期 | 变更 |
|---|---|
| 2026-07-09 | 建表；纳入 D01–D15；记录 `CompletionSource`（D13）与 demo Alt+E/G 键位 |
| 2026-07-10 | D16 InputListener（c455）；demo Ctrl+C/Esc 经 listener |
