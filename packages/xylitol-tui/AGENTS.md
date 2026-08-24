# packages/xylitol-tui

本 package **稳定边界**。仓库规则与 AGENTS 写法：根 `AGENTS.md`。产品面接线：`src/app/tui/AGENTS.md`。产品视觉/UX：`src/app/tui/DESIGN.md`。

`xylitol-tui` 是源自 `@earendil-works/pi-tui`（MIT）的 Rust **引擎 + 通用组件库**，按 xylitol 需求维护的 **独立 fork**（允许大幅分叉，不是持续 1:1 追平上游）。**零引用**主 crate `xylitol`。合规声明：本包 [`NOTICE`](NOTICE)。

## 边界

| 是 | 不是 |
|---|---|
| `TUI` 引擎、`Component`/`Focusable`、通用组件 | agent session、slash、`XyEvent`、业务状态机 |
| 同步库；产品面 host 驱动 | 绑定 tokio / 拥有产品事件循环 |
| `lib.rs` re-export = API 边界 SSOT | 应用层 theme token / 流式业务缓冲 / 产品 layout |

历史对齐源（行为参考，非逐文件镜像、非强制同步）：`../pi/packages/tui`。
**刻意差异台账（整合时防覆盖）**：本包 [`PI_DELTAS.md`](PI_DELTAS.md)。

## 设计与实验场

本仓 **产品视觉色板 MUST 只有一份**：[`src/app/tui/DESIGN.md`](../../src/app/tui/DESIGN.md)。交互设计稿在仓库顶层 [`designing/`](../../designing/)（辅助，不是运行时真值）。

| 角色 | 路径 | 说明 |
|---|---|---|
| **产品视觉 / 设计稿** | app `DESIGN.md` 色板 + 顶层 `designing/` 交互稿 | 色板一份；固定态给人看；运行时以产品代码为准 |
| **包交互演示** | `examples/agent_demo.rs`（Inline，`just demo-tui`）· `examples/agent_demo_alt.rs`（ApplicationOwned / alt-screen，`just demo-tui-alt-screen`）· `examples/host_loop_application_owned.rs`（最小 host，`just demo-tui-host-loop`） | 引擎 / 通用组件试跑；**≠** designing；共享实现 `agent_demo_impl.rs` |
| **运行时便利** | `Palette`（本包） | 对齐 DESIGN 的 Dark/Light 快照；组件仍只收闭包 |

### `agent_demo` 硬边界（防误导）

- **MUST NOT** 把 `agent_demo` / `agent_demo_alt` / `just demo-tui*` 当成产品 TUI 或 designing 交互稿。
- `XYLITOL_TUI_MOUSE` 仅在 **Inline** demo（及以其为目标的 PTY e2e）生效；**不是**产品 inline 鼠标开关；**禁止**用 `XYLITOL_AGENT_DEMO_MODE` 切模式（已拆双 example）。
- **MUST NOT** 为「对齐产品 chrome 词汇表」去改写 demo 屏上英文 / plate 文案（除非人类明确要求）；demo 文案 **允许**与产品中文 SSOT（队列条 / 滚动提示 / 命令面板…）不同。
- **MAY** 形态学对照产品（如 rail 默认、中间队列条）；对照 ≠ 同一 SSOT。
- 产品 chrome 用词：[`docs/architecture/TUI信息面与chrome词汇.md`](../../docs/architecture/TUI信息面与chrome词汇.md) — **只约束产品面文档与 host**，不约束本包 demo 字符串。
- 改产品视觉 / chrome：走 `src/app/tui/` 产品代码 + `just open-designing` 对照 / 产品测；**不要**默认先改 `agent_demo*` 当「落地」。

分发本库后：代码零依赖主 crate；文档与 `Palette` **继续引用** monorepo 的 app DESIGN 为活 SSOT（嵌入方也可自备 token 注入闭包）。**不要**在本包另起平行 design 文档树。

## 与 pi-tui 的刻意差异（不得回退成「必须完整 port」）

目标：**差分渲染 + 可组合组件 + crossterm 原生输入**，并随 xylitol 本体迭代；产品壳与视觉在 `src/app/tui/`。上游 bugfix 可择优吸收，**默认不**为对齐而回退本表决议。

### 底层 / 输入

| 主题 | pi-tui | xylitol-tui |
|---|---|---|
| 终端 I/O | 自管 VT / raw 序列较多 | **优先 crossterm Command**（raw、paste、Kitty push/pop、`Clear`/`SetTitle`/`cursor::*`、同步输出）；仅 OSC 9;4 等库未暴露的才 `write_raw` |
| stdin | 自研 `stdin-buffer` | **不移植**；crossterm 已解码 `Event` |
| 按键模型 | VT 字符串 / 自解析为主 | **硬切 `InputEvent::{Key,Paste,Mouse}`**；Mouse 默认不刷帧；禁止 KeyEvent→VT→parse 运行时路径 |
| 键匹配 | 字符串 `matchesKey` 等 | 运行时 `matches_key_event` / `KeybindingsManager::matches_event`；`matches_key`/`parse_key` 仅测试与配置字符串 |
| 平台专属输入 | `native-modifiers`、Apple/Windows 路径 | **不移植** |
| 调试写盘 | `writeLogPath` | **不移植**（用 tracing / 应用面日志） |

