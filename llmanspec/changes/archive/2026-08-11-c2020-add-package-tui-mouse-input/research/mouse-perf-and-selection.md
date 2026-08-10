# Research: 鼠标地基与选区 / 性能

> Change: `c2020-add-package-tui-mouse-input`
> 一手依据：本仓代码 + crossterm 事件模型（经包内用法）。非 live specs。

## 代码事实（本仓）

| 点 | 证据 |
|---|---|
| `InputEvent` 仅 Key/Paste | `packages/xylitol-tui/src/tui.rs` `enum InputEvent` |
| demo 环丢弃 Mouse | 同文件 `start_impl`：`Event::Key/Resize/Paste`，其余 `_ => {}` |
| 产品 host 扇入 | `src/app/tui/mod.rs` 等仅映射 Key（及既有 Paste 路径）；无 Mouse |
| Terminal start | `terminal.rs`：raw + bracketed paste + Kitty push；**无** `EnableMouseCapture` |
| 景观缺口 | `docs/research/xylitol-tui-capability-hooks-vs-landscape-2026.md` §5 |
| c1760 预留 | 产品侧 segment↔行映射；「包侧 mouse 另 change」 |

## 产品风险：选区

开启 SGR/X10 mouse reporting 后，多数终端把拖选交给应用，**系统选区复制变难**。竞品策略（景观二手对照，propose 时再核一手文档）：

- OpenCode 等：可关 mouse 保选区
- Gemini：快捷键切换 mouse

**一手补强**（见 [`crossterm-mouse-api.md`](./crossterm-mouse-api.md)）：crossterm **0.29.0** 文档/源码**未**记载选区失效或 Shift 透传保证；`EnableMouseCapture` 固定开 `1003` any-event（含 `Moved`）。

**本草案建议**：包级默认 **不** Enable；产品显式开（或折叠点击需要时开）。选区缓解靠关捕获 / 终端习惯，**不要**把「库保证 Shift 选区」写进合约。禁止 silent 默认抢选区。

## 性能边界

| 风险 | 缓解 |
|---|---|
| Mouse move 洪水 | 默认忽略 move；只处理 Down/Up（或 click 合成） |
| 每事件 `request_render` | 未命中可折叠区 → MUST NOT dirty |
| 与 ath24/ath25 | Tick 闸与 entry paint-cache 不变；Mouse 路径不得绕过 dirty 标志乱刷 |
| 与 c1370/c1505 | 鼠标地基不减行、不切片；不宣称治长历史 CPU |

## 建议验收（日后 propose）

1. Enable/Disable 成对，退出 `finish_inline` 必 Disable。
2. 合成 `InputEvent::Mouse` 进 harness，根组件可 `continue` 忽略。
3. 文档写清选区 tradeoff。

## 开放

- 是否需要 `MouseKind::Scroll` 先映射到现有 PageUp/Down（另案，勿塞进本地基 MVP）。
