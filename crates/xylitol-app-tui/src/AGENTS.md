# crates/xylitol-app-tui/

本端专属边界。分层与 seam：`src/AGENTS.md`。引擎：`crates/xylitol-tui/AGENTS.md`。写法：根 `AGENTS.md`。

写/改本端：先读本节职责，再跟 `l8ng-write-tui`。刻意差异台账：[`PI_DELTAS.md`](./PI_DELTAS.md)。

## 角色（按职责，不按文件表）

基于 `xylitol-tui` 的 host 驱动产品端（CLI 默认 TUI）。包内通用组件可先在 `agent_demo` 验证引擎行为，再接线本端——验证引擎不等于产品 UI SSOT（demo 硬边界见 `crates/xylitol-tui/AGENTS.md`）。

| 角色 | 职责 | 禁止 |
|---|---|---|
| host / 入口循环 | 终端扇入、busy/idle 策略、同步步进 | 把业务堆成 God 协调者；能下沉的逻辑先下沉 |
| effects | 唯一异步副作用 drain 循环，送入 `XyDriver` / dispatch | 第二套 slash/steer 执行路径；harness 必须复用本循环 |
| commands | 解析为 pending | 直接执行副作用 |
| bridge | `XyEvent` 映射到 UI 模型 | layout 直接 match 事件 |
| layout / widgets | 呈现与局部交互 | 直接调 Driver / 读写 session |
| harness / tests | 合成切片护栏 | 与生产复杂度门禁混用（测试另计） |

运行时真值：本目录产品代码。视觉色板 SSOT：[`DESIGN.md`](./DESIGN.md) frontmatter；改色板后跑 `just qa-check-tui-tokens`（Palette 对照）。现行词「固定区」= layout/widgets；固定区概念勿称 `shell` / `scene`（既有 `SceneBuilder`、`choice_shell` 等命中 / 折叠场景符号除外）。

信息呈现词汇（固定）：讨论与本端文档 / host 必须使用 [`llmanspec/specs/app-tui-fixed-zone/`](../../../../llmanspec/specs/app-tui-fixed-zone/) 与产品代码中的固定词——尤其滚动提示、通知条、通知栈、待办栏、尾插 / 顶插；表外称呼勿再引入。通知分类与耐久结果、估算来源的呈现细则以该 spec 与产品代码为 SSOT。

跨端公共体验：已开放端动作语义一套，细则见 `llmanspec/specs/protocol-app/`、`user-experience/`；改公共交互前先对齐全部已开放端；未开放端（gpui）不写成现行必须，也不在本端预埋第二套动作 id。

## 硬约束

