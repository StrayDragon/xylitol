---
depends_on:
- c2071-update-app-tui-host-mode-b-only
blocks: []
---

# Append-only 瘦面：sub-agent 消费 + 多 app/TUI 架构验证

> **一句话**：在主线 ApplicationOwned 产品 TUI 之外，增加一条**几乎无折叠 / 极简键位 / 更矮块限 + session 旁 spill** 的 append-only 应用面，优先服务 sub-agent 观察与旁路；并借此验证分层是否真能支撑「多 app 模式、多 TUI 分别消费」的复用与不复用边界。
>
> **阶段**：Designed / pre-start（`proposal` + `design` + `tasks`；**未** Branch binding；**未**改 live specs / 应用代码）。
> **`ready_for_start`**：**true**（详见 `tasks.md`）。
> **不推翻**：[`c2071`](../archive/2026-08-12-c2071-update-app-tui-host-mode-b-only/proposal.md) 主会话 ath30 AO-only；库双入口见 [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)。

## Why

1. **主线变重是刻意的**：AO + 应用选区 +（后续）activity-fold / 点三角，适合主会话。子任务需要可扫、少抢焦点、少认知的旁路——不是 Print 纯 stdout，也不是第二套完整 chrome。
2. **折叠 ≠ InteractionMode**：`Ctrl+O` / `Alt+E` / hard-truncation（`att14`/`att16`）在 transcript 层；「无折叠一味输出」是呈现策略，应与主屏/alt-screen 载体拆开选型。
3. **溢出已有雏形**：工具硬截断的 `full_output_path` + `[Full output: …]` 可升格为该面的**统一 spill 契约**（session 旁唯一临时目录关联），且每块可见上限**严格小于**主线 TUI。
4. **架构探针**：xylitol 分层承诺 `XyDriver` + `XyEvent` 多面同构（Print / TUI / Server）。第二条（甚至更多）TUI/app 模式若能干净接线，是对「优秀复用 / 故意不复用」的活体验证；接不住则暴露 seam 缺口——本 change 同时是产品切片与架构压力测试。

产品方向对齐候补：[`docs/roadmaps/Sub-Agent编排.md`](../../../docs/roadmaps/Sub-Agent编排.md)；运行时接缝：`RuntimePorts::materialize_runtime`（隔离 actor，不共享 history/cancel/turn）。

## What Changes

1. **新产品面 `SurfaceMode::AppendOnly`**
   - 默认**无**折叠/展开交互（无 activity-fold、无块级 Ctrl+O）。
   - 超行 / 超块 → **session 旁唯一 spill 目录**；transcript 经 `full_output_path` / `[Full output: …]` 关联（收紧今日系统 tmp 散落）。
   - 块可见硬帽：**tool/bash/assistant = 3 行**；**write/diff 类 = 5 行**（均 &lt; 主线 5/10 且不可展开）。
   - **极简键位闭集**：Busy abort、退出、原生滚动；无 fold/slash 发现/spill 专用键。

2. **与主线 AO 的关系**
   - 主会话继续 ath30 ApplicationOwned-only（c2071 不变；specs 仅澄清主语=主会话）。
   - 本面是**额外产品入口**，不是复活用户可切换的产品双模式。
   - 物理载体：**库 `InteractionMode::Inline`**（主屏 + emulator-owned 选区）；产品名避免教「Inline 模式」。

3. **架构验证验收（与功能同权）**
   - `app` 再挂一种模式，经同一 `XyDriver`/`XyEvent`；**禁止** fork ReAct / 第二套事件总线。
   - 复用 vs 故意不复用清单见下表与 `design.md`；实现后审查 + harness 钉住。
   - 本票形态：**独立 TTY 入口**；AO 内嵌套面板后置。

4. **非目标**
   - 不实现 Sub-Agent 编排 M1–M4（委托权限 / 合并策略另票）。
   - 不改主线 fold 族（`c1760` / `c2040` / `c2050`）交付节奏。
   - 不把「无折叠」升格为 Web/TUI 公共减噪的第二套故事（面专属；见 `docs/roadmaps/Web与TUI同源.md`）。

