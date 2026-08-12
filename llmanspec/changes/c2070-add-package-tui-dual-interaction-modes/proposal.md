---
depends_on:
- c2020-add-package-tui-mouse-input
blocks:
- c1505-add-tui-scrollback-viewport-slice
- c1535-optimize-tui-stream-wrap-tail
- c1760-add-tui-activity-fold
- c2040-add-tui-mouse-click-fold-triangle
- c2050-update-activity-fold-mouse-leader
branch: sdd/c2070-add-package-tui-dual-interaction-modes
base_sha: 1dd5de3a53e28365099c2351abd0a767bbfab688
checkpointed: false
---

# xylitol-tui：双交互架构（终端选区 ↔ 应用内选区）

> **升格（2026-08-11）**：自 `delayed-changes/tui/` 整包升入 `llmanspec/changes/`。本 change 是**完整大需求**（库双入口 + Mode B 选区 MUST）；级联后续已拆为独立 change（见 `blocks`）。
> **角色**：`packages/xylitol-tui` 顶层基础——双入口可分治后再谈折叠/点击/viewport。
> **战略修订（2026-08-12，人拍）**：见「战略钉」；与早期「产品默认 A」冲突处以本节为准（live `ath30` **待 Specs landing 改写**）。

> **一句话**：库保留 **两条独立入口**（Inline = 库能力/遗留赋能；ApplicationOwned = alt-screen 产品路径）；**产品 app TUI 仅 Mode B**；Mode B 做成可复用库基础后接 fold cascade。

## 战略钉（2026-08-12）

| 层 | 决定 |
|---|---|
| **产品 `src/app/tui`** | **B-only** — 不维护用户可选 Inline；不承诺产品双模式 UX |
| **库 `xylitol-tui`** | **双入口分治** — Inline 与 ApplicationOwned **分别实现、分别管理**（对齐 Pi MainScreen / AltScreen 方向）；后续功能允许分叉 |
| **Inline 定位** | **库能力 / 历史遗留赋能**（差分引擎、Inline demo/e2e、对照）— **不是**产品一等交互面 |
| **Demo** | **两个 example 文件** + 两个 just recipe；**禁止** `XYLITOL_AGENT_DEMO_MODE` 切模式 |
| **验收** | Mode B 可确定用户行为 → **PTY / tmux 等高自动化**；人验只补手感与终端特例 |
| **禁止** | kill 库 Inline 引擎；Inline 上承诺无修饰点折叠；产品读 `XYLITOL_TUI_MOUSE` 当模式开关 |

依据：[`research/keep-or-drop-inline-mode.md`](./research/keep-or-drop-inline-mode.md)。

## Why

未修饰左键归属是 **oneof**；自然点折叠挂在 alt-screen / 应用内选区一侧。产品只交 B，避免 fold/选区/键位在两套产品故事上分叉；库仍留 Inline 作独立能力与回归对照。

### 与已归档 c2020 的关系（必读）

| | 说明 |
|---|---|
| 保留 | `enable_mouse_capture`、`InputEvent::Mouse`、Moved 不刷帧、teardown Disable |
| **不是**产品开关 | `XYLITOL_TUI_MOUSE` = Inline **lab / e2e**；产品 `TerminalGuard` **不读** |
| 本 change | Mode B 正式 Enable + 应用内选区 MUST；fold 点击在 `blocks` |

## 产品时序

| 阶段 | 做什么 |
|---|---|
| **本 change** | Mode B 库基础；战略收口（拆 demo、review、e2e 矩阵、ptim14 文档）；**propose 改 ath30 → 产品 B-only** |
| **紧随** | 库双入口结构收敛；Mode B PTY/tmux 自动化；产品 host 固定 B |
| **之后** | `blocks` 折叠/点击/性能（自然点折叠 = Mode B only） |

## What Changes

1. **库双入口**：Inline（差分 + 终端 scrollback + 原生选区）与 ApplicationOwned（alt-screen + app 选区/滚动/复制）— 演进为分治实现。
2. **Mode B MUST**：拖选、越界续选、松手复制（默认开）、dock 排除/夹边、Editor 独立多行选区、退出 dump、copy-notice 信号。
3. **产品闸**：app **仅** Mode B（改 `ath30`；删产品双模式设置叙事）。
4. **Demo 拆分**：两文件；去掉 env 切模式。
5. **E2E 矩阵**：Mode B 行为进自动化；见 research。
6. **非本 change 实现**：折叠三角 / L2–L3 / viewport slice → `blocks`。

## Capabilities

- `package-tui-interaction-modes`（双入口 + Mode B 选区·滚动·复制·输入排除）
- `package-tui-terminal-protocol` / `package-tui-engine`（按需增量）
- `app-tui-host`（**产品 B-only** 生命周期与 dock；`ath30` 待改写）

## Impact

| 层 | 影响 |
|---|---|
| `xylitol-tui` | Mode B 子系统 + Inline 遗留入口分治 |
| 产品 TUI | **仅** Mode B；Copied 落点/误触延后讨论 |
| 后续 | `blocks` 全部依赖本 change |

## 依赖图（frontmatter SSOT）

