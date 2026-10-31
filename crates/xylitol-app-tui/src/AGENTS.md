# crates/xylitol-app-tui/

本端专属边界。分层与 seam：`src/AGENTS.md`。引擎：`crates/xylitol-tui/AGENTS.md`。写法：根 `AGENTS.md`。

写/改本端：先读本节职责，再跟 `l8ng-write-tui`。刻意差异台账：[`PI_DELTAS.md`](./PI_DELTAS.md)。

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

运行时真值：本目录产品代码。视觉色板 SSOT：[`DESIGN.md`](./DESIGN.md) frontmatter；改色板后跑 `just qa-check-tui-tokens`（Palette 对照）。现行词「固定区」= layout/widgets；勿用 `shell`/`scene` 命名。

**信息呈现词汇（固定）**：讨论与**本端**文档 / host MUST 使用 [`llmanspec/specs/app-tui-fixed-zone/`](../../../../llmanspec/specs/app-tui-fixed-zone/) 与产品代码中的固定词——尤其 **滚动提示**、**通知条**、**通知栈**、**待办栏**、**ModeStrip**、**尾插 / 顶插**；弃用旧称（挂账 / Status trail / chrome / 壳层 / system 消息等）勿再引入。ModeStrip 仅单行、无交互，用于实验性模式标识；不得做成横幅或详情面板。**通知分类**：通知条 = 立刻要处理的阻塞拒绝/软闸/即时失败；右上角通知栈 = 即时确认（复制成功）；滚动提示 = 耐久导航/操作结果/错误；系统消息 = 必须进入模型上下文的耐久信息。耐久结果不得只走通知条；估算来源只由 footer `~`/`?%` 表达，不另立通知。

**与 `agent_demo` 分界**：包引擎演示，不是本端 SSOT。硬边界见 `crates/xylitol-tui/AGENTS.md`。

**跨端公共体验**：已开放端共有能力一套学习成本——细则：`llmanspec/specs/protocol-app/`、`user-experience/`。端侧附加：改公共交互前先对齐全部已开放端；未开放端（gpui）不要写成现行 MUST，也不要在本端预埋第二套动作 id。

## 硬约束

| 规则 | 禁止 |
|---|---|
| 滚动提示默认 **尾插**（跟底可见、保 paint-cache）；顶层原则与例外见 `app-tui-fixed-zone` spec 与本端代码；瞬时确认优先页脚 / 状态条 / 槽 | 无理由顶插；瞬时确认堆进滚动区 |
| 渲染只用 `xylitol_tui`；缺能力先改包再接线；产品路径 **host 驱动** | 准通用实现塞进本端；产品调 demo 启动 API（`TUI::start()`） |
| 鼠标默认 **ApplicationOwned**（host 启动构造时绑定，缺省 AO；应用内选区 + dock 排除）；命中/探针管线细节以包代码与 `package-tui-pointer` 合约为 SSOT；模式命名以包 AGENTS 硬约束为准（`Inline` / `ApplicationOwned`） | mid-session 热切；产品路径读/设 lab env（`XYLITOL_TUI_*`）或用 `HitPriorityFn` |
| 固定区可点元素（editor 上边框视图 chip、槽 `[关闭]`、槽行、队列条）必须能只靠指针到达；chip 命中以屏上实际绘制行为准（`app-tui-mouse-actions`） | 只在键位上暴露能力；按上次渲染宽度反推 chip 列 |
| Agent 只经产品线协议客户端（JSON-RPC 文本，端侧）或 Host 内 `XyDriver`（依赖纪律同 `src/AGENTS.md`） | 产品 TUI 默认同进程直握操作器 |
| Trust 在 CLI 门禁（见 `llmanspec/specs/app-tui-trust/`、`domain-security/`）；本端 `EditorSlot::Choice` 已解冻给内置 `ask` | Plate / Settings stub 冻结期间扩展其功能或抢先落对应 spec |
| 颜色走本端 theme token；已确认需求须先有 harness / BDD 护栏 | 另立 hex；没有护栏就先写实现 |
| UiEntry：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush；`Palette` = [`DESIGN.md`](./DESIGN.md) frontmatter 运行时快照 | 给 user / assistant 加轨或洗底 |
| steer / follow-up 走 `src/AGENTS.md` 的 `XyDriver` 队列边界 | 本端持有 ReAct 内部队列 |
| 会话能力覆盖 `/capabilities` 走 `CapabilitiesSlot`；toggle 经共享 dispatch 提交 `SetCapability`，下一回合边界生效（`app-tui-commands`） | 端侧改全局配置 / 立即拆当前回合工具 |
| 队列条每类至多一行、显示最新正文；>1 才显示计数（`app-tui-input`） | 逐条铺开导致固定区随队列线性增高 |

**Esc 行为**：分场景收敛（idle 空 editor 双 Esc 开树 / busy 锁存 abort 且 drain 仍须调 `XyDriver::abort` / overlay 先关槽）；完整矩阵以代码、harness 测与 stage-QA design 为 SSOT，此处不复制。Agent abort 与 Bang abort 文案 / 抑制策略不同。

## 验证与 HOW

| 角色 | 做什么 |
|---|---|
| Agent 必跑 | 相关 harness/tests；核心 BDD；`just fmt` + 相关 clippy |
| Agent 尽量跑 | `just test-tui-e2e-pty`（真终端） |
| 人类 | 最短手测观感；**不**替代 harness |

包侧验证分层（按键→状态 / snapshot / 时序 / 真终端 E2E）：`crates/xylitol-tui/AGENTS.md`「验证」+ skill `xylitol-dev`（`references/tui-harness.md`）。排障：`xylitol-dev`（`references/observability.md`）（勿整文件灌 log）。新增应用端：`l8ng-write-surface`。

应用端无头帧挂载（BDD 直驱真实渲染）：`SceneBuilder`（经 `xylitol::app::tui` 导出；契约 `package-tui-testing`）。

折叠/展开是**端侧纯呈现**：状态留客户端（`activity_fold` / `ScrollbackFold` / `FoldHitTable`），**不得**往返 Host，也不要求 Host 持久化折叠状态。产品 TUI attach 模式 **不得**直读 Host 会话存储（`XyRemoteDriver::session_store()` 为 `None`），会话读取只经协议 unary。历史按窗口加载：首屏最新一页（`HISTORY_PAGE = 200`）、窗口上限 `HISTORY_WINDOW = 800`，超出后最旧部分回落为顶部可点击 `ScrollNotice`；短会话不出现任何分页提示（`history_window`）。

live 只进 scrollback；历史/分叉以会话树为准。不做 Codex TranscriptView；不做运行时 Settings/Plate 改配置；**不**绑定 Ctrl+P 打开 Command Plate stub。
