# src/app/tui/ 终端 UI

本文件只放 `src/app/tui/` **专属**规则。分层架构、不变量、应用面状态表、seam（`composition → Driver → XyEvent`）的 SSOT 是 `src/AGENTS.md`，本文件不重复；全局规则见根 `AGENTS.md`。

> **写或改 TUI？** 先读 `src/AGENTS.md` 的分层不变量，再用 `write-tui` skill（`.agents/skills/write-tui/SKILL.md`，覆盖目标文件布局、新特性落点、测试放置、约定）。新增应用面的方法论总纲见 `write-surface` skill。

> **现状（2026-07-02）**：TUI 已落地（c340），依赖瘦身（c341：`ratatui-core` + `ratatui-crossterm` 直依），渲染 harness + RenderedLine seam（c360），流式 mutable-last-line + 组件化 + route B 底部面板（c365）。`mod.rs` 的 inline REPL 经 `InProcessDriver` 驱动；流式文字在 mutable 顶行每帧重绘（ratatui buffer 内，透明 bg 融进 scrollback），换行即 `insert_before` 固化进 scrollback。**thinking 是正文**（灰色，由 `thinking_buf` 独立流式 + commit 为 `ThinkingText` 行，显示在 mutable 顶行而非独立块）；首个 `TextDelta` 切到回复流（commit `AssistantText` 行）。底部 `BottomPanel`（border + bg）**只含 `InputPrompt`**，固定 3 行，idle 不填满 tail 区。`Spinner` 是底层组件但**当前未接线**——`Thinking…` 占位符 / 流式文字本身即活动指示。渲染层拆 `components/` 下 6 个可复用 widget。`/exit` `/model` 两条 slash 命令可用。

## 文件布局（已落地）

对标 kimi-code `apps/kimi-code/src/tui/`，适配单 crate + Driver seam。入口链：`main.rs → lib::run → app::cli::run`（mode 分发）`→ app::tui::run`（REPL 主循环）。目标目录：

- `mod.rs` — `run()` REPL 主循环：`tokio::select!` 多源（XyEvent mpsc / crossterm 阻塞读 / cancel / tick）。协调器，不堆业务逻辑。
- `init.rs` — panic 恢复 hook（c355）+ 本地终端初始化（替代 umbrella `ratatui::init`）：`DefaultTerminal` 类型别名 + `try_init_with_options` + `restore`（c341）。
- `terminal.rs` — `InlineTerminal`：`Viewport::Inline` 生命周期，RAII `Drop` 恢复 raw mode（spec tui15）；含 `wrap_to_width`（CJK 显示宽度）。
- `app.rs` — `TuiApp` 状态机：输入缓冲 / 双 stream buffer（text + thinking）/ spinner frame / streaming 态；spawn 任务 drain `EventStream` → mpsc。
- `render.rs` — `XyEvent → RenderedLine` 单一 seam + `RenderedLine` UI 数据类型 + wrap 工具 + `draw_tail_frame`（thin wrapper 委托 `Tail` widget + 设光标）。
- `input.rs` — crossterm 键位 → `InputOutcome`（Submit/Slash/Abort/Quit/Idle）。MVP 单行输入。
- `commands.rs` — slash 解析 `/exit` `/model`，复用 `protocol::Command` 语义（MVP 本地分发；统一 dispatch 是 c335 的职责）。
- `theme.rs` — 语义颜色 token SSOT（`Palette`：primary/text_dim/assistant/user_prompt/tool/error/spinner/thinking + `panel_bg`/`panel_border`）。
- `components/` — 可复用 widget（c365 组件化，每个 TestBackend 可独立验证，消费 UI 数据类型不碰 `XyEvent`/agent/infra）：
  - `transcript_line.rs` — `TranscriptLine`：`RenderedLine → Buffer`（wrap + CJK，含 `ThinkingText` 灰），insert_before commit 与 TestBackend 共用。
  - `mutable_line.rs` — `MutableLine`：pending_tail 顶行（caller 经 `MutableKind` 选 style：thinking 灰/text 正常/tool 黄，透明 bg 融进 scrollback，紧贴面板 top-anchored）。
  - `input_prompt.rs` — `InputPrompt`：输入框（无 `❯` 前缀，MVP 单行）。
  - `bottom_panel.rs` — `BottomPanel`：带 border + panel_bg 的 chrome 容器，**只含 InputPrompt**，固定 3 行（idle 不填满 tail 区）。
  - `tail.rs` — `Tail`：组合 MutableLine（顶）+ BottomPanel（底）；`draw_tail_frame` 退化为此。
  - `spinner.rs` — `Spinner`：单 glyph spinner（底层可复用，**当前未接线**——活动指示靠 `Thinking…` / 流式文字）。

