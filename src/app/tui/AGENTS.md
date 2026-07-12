# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

## 现状

基于 `xylitol-tui` 的 **host 驱动 UI**（c465 bridge + **c475 chrome** + **c476 live scrollback** + **c490 trust gate** + **c480/c481 input** + **c482 abort-resume**）。CLI 无参默认 TUI（c474）。

**已开闸（2026-07-11）**：轨 A / 轨 P 已落地；c465 已归档（2026-07-12）。原子交互仍建议先在 `packages/xylitol-tui` `agent_demo` 验证再进本面。开闸记录 SSOT：`src/AGENTS.md`。

## 优先路径

**闸门**：相关原子/交互 MUST 先在 `packages/xylitol-tui` `agent_demo` 验证，再进本面接线。**禁止**在 c491 stub 上扩活树 / filter / 真 Driver travel。

**不做 Codex 式 TranscriptView**（`c470` 已 `paused`；`app-tui-transcript` 已降级为 live scrollback）。

| 阶段 | 状态 |
|---|---|
| 包 TreeSelector + demo 搜索/filter/fold/label/pan/活树/travel/steer | 已归档（至 c469 / c468）；轨 P 打磨至 c570 已合入 |
| **c460** host 空壳 | 已落地（框架占位） |
| **c491** 产品双 Esc **假树**槽替换 | **stub 冻结**：仅双 Esc 开/Esc 关/Enter `travel → id`；**MUST NOT** 在此 stub 上扩展活树/filter/Driver |
| 产品 bridge（XyEvent→UI + Driver 合流） | **c465 已归档** |
| 产品 chrome · slash/键位 · DESIGN 视觉落地 · 真 session travel | chrome：**c475**；live scrollback：**c476**；trust：**c490**；input：**c480**；真 travel：另 change |

历史/分支 UX 以会话树为准；live 输出若有，只进 scrollback 行，见 `design/transcript.md` / `design/session-tree.md`。

## 视觉 / UX

**唯一视觉 SSOT**：本目录 **`DESIGN.md`** + **`design/*.md`**。包内不另起 design 文档树；`Palette` 对齐本 DESIGN。

**活实验场**：`just demo-tui`（`agent_demo`）= 产品 TUI 快速 playground——形状/交互先在此试，再进本 host。浏览器静图：`design/playground/`（Agent 默认忽略）。

包组件只收闭包主题，不承载产品 layout。chrome（c475）已注入；slash / 键位见 c480。

## Specs

产品面 capability：`app-tui-*`（`app-tui-host` / `bridge` / `transcript` / `chrome` / `input` / `commands`）。跨切面索引：`app-tui`。合约已归档：`archive/2026-07-10-c450-revise-app-tui-contract`。`app-tui-transcript` 壳仍在；**实现上不按 Codex 浏览面推进**。

steer / follow-up 键位依赖 **c461**（Agent+Driver 队列 seam）；本面只调 `Driver`，不持有 ReAct 队列。

## Debug 日志

**目标 / 现状（c460）**：debug 构建默认写即时日志；release 默认关。
- 路径：`~/.xylitol/logs/xylitol.log`（`<agent_dir>/logs/xylitol.log`）
- 查看：`tail -f ~/.xylitol/logs/xylitol.log`
- 覆盖：`RUST_LOG=…` 或 `XYLITOL_DEBUG=1`（release 也可用）
- 装配：`app/cli/logging.rs`；埋点 `target: "xylitol::tui"`。禁止 `println!`。

## 硬约束

- **产品面**：已开闸；下一闸 **c485 垂直切片**（合成 harness + 产品 PTY Fake smoke）。c491 假树保持 stub（见上表）；真 travel 另 change。
- 渲染/通用组件只用 `xylitol_tui`；禁止在本目录再实现差分引擎或通用 Editor/Markdown。
- **需要底层 TUI 能力时**：先到 `packages/xylitol-tui` 查是否已有或可扩展；缺能力在包内补，再由本面接线。
- 产品路径 **host 驱动**同步引擎；异步事件合流在本面；勿调 `TUI::start()`（demo 专用）。
- 驱动 agent 只经 `app/core/driver::Driver`（含 `steer` / `follow_up` / `clear_queue`）；禁止 reach `agent::session` / `runtime` / `infra`。
- slash 语义复用 `protocol::Command`，经 `app/core/dispatch`。
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

模块：`host.rs`（步进机）、`ui_root.rs`（产品根布局）、`terminal_guard.rs`（restore）、`tests.rs`（harness）。勿用 `shell`/`scene` 命名，以免与 bash/`infra::process::shell` 或泛化「场景」混淆。