## 复用 / 不复用

| 复用（MUST） | 不复用 / 面专属（MUST 分叉） |
|---|---|
| `XyDriver` / `XyEvent` 消费 | 折叠 / activity-fold / 点三角 / Ctrl+O 展开 |
| `composition::build_agent` 装配 | 完整产品键位表与 slash 发现 |
| session store / Trust 边界（不越父） | AO 应用内选区 / mouse capture / dock |
| 工具硬截断 → spill 路径族（升格 session 旁） | 主线 chrome 密度（status/footer/cue/queue 全套） |
| `RuntimePorts` 物化隔离约定（后续接线） | 主线块默认 5/10 行与可展开 |
| 共享 tool 展示 helper（语义仍通时） | 主线 `HostSession` 整树复制 |

## Capabilities

- **新**：`app-tui-append-only`（入口、Inline 载体、块帽、spill、键位闭集、架构探针）
- **触**：`app-tui-host` ath30 **主语澄清**（主会话 only；禁止 silently 推翻 AO-only）
- 编排侧另票：agent runtime / sub-agent 可见性

## Impact

| 层 | 影响 |
|---|---|
| `src/app` | 新 `SurfaceMode` + 瘦 host；组合根再挂一条面 |
| `packages/xylitol-tui` | 复用 Inline 引擎；**禁止**把主线 AO MUST 偷绑到瘦面 |
| `agent` / `protocol` | 理想零改；spill 路径若必须收 session 旁 → 最小 infra/工具累加器改动 |
| live specs | **start 之后** Specs landing；本阶段不改 |

## Open Questions（已钉）

| # | 问题 | 钉 |
|---|---|---|
| 1 | 载体 Inline vs AO 小窗 | **`InteractionMode::Inline`**；产品称 append-only surface |
| 2 | 独立入口 vs AO 内嵌套 | **独立 `SurfaceMode::AppendOnly`**；嵌套后置 |
| 3 | ath30 写法 | **并列新 capability** + ath30 主语澄清；不造削弱例外 |
| 4 | 最小键位 | abort + 退出 + 原生滚动；无 spill 键、无 fold/slash 发现 |
| 5 | 面 vs 编排 | **只交面 + 架构探针**；M1 另票；harness=`ScriptedDriver` |

细则与数字：`design.md`。任务拆分：`tasks.md`。

## Start readiness

| 项 | 状态 |
|---|---|
| 依赖 c2071 归档 | ✅ |
| design + tasks + OQ | ✅ |
| Branch / Specs / 代码 | 未做（本阶段禁止） |
| **ready_for_start** | **true** → 下一步 `llman sdd change start c2080-add-append-only-subagent-tui` |

## Further Notes

- 探索结论：折叠与 InteractionMode 正交；优先 **O2 瘦面** 而非复活产品双模式；架构验证与产品切片同权。
- 外部对照调研：[Pi/Cursor sub-agent UI](777d78bf-c386-4170-bff2-86bb30ab205b) → [`research/pi-cursor-subagent-ui.md`](./research/pi-cursor-subagent-ui.md)。摘要：
  - **Pi**：extension 另起 `pi --mode json` 子进程；流式刷父工具行；**默认 collapsed** + 全局 `Ctrl+O`；parallel 50KB/任务封顶；无专属子键位。
  - **Cursor（官方一手）**：父默认只见最终 summary；背景进度可写 `~/.cursor/subagents/`；多 agent 走 Agents Window；停父必停子。
  - **人拍（Designed）**：取 **真·无折叠瘦面 + 独立入口 + session spill**（非 Pi 默认折叠摘要；非本票做 Agents Window 嵌套）。
- 相关归档调研：[`c2070/research/keep-or-drop-inline-mode.md`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/research/keep-or-drop-inline-mode.md)。
