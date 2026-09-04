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
- `XYLITOL_TUI_MOUSE` 仅在 **Inline** demo（及以其为目标的 PTY e2e）生效；**不是**产品 inline 鼠标开关。Inline 与 ApplicationOwned 用双 example 区分，勿再加 env 热切。
- **MUST NOT** 为「对齐产品固定区词汇表」去改写 demo 屏上英文 / plate 文案（除非人类明确要求）；demo 文案 **允许**与产品中文 SSOT（队列条 / 滚动提示 / 命令面板…）不同。
- **MAY** 形态学对照产品（如 rail 默认、中间队列条）；对照 ≠ 同一 SSOT。
- 产品固定区用词：[`docs/architecture/TUI信息面与固定区词汇.md`](../../docs/architecture/TUI信息面与固定区词汇.md) — **只约束产品面文档与 host**，不约束本包 demo 字符串。
- 改产品视觉 / 固定区：走 `src/app/tui/` 产品代码 + `just open-designing` 对照 / 产品测；**不要**默认先改 `agent_demo*` 当「落地」。

分发本库后：代码零依赖主 crate；文档与 `Palette` **继续引用** monorepo 的 app DESIGN 为活 SSOT（嵌入方也可自备 token 注入闭包）。**不要**在本包另起平行 design 文档树。

## 与 pi-tui 的刻意差异（不得回退成「必须完整 port」）

目标：**差分渲染 + 可组合组件 + crossterm 原生输入**；产品壳与视觉在 `src/app/tui/`。上游 bugfix 可择优吸收，**默认不**为对齐而回退 [`PI_DELTAS.md`](PI_DELTAS.md) 的决议。主题对照、不移植清单、Overlay / Image 裁剪都以该台账为 SSOT，勿在本文件再抄一表。

缺能力时：**先在本包补通用能力，再由应用面接线**——不要把准通用实现塞进 `src/app/tui/`。

## 硬约束（不得回退）

1. 产品路径用 `dispatch_event` / `request_render` / `try_render` / `idle_tick`；`TUI::start()` 仅 demo。
2. 异步事件合流在 `src/app/tui/`，不进本 package。
3. `render` → `Vec<String>`（ANSI）；不引入结构化 `StyledLine` 层。
4. 主题用闭包注入；语义 token 映射在应用面（`src/app/tui/DESIGN.md`）。
5. 终端 I/O / 输入：优先 crossterm `Command`；硬切 `InputEvent::{Key,Paste,Mouse}`；不移植 `stdin-buffer` / 平台专属输入 / `writeLogPath`。细则 [`PI_DELTAS.md`](PI_DELTAS.md)。
6. `enable_mouse_capture` / `XYLITOL_TUI_MOUSE`：**包 API + lab/e2e 保留**；默认不 Enable。产品 **Inline** 不得经 env 自动开 capture。**ApplicationOwned**（alt-screen）经 `begin_application_owned_session` 挂 `ApplicationOwnedRuntime`（ScrollView 视口 + 选区 + dock 排除 + OSC52）。Inline 文档 MUST NOT 暗示「开了 mouse = 原生选区 + 应用点选」兼得。
7. 默认隐藏硬件光标；有 `CURSOR_MARKER` 时可相对定位 IME，但不得无条件 `show_cursor`。
8. **命名（代码 SSOT）**：交互模式 **MUST** 用 `InteractionMode::{Inline,ApplicationOwned}` 及对应 `ApplicationOwnedTui` / `ApplicationOwnedRuntime` / `finish_*`。**禁止**引入 `mode_a` / `mode_b` / `ModeA` / `ModeB` 符号。口语旧称仅允许出现在对照笔记时，且 MUST 立刻映射到 Inline / ApplicationOwned。勿加兼容别名。

## ApplicationOwned host checklist

产品 / 自写 host **MUST** 以 `examples/host_loop_application_owned.rs`（`just demo-tui-host-loop`）为样板；**禁止**把 `agent_demo_impl` 私有坐标算术当 SSOT。

构造 `ApplicationOwnedTui`（或 `TUI::with_interaction_mode(..., ApplicationOwned)`）→ `begin_application_owned_session` → host 驱动 `dispatch_event` / `idle_tick` / `try_render`（**勿**产品路径调 `TUI::start()`）→ 每帧 `set_dock_rows` → Editor 用 `editor_screen_origin` 收**绝对** screen 坐标 → `mouse_in_dock` 过滤 dock → 折叠命中 `set_transcript_hit_priority` → 复制走 `take_copy_notice` / Editor OSC52 → 退出 `finish`。挂起用 `with_terminal_suspended`（ApplicationOwned 自动重进 alt+mouse）。

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

- **包 E2E / `agent_demo*`**：引擎 + 通用组件 + 真终端协议；文案以 demo 自身为准。
- **产品 TUI**：Driver / bridge / layout / 键位 → 应用面 harness。层 4 另有隔离 config 的产品 Fake smoke，日常不绑真 API。
- 层 4 全 `#[ignore]`；缺 tmux 用 `just test-tui-e2e-pty`。操作细则：`test-tui-harness` skill。

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