### 引擎 / API 形状

| 主题 | pi-tui | xylitol-tui |
|---|---|---|
| 根类型 | `TUI extends Container` | `TUI` 持有根 `components`；另提供独立 `Container` 组件（组合，非继承） |
| Overlay | `OverlayHandle` + 完整 focus-restore 状态机 | `OverlayHandle` + **eligible/blocked/resume**（c575）；host `dispatch_event` reclaim |
| 事件循环 | 库内 `start` 常见 | 产品路径 **host 驱动** `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` **仅 demo** |
| 硬件光标 | 可开 | 默认 **隐藏**；Editor 用反色假光标（防流式闪烁） |
| 渲染输出 | `string[]` ANSI | 同：`Vec<String>`；**不**引入结构化 `StyledLine` |
| Editor 补全 | provider + 引擎内 `/` 特判较多 | **`CompletionSource` 注册表**；引擎只管 popup；`/` `@` 等为可插拔 Source（见 `completion.rs`） |

### 有意不移植 / 已裁剪的模块

- `stdin-buffer`、`native-modifiers`、Apple/Windows 专属输入、`writeLogPath`
- **`Image` 组件**与 Kitty/iTerm **完整 encode 路径**（c445 裁剪）；保留 `is_image_line`（宽度豁免）与 `hyperlink`
- pi coding-agent **产品壳**（transcript/slash/session UI）→ 在 `src/app/tui/`，不进本包
- 完整 overlay focus-restore 状态机 → **已落地**（c575）；产品壳仍不进本包

缺能力时：**先在本包补通用能力，再由应用面接线**——不要把准通用实现塞进 `src/app/tui/`。

## 硬约束（不得回退）

1. 产品路径用 `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` 仅 demo。
2. 异步事件合流在 `src/app/tui/`，不进本 package。
3. `render` → `Vec<String>`（ANSI）；不引入结构化 `StyledLine` 层。
4. 主题用闭包注入；语义 token 映射在应用面（`src/app/tui/DESIGN.md`）。
5. 终端 I/O / 输入硬切：见上表「底层 / 输入」。
6. `enable_mouse_capture` / `XYLITOL_TUI_MOUSE`：**包 API + lab/e2e 保留**；默认不 Enable。产品 **Inline** 不得经 env 自动开 capture。**ApplicationOwned**（alt-screen）经 `begin_application_owned_session` 挂 `ApplicationOwnedRuntime`（ScrollView 视口 + 选区 + dock 排除 + OSC52），进 alt-buffer + mouse（c2070 / `package-tui-interaction-modes`）。Inline 文档 MUST NOT 暗示「开了 mouse = 原生选区 + 应用点选」兼得。
7. 默认隐藏硬件光标；有 `CURSOR_MARKER` 时可相对定位 IME，但不得无条件 `show_cursor`。
8. **命名（代码 SSOT）**：交互模式与 ApplicationOwned API **MUST** 用自解释标识符——`InteractionMode::{Inline,ApplicationOwned}`、`ApplicationOwnedTui` / `ApplicationOwnedRuntime`、`set_dock_rows` / `dock_rows_hint`、`set_transcript_copy_on_release`、`set_append_session_to_main_scrollback_on_exit`、`finish` / `finish_application_owned` / `finish_inline`。**禁止**在新/改代码里引入 `mode_a` / `mode_b` / `ModeA` / `ModeB` / `*_mode_b_*` 符号（含 pub API、字段、测试函数名）。口语「Mode A/B」仅允许出现在对照旧笔记时，且 MUST 立刻映射到 Inline / ApplicationOwned。概念 ↔ 代码列：`emulator-vs-app-selection-oneof.md` §1（c2070 research，已冷归档（freeze）进 `llmanspec/changes/archive/freezed_changes.7z.archived`）。勿加兼容别名——一步到位改调用点。

## ApplicationOwned host checklist（ptim14）

产品 / 自写 host **MUST** 走此清单；**禁止**把 `agent_demo_impl` 私有坐标算术当 SSOT。样板：`examples/host_loop_application_owned.rs`（`just demo-tui-host-loop`）。