## TUI 专属约束（normative）

下列是分层不变量之外的 TUI 局部约定：

- **slash 命令语义复用** `crate::protocol::Command` 同名变体（对标 `app/rpc.rs::dispatch`），不另造命令体系。
- **颜色一律走** `theme.rs` 语义 token，组件不得硬编码颜色字面量。
- **组件只负责呈现与局部交互**，禁止直接调 `Driver`、读写 agent/session 状态（即应用面 seam 约束的具体化，见 `src/AGENTS.md`）。
- **依赖直依 ratatui-core + ratatui-crossterm**（c341），MUST NOT 引入 umbrella `ratatui` crate。c360 放宽了 widget 选型：`ratatui-widgets` 子集（`Paragraph`/`Block`/`List`/`ListState`/`Clear`）能复用就复用；当库 widget 不符合 inline 渲染需求（如特定 CJK/emoji 行为、自定义交互）时，MAY 基于 `ratatui-core` 原语（`Buffer`/`Layout`/`Style`/`Text`/`Widget` trait）自建独立 widget。选型由适配度驱动，不是一刀切。注意：`Widget` trait 在 `ratatui_core::widgets::Widget`，不在 `ratatui_widgets`。
- **渲染层与业务流解耦**（c360 spec tui42）：渲染函数 MUST 消费 UI 专用数据类型（`RenderedLine` enum：`UserInput`/`AssistantText`/`ThinkingText`/`ToolSummary`/`Status`），而非直接 match `XyEvent` 变体或调 `Driver`/agent 方法。`XyEvent → RenderedLine` 的翻译集中在单一 seam 函数，渲染层不知道 `XyEvent` 的存在。
- **渲染层有 TestBackend 验收**（c360 spec tui41）：渲染行为 MUST 用 `ratatui_core::backend::TestBackend` + `assert_buffer_lines` 覆盖（测行为非实现），至少覆盖 `commit_to_scrollback`（insert_before 路径）+ `draw_tail_frame` + CJK 换行 + TurnEnd flush。
- **新 `XyEvent` 渲染 / transcript 消息类型** → `render.rs` 或 `components/messages/`；**需要新 agent 行为** → 不进 TUI，先在 `runtime_protocol/` 加 port、`agent/` 加实现，TUI 只消费。

## Out of scope（intentionally deferred）

已知但**故意暂不做**的事，登记在此避免重复评估。需做时开独立变更。

- **多行编辑器**：当前 `input.rs` 是 MVP 单行；未来支持方向键 + 最多 3 行。
- **`StatusBar`（统计 + 模型名）**：底部信息栏，待 TUI 主体稳定后加。
- **Spinner 接线**：`spinner.rs` + `app.rs::spinner_idx` 已就位但未渲染（活动指示靠 `Thinking…`/流式文字）；接不接由真实体验决定。
- **FrameScheduler / EventBroker pause-resume**：codex 的帧调度与事件暂停机制，当前每帧全量重绘够用，未引入。
- **统一 dispatch（c335）**：TUI 的 slash 命令当前是本地 MVP 分发（`commands.rs`），等 c335 落地共享 dispatch 后切换。

## 编码约定

- 不过度封装，尤其一两行的函数直接内联，不套两层 wrapper。
- 无状态 / 无 UI 副作用的函数不作为 `mod.rs` 私有方法，放外部工具函数。
- 常量归 `theme.rs` 或对应 `components/` 内，不散落在逻辑代码里。
- Rust 命名遵循 `rustfmt.toml`：snake_case 模块/函数/变量，PascalCase 类型。
