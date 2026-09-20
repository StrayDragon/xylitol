# src/app/tui/

本端专属边界。分层与 seam：`src/AGENTS.md`。引擎：`packages/xylitol-tui/AGENTS.md`。写法：根 `AGENTS.md`。

写/改本端：先读本节职责，再跟 `l8ng-write-tui`。刻意差异台账：[`PI_DELTAS.md`](./PI_DELTAS.md)（包层另有一份，不得静默覆盖）。

## 角色（按职责，不按文件表）

基于 `xylitol-tui` 的 **host 驱动**产品端（CLI 默认 TUI）。包内通用组件可先在 `agent_demo` 验证引擎行为，再接线本端——**验证引擎 ≠ 产品 UI SSOT**。

| 角色 | 职责 | 禁止 |
|---|---|---|
| host / 入口循环 | 终端扇入、busy/idle 策略、同步步进 | 把业务堆成 God 协调者；能下沉的逻辑先下沉 |
| effects | **唯一**异步副作用 drain 循环 → `XyDriver` / dispatch | 第二套 slash/steer 执行路径；harness 必须复用本循环 |
| commands | 解析 → pending | 直接执行副作用 |
| bridge | `XyEvent` → UI 模型 | layout 直接 match 事件 |
| layout / widgets | 呈现与局部交互 | 直接调 Driver / 读写 session |
| harness / tests | 合成切片护栏 | 与生产复杂度门禁混用（测试另计） |

运行时真值：本目录产品代码。交互设计稿：[`designing/`](../../../designing/)（对照辅助）——阅读顺序、索引、tui-lab SOP 与快捷键约定见 [`designing/AGENTS.md`](../../../designing/AGENTS.md)；**默认忽略** `designing/app/`。改固定态跑 `scripts/check_tui_designing.py`（入 `just qa`）与 `check-tui-tokens`。现行词「固定区」= layout/widgets；勿用 `shell`/`scene` 命名。

**信息呈现词汇（固定）**：讨论与**本端**文档 / host MUST 使用 [`docs/architecture/TUI信息呈现与固定区词汇.md`](../../../docs/architecture/TUI信息呈现与固定区词汇.md) 表内词——尤其 **滚动提示**、**通知条**、**待办栏**、**尾插 / 顶插**；弃用词（挂账 / Status trail / chrome / 壳层 / system 消息等）以该表「弃用」节为准，勿再引入已退役的换模预告。

**与 `agent_demo` 分界**：包引擎演示，不是本端 SSOT、不是 designing。硬边界见 `packages/xylitol-tui/AGENTS.md`。

**跨端公共体验**：与第二产品端（gpui 桌面，尚未开放）共有能力一套学习成本——规则唯一约束板：[`docs/roadmaps/跨端同源.md`](../../../docs/roadmaps/跨端同源.md)；落地心智：[`docs/architecture/库与多客户端.md`](../../../docs/architecture/库与多客户端.md)。端侧附加：改公共交互前先对齐全部端；未开放端不要写成现行 MUST，也不要在本端预埋第二套动作 id。

## 硬约束

| 规则 | 禁止 |
|---|---|
| 滚动提示默认 **尾插**（跟底可见、保 paint-cache）；顶层原则与例外条件见 [`docs/architecture/TUI信息呈现与固定区词汇.md`](../../../docs/architecture/TUI信息呈现与固定区词汇.md)「信息分类 × 落点」；瞬时确认优先页脚 / 状态条 / 槽 | 无理由顶插；瞬时确认堆进滚动区 |
| 渲染只用 `xylitol_tui`；缺能力先改包再接线；产品路径 **host 驱动** | 准通用实现塞进本端；产品调 demo 启动 API（`TUI::start()`） |
| 鼠标默认 **ApplicationOwned**：host 启动构造时绑定（缺省 AO），begin 后开 capture，应用内选区 + dock 排除；折叠命中经 `set_transcript_hit_priority`；模式命名以包 AGENTS 硬约束为准（`Inline` / `ApplicationOwned`） | mid-session 热切；读 `XYLITOL_TUI_MOUSE`（lab env）；把 `XYLITOL_TUI_INLINE=1` 当产品旗标（仅 lab 窥视 Inline 构造） |
| Agent 只经产品信封客户端（端侧）或 Host 内 `XyDriver`（依赖纪律同 `src/AGENTS.md`） | 产品 TUI 默认同进程直握操作器 |
| Trust 在 CLI 门禁（见 `docs/architecture/信任与项目门禁.md`）；本端 `EditorSlot::Choice` 已解冻给内置 `ask` | Plate / Settings stub 冻结期间扩展其功能或抢先落对应 spec |
| 颜色走本端 theme token；已确认需求须先有 harness / BDD 护栏 | 另立 hex；没有护栏就先写实现 |
| UiEntry：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush；`Palette` = [`DESIGN.md`](./DESIGN.md) frontmatter 运行时快照 | 给 user / assistant 加轨或洗底 |
| steer / follow-up 走 `src/AGENTS.md` 的 `XyDriver` 队列边界 | 本端持有 ReAct 内部队列 |

**Esc 行为**（实现细节以代码与 stage-QA design 为准）：

- Idle 空 editor → 双 Esc 开树
- Busy 无 overlay → 置 abort 锁存（latch），Xy 抑制立即生效；drain 仍须调用 `XyDriver::abort`
- Busy + overlay → 先关槽，不 abort
- Agent abort 与 Bang abort 文案 / 抑制策略不同（见代码与 harness）

## 验证与 HOW

| 角色 | 做什么 |
|---|---|
| Agent 必跑 | 相关 harness/tests；核心 BDD；`just fmt` + 相关 clippy |
| Agent 尽量跑 | `just test-tui-e2e-pty`（真终端） |
| 人类 | 最短手测观感；**不**替代 harness |

包侧验证分层（按键→状态 / snapshot / 时序 / 真终端 E2E）：`packages/xylitol-tui/AGENTS.md`「验证」+ skill `test-tui-harness`。排障：`xylitol-inspect-runtime-logs`（勿整文件灌 log）。新增应用端：`l8ng-write-surface`。

应用端无头帧挂载（BDD 直驱真实渲染）：`SceneBuilder`（经 `xylitol::app::tui` 导出；契约 `package-tui-testing`）。

live 只进 scrollback；历史/分叉以会话树为准。不做 Codex TranscriptView；不做运行时 Settings/Plate 改配置；**不**绑定 Ctrl+P 打开 Command Plate stub。
