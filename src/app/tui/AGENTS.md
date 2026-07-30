# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎：`packages/xylitol-tui/AGENTS.md`。写法：根 `AGENTS.md`。

写/改本面：先读本节职责，再跟 `write-tui`。刻意差异台账：[`PI_DELTAS.md`](./PI_DELTAS.md)（包层另有一份，不得静默覆盖）。

## 角色（按职责，不按文件表）

基于 `xylitol-tui` 的 **host 驱动**产品面（CLI 默认 TUI）。原子交互优先在包内 demo 验证后再接线。

| 角色 | 职责 | 禁止 |
|---|---|---|
| host / 入口循环 | 终端扇入、busy/idle 策略、同步步进 | 把业务堆成 God 协调者；能下沉的逻辑先下沉 |
| effects | **唯一**异步副作用泵 → `XyDriver` / dispatch | 第二套 slash/steer 执行路径；harness 必须复用本泵 |
| commands | 解析 → pending | 直接执行副作用 |
| bridge | `XyEvent` → UI 模型 | layout 直接 match 事件 |
| layout / widgets | 呈现与局部交互 | 直接调 Driver / 读写 session |
| harness / tests | 合成切片护栏 | 与生产硬顶混用（测试另计） |

视觉 / UX SSOT：`DESIGN.md` + `design/`。历史文案「chrome」= layout/widgets；勿用 `shell`/`scene` 命名。

**与未来 Web 的公共体验（跨面）**：凡 TUI 与 Web **共有**的能力（会话、改道、折叠/展开类减噪、即时设置等），用户学习模型与动作语义 MUST 同源——理解成本一致；快捷键 / 发现方式 SHOULD 尽量同构（允许 OS 修饰键差异与 Web 额外点击）。**仅**某一面独有的能力才可另起交互。约束板：[`docs/roadmaps/Web与TUI同源.md`](../../../docs/roadmaps/Web与TUI同源.md)；落地心智：[`docs/architecture/库与多客户端.md`](../../../docs/architecture/库与多客户端.md)。改公共交互前先对齐全套面，禁止静默开出「只教 TUI」的第二套故事。当前未兑现切片示例：长历史 activity 折叠（同文 M1b；草案 `c1760` / `c1755`）。

## 硬约束

- 渲染只用 `xylitol_tui`；缺能力先改包再接线。产品路径 **host 驱动**（demo 专用启动 API 勿用于生产面）。
- Agent 只经 `XyDriver`；禁止 reach `agent` / `infra` 内部（同 `src/AGENTS.md`）。
- Trust 在 CLI 闸；本面 Choice 等 stub **冻结**，产品未拍板勿扩活树/活设置。
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