| 步骤 | API |
|---|---|
| 构造 | `ApplicationOwnedTui::new(term)` 或 `TUI::with_interaction_mode(..., ApplicationOwned)` |
| 开会话 | `terminal.start()` → `begin_application_owned_session` / `ApplicationOwnedTui::begin` |
| 环 | host 驱动 `dispatch_event` → `idle_tick` → `try_render` / `render_now`（**勿**产品路径调 `TUI::start()`） |
| dock | 每帧（或 dock 变）`set_dock_rows`；组件可 `dock_rows_hint` |
| Editor 命中 | `editor_screen_origin(term_rows, dock_rows, rows_above_editor)` → `Editor::set_screen_origin` → 传 **绝对** screen `InputEvent::Mouse`（Editor 内减 origin） |
| dock 过滤 | `mouse_in_dock`；按下始于 dock 不启 transcript 选区（引擎已做）；Editor 仅收 dock/拖选中事件 |
| 复制提示 | `take_copy_notice` / `copy_notice_active` → 壳层短提示（勿写 transcript） |
| fold hit（c2040 tui-mouse-click-fold-triangle） | `set_transcript_hit_priority` — Left Down 优先于选区；回调 `true` 则吞按下并清 transcript 选区 |
| Editor OSC52 | `Editor::take_pending_clipboard` → `enqueue_clipboard_sequences` |
| 退出 | `finish`（按模式分发）；或显式 `finish_application_owned` / `finish_inline` |
| 挂起 | `with_terminal_suspended` — ApplicationOwned 自动重进 alt+mouse（ptim11） |

入口类型：`ApplicationOwnedTui`（Deref→`TUI`）。布局纯函数：`editor_screen_origin` / `mouse_in_dock`。

## 验证（本文件 = 人类/agent 验证分工 SSOT）

其它文档（skill / just 注释）**只引用本节**，勿另写平行长文。

| 验什么 | 在哪跑 | 命令 |
|---|---|---|
| 包组件层 1–3（键序列 / snapshot / 时序） | 本包 `tests/` | `just test-tui` |
| 真终端层 4（crossterm / PTY / tmux） | 工作区 `tests/tui_e2e/`：主场景 spawn **`agent_demo`**；另含产品 Fake smoke（`pty_product_*`，隔离 config） | `just test-tui-e2e`（或 `-pty` / `-tmux`） |
| 仓库全量门禁（不含层 4） | 全仓 | `just qa` |
| 全量门禁 + 层 4 | 全仓 | `just qa-e2e` |
| 产品 host / `XyEvent` / slash 接线 | `src/app/tui/tests.rs` 等 | 随产品测；**不**替代上表 |

**分工（勿混）**

- **包 E2E / `agent_demo*`**：引擎 + 通用组件 + 真终端协议；就绪探针 `DEMO_READY_NEEDLE`（`tests/tui_e2e.rs`，footer `theme:dark`——勿用易滚出视口的标题行）。PTY 上 plate/settings 宜用 `XYLITOL_AGENT_DEMO_INITIAL_PROMPT` + 足够行高；tmux 用 `C-p` / `C-s`。**文案 / chrome 标签以 demo 自身为准**，勿按产品词表强改。ApplicationOwned：example `agent_demo_alt` / `just demo-tui-alt-screen`；PTY 最小闸见 `pty_agent_demo_alt_*`。
- **产品 TUI**：Driver / bridge / layout / 键位 → 应用面 harness（`src/app/tui`）。层 4 另有 **`pty_product_*` Fake smoke**（隔离 HOME/config，不绑真 LLM）；日常仍勿把全量门禁默认绑完整配置/真 API。
- 层 4 全 `#[ignore]`；缺 tmux 时用 `just test-tui-e2e-pty`。操作细则：`test-tui-harness` skill（how-to，非第二份边界文）。

## Specs

本包能力 specs 使用 `package-tui-*` 前缀（见根 `AGENTS.md` 与 `llmanspec/AGENTS.md`）。产品面用 `app-tui-*`（不再堆进单体 `app-tui`）。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 改组件 / 引擎 / 扩测试 | `test-tui-harness` skill（落点）；边界见上「验证」 |
| 扩展 Editor 补全触发（`/` `@` `$` `^`…） | `CompletionSource` + `set_completion_sources`（`src/completion.rs`）；勿在 `editor.rs` 硬编码触发符 |
| 对照 / 合并 pi-tui 行为 | 先读 [`PI_DELTAS.md`](PI_DELTAS.md)；不得静默回退表中决议 |
| 改产品 TUI 面 / UX / 视觉 | `l8ng-write-tui` + 产品代码 + `src/app/tui/DESIGN.md` + `just open-designing`；**勿**默认改 `agent_demo` 当产品落地 |
| 改包引擎 / 通用组件 / demo | `test-tui-harness`；`just demo-tui` 仅验证包能力 |
| 改色板 | 只改 app `DESIGN.md` → `just sync-tui-tokens` → 对齐 `Palette`；`just check-tui-tokens` |
| 打开交互设计稿 | `just open-designing`（对照稿；≠ 产品真值；≠ demo） |
| 日常 / 真终端闸 | 上「验证」表 |

裁剪与待补 API 随接线演进，以代码与 `PI_DELTAS.md` 为准，不在本文件维护进度清单。
