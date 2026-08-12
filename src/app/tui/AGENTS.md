# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎：`packages/xylitol-tui/AGENTS.md`。写法：根 `AGENTS.md`。

写/改本面：先读本节职责，再跟 `write-tui`。刻意差异台账：[`PI_DELTAS.md`](./PI_DELTAS.md)（包层另有一份，不得静默覆盖）。

## 角色（按职责，不按文件表）

基于 `xylitol-tui` 的 **host 驱动**产品面（CLI 默认 TUI）。包内通用组件可先在 `agent_demo` 验证引擎行为，再接线本面——**验证引擎 ≠ 产品 UI SSOT**。

| 角色 | 职责 | 禁止 |
|---|---|---|
| host / 入口循环 | 终端扇入、busy/idle 策略、同步步进 | 把业务堆成 God 协调者；能下沉的逻辑先下沉 |
| effects | **唯一**异步副作用泵 → `XyDriver` / dispatch | 第二套 slash/steer 执行路径；harness 必须复用本泵 |
| commands | 解析 → pending | 直接执行副作用 |
| bridge | `XyEvent` → UI 模型 | layout 直接 match 事件 |
| layout / widgets | 呈现与局部交互 | 直接调 Driver / 读写 session |
| harness / tests | 合成切片护栏 | 与生产硬顶混用（测试另计） |

视觉 / UX SSOT：`DESIGN.md` + `design/`（含 **产品专用** `design/playground/` 静图）。历史文案「chrome」= layout/widgets；勿用 `shell`/`scene` 命名。

**信息面词汇（固定）**：讨论与**本面**文档 / host MUST 使用 [`docs/architecture/TUI信息面与chrome词汇.md`](../../../docs/architecture/TUI信息面与chrome词汇.md) 表内词——尤其 **下轮预告**（next-turn cue，≠ message）、**滚动提示**（`UiEntry::ScrollNotice` / `push_scroll_notice`）、**壳层通告**（chrome toast / `push_chrome_toast`，status 上方；≠ ScrollNotice / `UiEntry::Error`）、**尾随 / 顶插**。禁止主用「挂账」「Status trail」「system 消息」/`UiEntry::System` 指 UI。

**与 `agent_demo` 分界**：`just demo-tui` / `packages/…/agent_demo` = 包引擎演示，**不是**本面 SSOT，**不是** DESIGN playground。**MUST NOT** 把 demo 屏上英文 / plate 文案当成产品 chrome 真值；**MUST NOT** 为对齐本词表去强改 demo 字符串（除非人类明确要求）。产品视觉改动走本目录 + `just open-design-playground` / 产品测。

**与未来 Web 的公共体验（跨面）**：凡 TUI 与 Web **共有**的能力（会话、改道、折叠/展开类减噪、即时设置等），用户学习模型与动作语义 MUST 同源——理解成本一致；快捷键 / 发现方式 SHOULD 尽量同构（允许 OS 修饰键差异与 Web 额外点击）。**仅**某一面独有的能力才可另起交互。约束板：[`docs/roadmaps/Web与TUI同源.md`](../../../docs/roadmaps/Web与TUI同源.md)；落地心智：[`docs/architecture/库与多客户端.md`](../../../docs/architecture/库与多客户端.md)。改公共交互前先对齐全套面，禁止静默开出「只教 TUI」的第二套故事。当前未兑现切片示例：长历史 activity 折叠（同文 M1b；草案 `c1760`；前置 `c1755` 已归档）。

## 硬约束

- 滚动提示 / 导航瞬时提示：默认 **尾随**（跟底可见、保 paint-cache）。**顶插不是绝对禁令**——顶层原则是高效绘制 + 用户跟底仍能合理看见关键反馈；仅当有明确理由（且接受缓存失效 / 视口外风险）才可顶插，须在 design/提案写清。瞬时确认优先页脚 / 状态条 / **下轮预告** / 槽，不要堆滚动提示。

- 渲染只用 `xylitol_tui`；缺能力先改包再接线。产品路径 **host 驱动**（demo 专用启动 API 勿用于生产面）。
- **鼠标**：产品默认 **ApplicationOwned**（alt-screen）：会话 begin 后开 mouse capture，应用内选区 + dock 排除输入面；**不**读 `XYLITOL_TUI_MOUSE`（该 env 仅库 lab / e2e / Inline demo）。库仍暴露 Inline 构造入口。模式在 host **启动构造**时绑定（`new_product_ui_with_meta_mode` / `TuiRunOptions`，缺省 ApplicationOwned）；**禁止** mid-session 热切 / 再引入 `apply_interaction_mode`。折叠三角列点击（Tool/Diff/Ask/Thinking per-id + Compaction / OutputViewport / Segment）经 `set_transcript_hit_priority` 接线（[`c2040`](../../../llmanspec/changes/archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/proposal.md)；广义剩余 [`c2045`](../../../llmanspec/changes/c2045-add-tui-fold-target-remaining/proposal.md)；库双模式见 [`c2070`](../../../llmanspec/changes/archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md)）。命名：用 `Inline` / `ApplicationOwned`，勿写 `mode_a`/`mode_b`（见 `packages/xylitol-tui/AGENTS.md` §硬约束 8）。
- Agent 只经 `XyDriver`；禁止 reach `agent` / `infra` 内部（同 `src/AGENTS.md`）。
- Trust 在 CLI 闸；本面 `EditorSlot::Choice` 已解冻给内置 `ask`（c1850）。Plate/Settings stub 仍冻结；产品未拍板勿扩活树/活设置。
- Esc（行为规则；实现细节以代码与 stage-QA design 为准）：
  - Idle 空 editor → 双 Esc 开树
  - Busy 无 overlay → abort latch；立刻臂装 Xy 抑制，drain 仍须 `XyDriver::abort`
  - Busy + overlay → 先关槽，不 abort
  - Agent abort 与 Bang abort 文案/抑制策略不同（见代码与 harness）
- 颜色走本面 theme token。已确认需求须有 harness / BDD 护栏。
- steer / follow-up 经 Driver 队列；本面不持有 ReAct 队列。

## 验证与 HOW

| 角色 | 做什么 |
|---|---|
| Agent 必跑 | 相关 harness/tests；核心 BDD；`just fmt` + 相关 clippy |
| Agent 尽量跑 | `just test-tui-e2e-pty`（真终端） |
| 人类 | 最短手测观感；**不**替代 harness |

包侧五层：`packages/xylitol-tui/AGENTS.md` + skill `test-tui-harness`。排障：`xylitol-inspect-runtime-logs`（勿整文件灌 log）。新增应用面：`write-surface`。

live 只进 scrollback；历史/分叉以会话树为准。不做 Codex TranscriptView；不做运行时 Settings/Plate 改配置；**不**绑定 Ctrl+P 打开 Command Plate stub。