```text
c2020（已归档）
  └─ c2070（本 change）
       ├─ c1760 / c2040 / c2050（折叠·点击）
       └─ c1505 / c1535（长历史性能，软相关）
```

## Out of scope

- kill 库 Inline / 差分引擎
- 在 Inline 承诺无修饰点折叠
- 复活 fold-leader；追平 Pi 全部 chrome
- 实现 `blocks` 内后续 change
- 本回合直接改 live `ath30` 正文（须正式 Specs landing）

## Open Questions（已钉 / 待钉）

1. alt-buffer：**钉** 倾向 `?1049h`；合约「自管视口 MUST + alt-buffer SHOULD」（`ptim02`）。
2. 复制后端：**钉** 默认松手复制开；OSC52 与/或本地。
3. Inline Alt-hold 点折叠：**推迟**；产品不需要（B-only）。
4. 双入口物理拆分（两 type vs 策略对象）形状、e2e MUST 集 → 见进行中 research / review。

## 调研

| 文档 | 内容 |
|---|---|
| [`research/emulator-vs-app-selection-oneof.md`](./research/emulator-vs-app-selection-oneof.md) | 术语与 oneof |
| [`research/native-selection-vs-click-fold.md`](./research/native-selection-vs-click-fold.md) | 方案表 |
| [`research/alt-hold-capture-and-scrollback-hit.md`](./research/alt-hold-capture-and-scrollback-hit.md) | Alt-hold；历史上滚不可点 |
| [`research/pi-starline-click-expand-vs-rust.md`](./research/pi-starline-click-expand-vs-rust.md) | starline / Rust |
| [`research/pi-dual-tui-modes-and-xylitol-cost.md`](./research/pi-dual-tui-modes-and-xylitol-cost.md) | Pi 证据、代价、ratatui |
| [`research/pi-altscreen-selection-scroll-copy-input.md`](./research/pi-altscreen-selection-scroll-copy-input.md) | Pi AltScreen 选区/滚动/复制/输入 |
| [`research/zellij-selection-scroll-copy-input.md`](./research/zellij-selection-scroll-copy-input.md) | Zellij 选区滚动/复制/特例 |
| [`research/xylitol-mode-b-subsystem-cut.md`](./research/xylitol-mode-b-subsystem-cut.md) | xylitol Mode B 子系统切分建议 |
| [`research/keep-or-drop-inline-mode.md`](./research/keep-or-drop-inline-mode.md) | 产品 B-only / 库留 Inline |
| [`research/open-questions-deep-dive-agenda.md`](./research/open-questions-deep-dive-agenda.md) | 深挖题议程（P0–P2） |
| [`research/mode-b-strict-review-2026-08-12.md`](./research/mode-b-strict-review-2026-08-12.md) | 严苛 review（agent） |
| [`research/mode-b-e2e-automation-and-open-questions.md`](./research/mode-b-e2e-automation-and-open-questions.md) | e2e 矩阵（agent） |

## Further Notes

- cascade 五件独立 `changes/<id>/`；依赖 YAML SSOT。
- Mode B 已交付：`ModeBRuntime` + 选区/dock/Editor/dump/copy-notice；产品 host 换栈 API 已有，**默认仍 A（代码）直至 ath30 改写**。
- **人拍（2026-08-12）**：产品 B-only；库双入口分治；demo 两文件；高自动化 e2e。Inline = 库遗留赋能，非产品面。
- **Strict review**（[`research/mode-b-strict-review-2026-08-12.md`](./research/mode-b-strict-review-2026-08-12.md)，[Strict Mode B review](f5555dc2-053f-4444-abc1-f603f6d14fe5)）：行为/单测扎实；**P0** = 单 `TUI` + 散布的 `application_session_active` 挡双入口；ptim14 host 胶（Editor remap、copy-notice 绘制）仍在 `agent_demo` 非可复用入口。**P1** = dump 无可关、Editor 坐标双路径、docs/D16 仍写默认 A。
- **E2E 缺口**（[`research/mode-b-e2e-automation-and-open-questions.md`](./research/mode-b-e2e-automation-and-open-questions.md)，[E2E matrix research](d555305b-bbc7-4cb9-9ac8-37865b11e769)）：层 5 PTY/tmux **几乎零 Mode B**（仅 Inline demo + `XYLITOL_TUI_MOUSE` CSI lab）；H1–H7 真终端未自动化；库 `interaction_modes_test` 已盖进程内。产品 B-only / fold 前 MUST 先补 PTY 探针（alt CSI / OSC52 / dump 子集）。
- **Demo 拆分 + Mode B PTY 最小闸（2026-08-12）**：`agent_demo` / `agent_demo_alt` + shared `agent_demo_impl`；`pty_agent_demo_alt_mode_b_alt_mouse_and_exit_dump` + `pty_agent_demo_alt_mode_b_drag_select_osc52`（`just test-tui-e2e-pty`）。

## Ethics

- Inline 文档不得暗示「开了 mouse = 自然选区 + 直接点」。
- Mode B / 产品 MUST 写清：选区/滚轮归应用；自动化 + 人验覆盖跨页选与松手复制。
