# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

## 现状

基于 `xylitol-tui` 的 **host 驱动 UI**（c465 bridge + **c475 layout 壳** + **c476 live scrollback** + **c490 trust gate** + **c480/c481 input** + **c482 abort-resume** + **c485 vertical slice**）。CLI 无参默认 TUI（c474）。

**已开闸（2026-07-11）**：轨 A / 轨 P 已落地；轨 B MVP（至 c485）已归档（2026-07-12）。原子交互仍建议先在 `packages/xylitol-tui` `agent_demo` 验证再进本面。开闸记录 SSOT：`src/AGENTS.md`。

## 优先路径

**闸门**：相关原子/交互 MUST 先在 `packages/xylitol-tui` `agent_demo` 验证，再进本面接线。下一波 draft：`/models`（c630）· 树 filter/fold/fork（c635–c645）· 真 `$EDITOR`（c650）；设计闸 **c625**。**不做** Settings/Plate 运行时改配置；computer-use 延后。

**不做 Codex 式 TranscriptView**（原 c470 草案已移除；`app-tui-transcript` 合约仅约束 live scrollback）。

| 阶段 | 状态 |
|---|---|
| 包 TreeSelector + demo 搜索/filter/fold/label/pan/活树/travel/steer | 已归档（至 c469 / c468）；轨 P 打磨至 c570 已合入 |
| **c460** host 空壳 | 已落地（框架占位） |
| **c491** 产品双 Esc 会话树槽 | **c615**：`Driver::session_tree(MessageHistory)` 活树 + `travel_session_tree` Enter |
| 产品 bridge（XyEvent→UI + Driver 合流） | **c465 已归档** |
| 产品 layout · slash/键位 · DESIGN 视觉 | layout：**c475**；live：**c476**；trust：**c490**；input：**c480/c481**；slice：**c485**；活树 travel：**c615** |
| 垂直切片验收 | **c485 已归档**（`harness.rs` H1–H11 + `tests/tui_e2e` 产品 PTY Fake） |

历史/分支 UX 以会话树为准；live 输出若有，只进 scrollback 行，见 `design/transcript.md` / `design/session-tree.md`。

## 视觉 / UX

**唯一视觉 SSOT**：本目录 **`DESIGN.md`** + **`design/*.md`**。包内不另起 design 文档树；`Palette` 对齐本 DESIGN。

**三层预览**：浏览器 `design/playground/` = **静态设计图**（固定状态）；`just demo-tui` = **动态** playground；本目录 = **生产** host。细则：`design/AGENTS.md`。

包组件只收闭包主题，不承载产品整页 layout。layout 壳（c475）已注入；slash / 键位见 c480。

## Specs

产品面 capability：`app-tui-*`（含 `app-tui-vertical-slice`；另有 `host` / `bridge` / `transcript` / `app-tui-chrome`（**合约 id 不改名**；语义=layout 壳） / `input` / `commands`）。跨切面索引：`app-tui`。合约已归档：`archive/2026-07-10-c450-revise-app-tui-contract`。`app-tui-transcript` 壳仍在，语义为 live scrollback（**非** Codex 浏览面）。

steer / follow-up 键位依赖 **c461**（Agent+Driver 队列 seam）；本面只调 `Driver`，不持有 ReAct 队列。

## Debug 日志

**目标 / 现状（c460）**：debug 构建默认写即时日志；release 默认关。
- 路径：`~/.xylitol/logs/xylitol.log`（`<agent_dir>/logs/xylitol.log`）
- 查看：`tail -f ~/.xylitol/logs/xylitol.log`
- 覆盖：`RUST_LOG=…` 或 `XYLITOL_DEBUG=1`（release 也可用）
- 装配：`app/cli/logging.rs`；埋点 `target: "xylitol::tui"`。禁止 `println!`。

## 硬约束

- **产品面**：已开闸；轨 B 至 **c493** 已归档。**c494** EditorSlot 槽机 + 共享 `effects::drain_pending`。**c615** MessageHistory 活树 + `travel_session_tree`（`effects::drain_pending` 异步泵）。包侧 c575（D08）已归档。
- 渲染/通用组件只用 `xylitol_tui`；禁止在本目录再实现差分引擎或通用 Editor/Markdown。
- **需要底层 TUI 能力时**：先到 `packages/xylitol-tui` 查是否已有或可扩展；缺能力在包内补，再由本面接线。
- 产品路径 **host 驱动**同步引擎；异步事件合流在本面；勿调 `TUI::start()`（demo 专用）。
- 驱动 agent 只经 `app/core/driver::Driver`（含 `steer` / `follow_up` / `clear_queue`）；禁止 reach `agent::session` / `runtime` / `infra`。
- slash 语义复用 `protocol::Command`，经 `app/core/dispatch`；解析收口在 `commands.rs`。
- 组件不直接调 `Driver`、不读写 session；颜色走本面 theme 语义 token（对齐 `DESIGN.md`）。
- 已确认需求须有足够 **harness / 模拟环境** 验证（`HostEvent` 注入 + `TestTerminal`）；难且易错逻辑不外包给低质量实现。包侧五层 / 真终端 E2E 与产品接线测的分工：**唯一 SSOT** → [`packages/xylitol-tui/AGENTS.md`](../../packages/xylitol-tui/AGENTS.md)「验证」。

## HOW（指针）

| 任务 | 去哪 |
|---|---|
| 写/改本面 | `write-tui` skill |
| UX / token / layout | 本目录 `DESIGN.md` + `design/*`；人类 playground 见 `design/AGENTS.md` |
| 底层能力是否已有 / 如何扩展 | `packages/xylitol-tui/AGENTS.md` |
| 新增/改造应用面 | `write-surface` skill |
| 包内组件与五层 / E2E 分工 | [`packages/xylitol-tui/AGENTS.md`](../../packages/xylitol-tui/AGENTS.md)「验证」；how-to → `test-tui-harness` |
| 排查（禁 println） | `tail -f ~/.xylitol/logs/xylitol.log` |

模块：`host.rs`（步进机）、`effects.rs`（唯一 `drain_pending`）、`commands.rs`（slash/bang）、`layout/`（`slots::EditorSlot` + `root` + `LayoutTheme`）、`widgets/`（产品组合件：scrollback / queue strip / glyphs）、`bridge/`（`apply_xy_event` + `handlers/` 事件族）、`terminal_guard.rs`、`tests.rs` / `harness.rs`（合成切片）、`tests/tui_e2e`（产品 PTY）。原子组件来自 `xylitol_tui`；勿在本面再实现通用 Editor/Markdown。勿用 `shell`/`scene` 命名，以免与 bash/`infra::process::shell` 或泛化「场景」混淆。历史文档里的「chrome」= 本面 layout/widgets（非浏览器）。