| 规则 | 禁止 |
|---|---|
| 滚动提示默认尾插（跟底可见、保 paint-cache）；顶层原则与例外见 `app-tui-fixed-zone` spec 与本端代码；瞬时确认优先页脚 / 状态条 / 槽 | 无理由顶插；瞬时确认堆进滚动区 |
| 渲染只用 `xylitol_tui`；缺能力先改包再接线；产品路径 host 驱动 | 准通用实现塞进本端；产品调 demo 启动 API（`TUI::start()`） |
| 鼠标默认 ApplicationOwned（host 启动构造时绑定；应用内选区 + dock 排除）；模式命名以包 AGENTS 为准 | mid-session 热切 |
| 命中 / 探针管线以 `package-tui-pointer` 合约为 SSOT | 产品路径读/设 lab env（`XYLITOL_TUI_*`）或用 `HitPriorityFn` |
| 固定区可点元素（editor 上边框视图 chip、槽 `[关闭]`、槽行、队列条）必须能只靠指针到达；chip 命中以屏上实际绘制行为准（`app-tui-mouse-actions`） | 只在键位上暴露能力；按上次渲染宽度反推 chip 列 |
| Agent 只经产品线协议客户端（JSON-RPC 文本，端侧）或 Host 内 `XyDriver`（依赖纪律同 `src/AGENTS.md`） | 产品 TUI 默认同进程直握操作器 |
| Trust 在 CLI 门禁（见 `llmanspec/specs/app-tui-trust/`、`domain-security/`）；本端 `EditorSlot::Choice` 供内置 `ask` 使用 | Plate / Settings stub 冻结期间扩展其功能或抢先落对应 spec |
| 颜色走本端 theme token；已确认需求须先有 harness / BDD 护栏 | 另立 hex；没有护栏就先写实现 |
| UiEntry：tool/bash/diff = status 左边轨 + gutter；user / assistant / thinking = flush | 给 user / assistant 加轨或洗底 |
| steer / follow-up 走 `src/AGENTS.md` 的 `XyDriver` 队列边界 | 本端持有 ReAct 内部队列 |
| 会话能力覆盖 `/capabilities` 走 `CapabilitiesSlot`；toggle 经共享 dispatch 提交 `SetCapability`，下一回合边界生效（`app-tui-commands`） | 端侧改全局配置 / 立即拆当前回合工具 |
| 队列条每条排队消息独占一行：Steering 组在上、Follow-up 组在下，行首 muted 组标签（`Steering` / `Follow-up`），正文截断为单行（省略号），固定区高度随队列条数增长；队列真值源在 Host：排队消息下一拍全部交付 Host 队列，本端只保留扣住的首条门禁根提交；本端跟随 Host 事件带（idle 收到 AgentStart 也进入忙碌），不得本地起跑排队消息；点击某行只还原该条，Alt+Up 后进先出；鼠标悬停只高亮指针所在那一行（`app-tui-input`、`server-core` r3378 续跑队列自消费） | 隐藏排队行或按组折叠成一行；正文换行占多行；行首带组内序号或组深度计数；悬停高亮整条队列；footer/status 另加队列计数徽章 |

Esc 行为分场景收敛：

| 场景 | 行为 |
|---|---|
| idle 空 editor | 双 Esc 开树 |
| busy | 锁存 abort；drain 仍须调 `XyDriver::abort` |
| overlay | 先关槽 |

完整矩阵以代码、harness 测与 stage-QA design 为 SSOT，此处不复制。Agent abort 与 Bang abort 的文案 / 抑制策略不同。

## 验证与 HOW

| 角色 | 做什么 |
|---|---|
| Agent 必跑 | 相关 harness/tests；核心 BDD；`just fmt` + 相关 clippy |
| Agent 尽量跑 | `just test-tui-e2e-pty`（真终端） |
| 人类 | 最短手测观感，不替代 harness |

包侧验证分层（按键到状态 / snapshot / 时序 / 真终端 E2E）：`crates/xylitol-tui/AGENTS.md`「验证」+ skill `xylitol-dev`（`references/tui-harness.md`）。排障：`xylitol-dev`（`references/observability.md`）（勿整文件灌 log）。新增应用端：`l8ng-write-surface`。

应用端无头帧挂载（BDD 直驱真实渲染）：`SceneBuilder`（经 `crates/xylitol-app-tui/src/lib.rs` 导出；契约 `package-tui-testing`）。

折叠 / 展开是端侧纯呈现：状态留客户端（`activity_fold` / `ScrollbackFold` / `FoldHitTable`），不得往返 Host，也不要求 Host 持久化折叠状态。

产品 TUI attach 模式不得直读 Host 会话存储（`XyRemoteDriver::session_store()` 为 `None`），会话读取只经协议 unary。

历史按窗口加载：首屏最新一页、窗口有上限（常量以代码为准），超出后最旧部分回落为顶部可点击 `ScrollNotice`；短会话不出现分页提示（`history_window`）。

live 只进 scrollback；历史/分叉以会话树为准。不做 Codex TranscriptView；不做运行时 Settings/Plate 改配置，不绑定 Ctrl+P 打开 Command Plate stub。
