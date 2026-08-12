---
depends_on:
- c2071-update-app-tui-host-mode-b-only
blocks: []
---

# Append-only 瘦面：sub-agent 消费 + 多 app/TUI 架构验证

> **一句话**：在主线 ApplicationOwned 产品 TUI 之外，增加一条**几乎无折叠 / 极简键位 / 更矮块限 + session 旁 spill** 的 append-only 应用面（口语常称 inline），优先服务 sub-agent 观察与旁路；并借此验证分层是否真能支撑「多 app 模式、多 TUI 分别消费」的复用与不复用边界。
>
> **阶段**：purpose-draft（仅本文件）。探索纪要已并入下文；正式化走 `llman-sdd-propose`。
> **不推翻**：[`c2071`](../archive/2026-08-12-c2071-update-app-tui-host-mode-b-only/proposal.md) 主会话 ath30 AO-only；库双入口见 [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)。

## Why

1. **主线变重是刻意的**：AO + 应用选区 +（后续）activity-fold / 点三角，适合主会话。子任务需要可扫、少抢焦点、少认知的旁路——不是 Print 纯 stdout，也不是第二套完整 chrome。
2. **折叠 ≠ InteractionMode**：`Ctrl+O` / `Alt+E` / hard-truncation（`att14`/`att16`）在 transcript 层；「无折叠一味输出」是呈现策略，应与主屏/alt-screen 载体拆开选型。
3. **溢出已有雏形**：工具硬截断的 `full_output_path` + `[Full output: …]` 可升格为该面的**统一 spill 契约**（session 旁唯一临时目录关联），且每块展开上限**严格小于**主线 TUI。
4. **架构探针**：xylitol 分层承诺 `XyDriver` + `XyEvent` 多面同构（Print / TUI / Server）。第二条（甚至更多）TUI/app 模式若能干净接线，是对「优秀复用 / 故意不复用」的活体验证；接不住则暴露 seam 缺口——本 change 同时是产品切片与架构压力测试。

产品方向对齐候补：[`docs/roadmaps/Sub-Agent编排.md`](../../../docs/roadmaps/Sub-Agent编排.md)；运行时接缝：`RuntimePorts::materialize_runtime`（隔离 actor，不共享 history/cancel/turn）。

## What Changes（意向；propose 时拆）

1. **新产品面（暂名 append-only / subagent-surface）**
   - 默认**无**折叠/展开交互（无 activity-fold、无块级 Ctrl+O 展开故事）。
   - 超行 / 超块 → 写入 **session 旁唯一临时目录**并在 transcript 关联（扩展/收紧今日 `full_output_path` 语义，而非另起无关联散落文件）。
   - 每块可见上限 **严格小于** 主线 AO TUI（具体数字 propose 时钉）。
   - **极简键位**（几乎无产品快捷键；abort / 退出等最小闭集另钉）。

2. **与主线 AO 的关系**
   - 主会话继续 ath30 ApplicationOwned-only（c2071 不变）。
   - 本面是**额外产品入口 / 子会话呈现**，不是复活「用户可切换的产品双模式 forever」。
   - 是否物理使用库 `InteractionMode::Inline`（主屏 + 原生选区）→ Open Questions；正式名避免与库 Inline 口语绑死，除非决策要求 emulator-owned 选区。

3. **架构验证验收（与功能同权）**
   - 证明 `app` 可再挂一种模式，经同一 `XyDriver`/`XyEvent` 消费；**禁止**为第二面 fork ReAct / 第二套事件总线。
   - 列出**必须复用** vs **故意不复用**清单（见下表草案）；实现后用审查 + 行为测钉住。
   - 多 TUI：主 AO host 与 append-only host 可并存为两条 app 路径（进程内嵌套 vs 独立入口 → Open Questions）。

4. **非目标（草案）**
   - 不实现完整 Sub-Agent 编排 M2–M4（委托权限 / 合并策略另票）。
   - 不改主线 fold 族（`c1760` / `c2040` / `c2050`）交付节奏。
   - 不把「无折叠」升格为 Web/TUI 公共减噪的第二套故事（面专属；见 `docs/roadmaps/Web与TUI同源.md`）。

## 复用 / 不复用（草案矩阵）

| 复用（MUST） | 不复用 / 面专属（MUST 分叉） |
|---|---|
| `XyDriver` / `XyEvent` 消费 | 折叠 / activity-fold / 点三角 |
| session store / Trust 边界（不越父） | 完整产品键位表与 slash 发现 |
| 工具硬截断 → spill 路径族（可收紧上限） | AO 应用内选区 / mouse capture / dock 排除 |
| `RuntimePorts` 物化隔离 runtime（sub-agent） | 主线 chrome 密度（status/footer/cue 全套） |
| 共享 tool 展示 helper（`tool_display` 等）若语义仍通 | 主线块默认 viewport 行数与 Ctrl+O 展开 |

## Capabilities（意向）

- 新或扩：`app-tui-*`（host / transcript 溢出与键位预算；正式名 propose 时定）
- 按需触：`app-tui-host`（ath30 **例外条款**或并列「第二面」req，禁止 silently 推翻 AO-only）
- 编排侧另票：agent runtime / sub-agent 可见性（本草案可先只交「面」）

## Impact

| 层 | 影响 |
|---|---|
| `src/app` | 新模式入口 + 瘦 host；组合根再挂一条面 |
| `packages/xylitol-tui` | 可能复用 Inline 引擎；**禁止**把主线 AO MUST 偷绑到瘦面 |
| `agent` / `protocol` | 理想零改或仅事件投影；若必须改 → 证明现有 seam 不够 |
| live specs | propose 时 Specs landing；本草案不改 |

## Open Questions

1. **载体**：必须 `InteractionMode::Inline`，还是 append-only 呈现即可（AO 小窗也行）？
2. **嵌套形态**：独立 TTY/进程入口 vs 主 AO 内子面板/旁路？
3. **ath30 写法**：并列「第二产品面」req，还是给 sub-agent 路径显式例外？
4. **最小键位闭集**：仅 abort/退出，还是含滚动/打开 spill 路径？
5. **与 Sub-Agent roadmap**：本票是否只交「面 + 架构探针」，编排 M1 另开 change？

## Further Notes

- 探索结论：折叠与 InteractionMode 正交；优先 **O2 瘦面** 而非复活产品双模式；架构验证与产品切片同权。
- 外部对照调研（已完成，[Pi/Cursor sub-agent UI](777d78bf-c386-4170-bff2-86bb30ab205b)）：[`research/pi-cursor-subagent-ui.md`](./research/pi-cursor-subagent-ui.md)。摘要：
  - **Pi**：extension 另起 `pi --mode json` 子进程；流式刷父工具行；**默认 collapsed** + 全局 `Ctrl+O`/`app.tools.expand`；parallel 对模型 50KB/任务封顶，全文在 `details`；无专属子键位。
  - **Cursor（官方一手）**：父默认只见最终 summary；背景进度可写 `~/.cursor/subagents/`；多 agent 走 Agents Window；停父必停子；**未找到**卡片折叠/专用键位的官方 UI 细则。
  - **支持 c2080**：摘要隔离 + 文件 spill + 硬帽 + 极简键（Pi 无子键表）。
  - **张力**：Pi 默认就是「折叠摘要 + 按需展开」；Cursor 用独立 Agents Window——propose 时仍要在「真·无折叠瘦面」vs「摘要条 + 偶发展开 / 独立面板」间人拍（映射 Open Questions 1–2、4）。
- 相关归档调研：[`c2070/research/keep-or-drop-inline-mode.md`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/research/keep-or-drop-inline-mode.md)。
