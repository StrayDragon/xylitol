# codex TUI 调研报告

## 0. 元信息

| 项目 | 值 |
|------|-----|
| **技术栈** | ratatui 0.29.0 (git fork: `github.com/nornagon/ratatui`, rev `9b2ad129`) + crossterm (git fork: `github.com/nornagon/crossterm`, rev `87db8bfa`) |
| **后端** | crossterm (通过 `CrosstermBackend<Stdout>`)。启用 feature: `bracketed-paste`, `event-stream`, `scrolling-regions`, `unstable-backend-writer`, `unstable-rendered-line-info`, `unstable-widget-ref` |
| **TUI 入口** | `codex-rs/tui/src/lib.rs:1479` — `run_ratatui_app()`: 初始化 terminal → 创建 `Tui` → 进入 `App::run()` 事件循环 |
| **ratatui 依赖形态** | **umbrella `ratatui` crate** (非 `ratatui-core`)。codex 使用 `ratatui = "0.29.0"` 完整 crate，通过 workspace 级 `[patch.crates-io]` 替换为 OpenAI fork。 |
| **报告日期** | 2026-07-02 |
| **仓库** | `/home/l8ng/Projects/__straydragon__/codex` (OpenAI Codex CLI Rust 实现) |

> 注: xylitol 使用 `ratatui-core` + `ratatui-widgets`(子 crate 拆分发)。codex 用 umbrella。这意味着 xylitol 的积木集(ratatui-core 提供的 trait/backend/widgets)是 ratatui 0.29 umbrella 所提供公共 API 的一个**子集**,但 core widget 名称和 trait 签名一致。

## 1. Widget/组件 全景清单

### 1.1 层次结构概览

```
App (顶层编排器)
├── ChatWidget (主聊天控件)
│   ├── Transcript 区域
│   │   ├── HistoryCell[] (已提交消息,类型多样)
│   │   └── active_cell (流式中,可变)
│   └── BottomPane (底部交互面板)
│       ├── ChatComposer (文本输入框,含 @/$ mention)
│       └── BottomPaneView 栈 (弹窗/对话框)
│           ├── ApprovalOverlay (命令审批)
│           ├── ListSelectionView (模型/配置选择)
│           ├── RequestUserInputOverlay (用户输入请求)
│           ├── McpServerElicitationOverlay (MCP 服务器引出)
│           ├── AppLinkView (应用链接/安装引导)
│           ├── HooksBrowserView (hooks 浏览器)
│           ├── CustomPromptView (自定义 prompt)
│           ├── SkillsToggleView (技能开关)
│           ├── SkillPopup (技能详情弹窗)
│           ├── CommandPopup (命令弹窗)
│           ├── FileSearchPopup (文件搜索弹窗)
│           ├── FeedbackView (反馈提交)
│           └── ...
├── PagerOverlay (Ctrl+T 转录全屏)
├── OnboardingScreen (首次使用引导)
├── ResumePicker (会话恢复选择器)
└── FileSearchManager (文件搜索)
```

### 1.2 详细组件清单

| 名称 | 类别 | 文件:行 | 职责 | 对 xylitol 可借鉴度 |
|------|------|---------|------|-------------------|
| **App** | chrome | `src/app.rs:1-1376` | 顶层编排器: 事件循环 `run()`, AppServer 通信, 路由分发, 配置持久化, 会话生命周期 | 高 — 架构模式(事件驱动+select!+状态机)可直接借鉴 |
| **ChatWidget** | chrome | `src/chatwidget.rs:1-2073` | 主聊天表面: 管理 transcript cells, active_cell 流式更新, agent turn 状态, overlay 同步 | 高 — coordination 模式 |
| **HistoryCell** (enum) | 消息 | `src/history_cell/mod.rs:1-333+` | 转录条目的单元枚举,约 20+ 变体 | 高 — 设计模式(每个消息类型一个 variant) |
| ├ UserHistoryCell | 消息 | `src/history_cell/messages.rs` | 用户消息渲染 | 中 |
| ├ AgentMessageCell | 消息 | `src/history_cell/messages.rs` | Agent 消息(含 markdown 流式) | 高 — markdown 流式渲染算法 |
| ├ ExecCommandCell | 消息 | `src/history_cell/exec.rs` | 命令执行输出(exit code + stdout/stderr) | 高 — exec cell 的 truncation 和折叠逻辑 |
| ├ McpToolCallCell | 消息 | `src/history_cell/mcp.rs` | MCP 工具调用显示 | 中 |
| ├ ApplyPatchCell | 消息 | `src/history_cell/patches.rs` | 文件 diff/patch 应用显示 | 中 |
| ├ ReasoningCell | 消息 | `src/history_cell/messages.rs` | 推理过程显示(可折叠) | 高 — reasoning 折叠UI |
| ├ PlanCell | 消息 | `src/history_cell/plans.rs` | plan 步骤渲染 | 中 |
| ├ SearchCell | 消息 | `src/history_cell/search.rs` | 搜索结果展示 | 低 |
| ├ NoticeCell | 消息 | `src/history_cell/notices.rs` | 系统通知/错误 | 高 |
| └ ... | | | | |
| **ChatComposer** | 输入 | `src/bottom_pane/chat_composer.rs:1-11203` | 多行文本输入, @/$ mention 弹出, 占位符, paste burst 检测, vim 模式, 多 attachment | **极高** — 核心输入组件,xylitol 可直接复用设计 |
| **ComposerInput** (pub) | 输入 | `src/public_widgets/composer_input.rs:1-136+` | ChatComposer 的公开发布封装,面向其他 crate(codex-cloud-tasks等) | **极高** — xylitol 可直接引用此设计 |
| **Footer** | chrome | `src/bottom_pane/footer.rs:1-2082` | 底部状态栏: 上下文百分比, 快捷键提示, 模式指示器 | 高 — 状态栏设计模式 |
| **ApprovalOverlay** | 对话框 | `src/bottom_pane/approval_overlay.rs:1-2409` | 命令审批弹窗: 显示命令/权限/网络策略, 提供选项列表 | **极高** — 审批弹窗设计,xylitol 定要 |
| **ListSelectionView** | 对话框 | `src/bottom_pane/list_selection_view.rs:1-2810` | 通用选择列表(模型选择/配置选择/approval mode选择) | **极高** — 可复用的通用选择器 |
| **RequestUserInputOverlay** | 对话框 | `src/bottom_pane/request_user_input/mod.rs` | 用户输入请求弹窗,多问题表单 | 高 |
| **McpServerElicitationOverlay** | 对话框 | `src/bottom_pane/mcp_server_elicitation.rs:1-2732` | MCP 服务器安装/配置引出 | 低 — 特定于 MCP |
| **AppLinkView** | 对话框 | `src/bottom_pane/app_link_view.rs:1-1704` | 应用安装/启用/认证引导 | 低 |
| **HooksBrowserView** | 对话框 | `src/bottom_pane/hooks_browser_view.rs:1-1634` | hooks 浏览/管理 | 低 |
| **StatusIndicatorWidget** | 流式 | `src/status_indicator_widget.rs:1-478+` | 任务进行中状态行(spinner+interrupt hint+details) | **极高** — 流式状态指示器 |
| **Shimmer** | 流式 | `src/shimmer.rs:1-41+` | 呼吸闪烁动画(用于处理中文字) | 高 — shimmer 效果 |
| **StreamController** | 流式 | `src/streaming/controller.rs:1-1843+` | 双区域(稳定区+tail区)流式 markdown 渲染控制器 | **极高** — 核心流式算法: "已提交" vs "tail" 分区域控制 |
| **StreamState** | 流式 | `src/streaming/mod.rs:1-65+` | newline-gated markdown 收集 + FIFO 提交队列 | **极高** — 换行门控收集策略 |
| **MarkdownStreamCollector** | 流式 | `src/markdown_stream.rs` | 增量 markdown 解析器 | 高 |
| **MarkdownRender** | 流式 | `src/markdown_render.rs:1-2771` | markdown→ratatui Line 渲染(持代码高亮) | 高 — syntect 集成 |
| **DiffRender** | 流式 | `src/diff_render.rs:1-2534` | diff 可视化渲染(侧栏 + 着色) | 高 |
| **PagerOverlay** | chrome | `src/pager_overlay.rs:1-1570` | 全屏转录叠加层(Ctrl+T), 历史+live tail 同步 | 中 |
| **BottomPane** | chrome | `src/bottom_pane/mod.rs:1-3060` | 底部面板容器:Composer + View 栈管理 | **极高** — View 栈架构模式 |
| **BottomPaneView** trait | chrome | `src/bottom_pane/bottom_pane_view.rs:1-93+` | 可插入 View 的 trait 定义(handle_key, is_complete, completion) | **极高** — xylitol 可直接用此 trait |
| **Renderable** trait | chrome | `src/render/renderable.rs:1-424+` | 自定义渲染 trait (render, desired_height, cursor_pos, cursor_style) | 高 — 替代 ratatui Widget 的自定义抽象 |
| **FlexRenderable** | chrome | `src/render/renderable.rs` | 弹性布局容器(flex 比例分配高度) | 高 |
| **ColumnRenderable** | chrome | `src/render/renderable.rs` | 列式布局容器 | 中 |
| **custom_terminal** | chrome | `src/custom_terminal.rs:1-894+` | 自定义 Terminal 封装(跟踪光标,OSC 8 超链接,视图区,行插入) | **极高** — OSC 8 超链接 + `insert_history_lines` 核心 |
| **Keymap** | chrome | `src/keymap.rs:1-2947` | 运行时键位映射(上下文感知,全局回退,配置解析) | 高 — 键位解析/冲突检测算法 |
| **LiveWrap** | 工具 | `src/live_wrap.rs` | 行实时换行(用于历史行插入) | 中 |
| **Wrapping** | 工具 | `src/wrapping.rs:1-1657` | textwrap 集成 + ratatui Line 换行 | 高 — word_wrap_lines/word_wrap_line 工具 |
| **TextFormatting** | 工具 | `src/text_formatting.rs` | truncation, proper_join, format 工具 | 中 |
| **OnboardingScreen** | 对话框 | `src/onboarding/onboarding_screen.rs` | 首次运行引导(登录/信任目录) | 低 |
| **ResumePicker** | 对话框 | `src/resume_picker.rs:1-6351` | 会话恢复/分叉选择器(表格布局) | 中 |
| **ThemePicker** | 对话框 | `src/theme_picker.rs` | 主题选择界面 | 低 |
| **ModelCatalog** | 对话框 | `src/model_catalog.rs` | 模型目录(模型选择) | 中 |
| **AgentPicker** | 对话框 | `src/multi_agents.rs` | 多 agent 选择器 | 低 |
| **GoalDisplay** | chrome | `src/goal_display.rs` | goal/budget 显示 | 中 |
| **TokenUsage** | chrome | `src/token_usage.rs` | token 用量追踪/显示 | 中 |
| **CwdPrompt** | 对话框 | `src/cwd_prompt.rs` | 工作目录选择对话框 | 中 |
| **AsciiAnimation** | 流式 | `src/ascii_animation.rs` | ASCII 宠物动画 | 低 |
| **Frames** | 流式 | `src/frames.rs` | 36 帧动画编译期嵌入(10 种变体) | 低 |

### 1.3 渲染系统两条路线

codex TUI 同时使用两套渲染 API:

1. **ratatui WidgetRef trait** — 用于简单组件(PagerOverlay, 部分历史 cell 的 Paragraph 渲染)
2. **自定义 `Renderable` trait** — 用于需要灵活布局(height negotiation)的复杂组件:
   - `render(&self, area: Rect, buf: &mut Buffer)` — 渲染到 buffer
   - `desired_height(&self, width: u16) -> u16` — 向父级报告期望高度
   - `cursor_pos(&self, area: Rect) -> Option<(u16, u16)>` — 光标位置
   - `cursor_style(&self, area: Rect) -> SetCursorStyle` — 光标样式

`Renderable` 与 `FlexRenderable` (flex 比例分配) + `ColumnRenderable` 构成了自定义组合布局系统。

## 2. 操作形态

### 2.1 输入交互

```
crossterm::event_stream() → TuiEventStream (raw)
  → App::handle_tui_event()  // 全局热键(重分发? / Ctrl+C / Ctrl+D)
  → ChatWidget::handle_key_event()  // 判断是否中断/审批等
    → BottomPane::handle_key_event()  // View 栈优先
      → 活跃 View.handle_key_event()  // 如果有活跃弹窗
      → ChatComposer.handle_key_event()  // 默认输入
```

**输入链关键节点**:
- `tui.rs` — `TuiEventStream` 包装 crossterm 原始事件, 支持 `EventBroker` 测试注入
- `app.rs` — `App::run()` 中 `tokio::select!` 同时监听: TUI 事件 / AppServer 事件 / 内部 AppEvent
- `keymap.rs` — `RuntimeKeymap` 分 `AppKeymap` / `ChatKeymap` / `ComposerKeymap` / `EditorKeymap` / `VimNormalKeymap` / `PagerKeymap` / `ListKeymap` / `ApprovalKeymap`,每个都绑定到 `KeyBinding` 列表

**ChatComposer 输入特性**:
- 多行输入 (Enter 提交, Shift+Enter 新行)
- `@` mention (文件/插件/skill 搜索弹窗)
- `$` mention (环境变量/命令)
- paste burst 检测 (粘帖 >300ms 视为 burst)
- 占位符系统 (`{cwd}`, `{time}` 等自动展开)
- `ComposerInput` pub 封装供外部 crate 复用

### 2.2 流式渲染

```
AppServer → ServerNotification 流
  → AppServerSession::next_event()
  → App::handle_app_server_event()
  → ChatWidget::update_from_stream()  // 更新 active_cell
  → StreamController::emit(text) → StreamCore 处理
    → raw_source 追加 + 重新处理
    → 增量渲染: newline-gated (MarkdownStreamCollector)
    → 两区域:
      - 稳定区(committed): 已提交到动画队列的完整行
      - tail 区: 当前行可变(可被后续 token 修改)
    → 表 holdback 检测: 检测到 pipe 表格时,表头部到尾部的行保留在 tail 区不提交
```

**关键机制**:
- `StreamCore` — 双区域流式控制器(`src/streaming/controller.rs`)
- `StreamState` — FIFO 队列 + 时间戳追踪(`src/streaming/mod.rs`)
- `Chunking` — 自适应 drain 策略(`src/streaming/chunking.rs`)
- `CommitTick` — drain 时机控制器(`src/streaming/commit_tick.rs`)
- `TableHoldbackScanner` — pipe 表格检测(`src/streaming/table_holdback.rs`)
- `MarkdownStreamCollector` — 增量 markdown 解析(`src/markdown_stream.rs`)
- 宽度变更时 `StreamCore::set_width()` 重新渲染全部 + 重建 queue

### 2.3 中断/abort

- **Ctrl+C**:
  1. 活跃 View 优先处理(一般 dismiss 弹窗)
  2. 否则 ChatComposer history search 取消
  3. 否则→中断当前 agent turn (发送 `/abort` 给 AppServer)
  4. 双按 Ctrl+C → 退出 TUI
- **Esc**:
  - 弹窗取消/返回
  - 组件内状态重置
- Backtrack: `src/app_backtrack.rs` 提供"回到上一步"状态管理

### 2.4 对话框/overlay 触发与退出

**BottomPaneView 栈架构** (`src/bottom_pane/bottom_pane_view.rs`):

```
trait BottomPaneView: Renderable {
  fn handle_key_event(&mut self, key: KeyEvent);
  fn is_complete(&self) -> bool;       // 视图是否完成
  fn completion(&self) -> Option<ViewCompletion>;  // Accepted/Cancelled
  fn dismiss_after_child_accept(&self) -> bool;
  fn view_id(&self) -> Option<&'static str>;   // 外部刷新标识
  fn on_ctrl_c(&mut self) -> CancellationEvent;
}
```

- `BottomPane` 维护 `views: Vec<Box<dyn BottomPaneView>>` 栈
- `push()` 压入新 View → 取代 Composer 成为输入目标
- View 完成后自动 pop → 回到 Composer
- View 可以通过 `FrameRequester` 请求重绘
- 部分 View (如 `RequestUserInputOverlay`) 支持多层嵌套(子 View accepted 后父 View 继续)

**触发方式**:
| 事件 | 弹窗 | 位置 |
|------|------|------|
| 工具决定要执行命令 | `ApprovalOverlay` | `approval_events.rs` → `ChatWidget.show_approval_overlay()` |
| 用户按 `Ctrl+P` | `ListSelectionView` (模型选择) | `app/event_dispatch.rs` |
| 用户按 `@` | mention popup (ChatComposer内嵌) | `chat_composer.rs` |
| AppServer 发来 `ServerRequest(PermissionsRequest)` | `ApprovalOverlay` (权限请求) | `app/app_server_events.rs` |
| 用户按 `Ctrl+T` | `PagerOverlay` (转录全屏) | `app/event_dispatch.rs` |
| 用户按 `?` | shortcut overlay (ChatComposer内嵌) | `chat_composer.rs` |

## 3. 测试 Harness(核心章节)

### 3.1 总体策略

codex TUI 使用**三层测试策略**:

| 层 | 范围 | 工具 | 关键文件 |
|----|------|------|---------|
| **Unit/Snapshot** | 单个 widget 渲染 | `insta::assert_snapshot!(name, terminal.backend())` + `TestBackend` (ratatui 内置) | 散布在各组件 `#[cfg(test)] mod tests` 中 |
| **VT100 Integration** | 历史行插入/换行行为 | `VT100Backend` 包装 `CrosstermBackend<vt100::Parser>` | `tests/suite/vt100_history.rs`, `tests/test_backend.rs` |
| **Full E2E** | 事件驱动 + AppServer mock | `AppServerSession` mock + `wiremock` + `AppEvent` 通道 | `app/tests.rs` (6263 行!), `core/suite/` |

### 3.2 快照机制

**实现原理**:
- 渲染组件到 `TestBackend` (ratatui 内置,内存 buffer), 然后 `insta::assert_snapshot!(name, terminal.backend())`
- `TestBackend` 实现了 `std::fmt::Display`, insta 能直接 dump buffer 内容为字符串
- 快照文件位于各组件的 `snapshots/` 目录下, 命名格式: `{crate}__{module}__{test_name}.snap`

**典型 snapshot 测试模式** (来自 `chat_composer.rs:4640`):

```rust
fn snapshot_composer_state(name: &str, enhanced_keys_supported: bool, setup: F) {
    let mut composer = ChatComposer::new(/*…*/);
    setup(&mut composer);

    let footer_props = composer.footer_props();
    let height = footer_lines + footer_spacing + 8;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| composer.render(f.area(), f.buffer_mut())).unwrap();

    insta::assert_snapshot!(name, terminal.backend());
}
```

**更新快照**:
```bash
just test -p codex-tui           # 运行生成 .snap.new
cargo insta pending-snapshots -p codex-tui  # 审查
cargo insta accept -p codex-tui             # 接受
```

**两种渲染后端用于 snapshot**:
- `TestBackend` (ratatui 内置, 内存 buffer, 用于组件级别)
- `VT100Backend` (包装 vt100::Parser, 可以查询 cursor 位置/screen rows, 用于集成测试)

VT100Backend 的核心实现 (`src/test_backend.rs`):

```rust
pub struct VT100Backend {
    crossterm_backend: CrosstermBackend<vt100::Parser>,
}

impl Backend for VT100Backend {
    fn size(&self) -> io::Result<Size> {
        let (rows, cols) = self.vt100().screen().size();
        Ok(Size::new(cols, rows))
    }
    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(self.vt100().screen().cursor_position().into())
    }
    // ...
}
```

### 3.3 fake 数据

**正常路径**:

1. **AppEvent + AppCommand 通道**: 测试用 `tokio::sync::mpsc` channel 创建 `(tx, rx)` → `AppEventSender(tx)`,构建 `ChatWidget` 时传入
2. **模型预设**: `test_support::TEST_MODEL_PRESETS` 是 `LazyLock<Vec<ModelPreset>>`,从 bundled models.json 加载并排序
3. **离线模型信息**: `codex_models_manager::test_support::construct_model_info_offline_for_tests()`
4. **配置**: `ConfigBuilder::default().build().unwrap()` + `ConfigOverrides` 覆盖

**App 级 E2E 测试** (`app/tests.rs`):

```rust
// 构造 App
let (chat_widget, app_event_tx, _rx, _op_rx) = make_chatwidget_manual_with_sender().await;
let config = chat_widget.config_ref().clone();
let app = App { ... };
```

**AppServer 模拟**: 直接构建 `ServerNotification` / `ServerRequest` 结构体,通过 app_event_tx 模拟事件:

```rust
// 模拟 turn_started 事件
app.handle_event(tui, &mut app_server,
    AppEvent::CodexOp(Op::TurnStarted { thread_id, turn_id })
).await?;
```

**异常路径**: 直接构造错误通知:

```rust
AppEvent::CodexOp(Op::TurnError {
    error: TurnError::CodexError {
        codex_error_info: Some(CodexErrorInfo {
            message: "Rate limit exceeded".into(),
            // ...
        }),
        // ...
    },
    // ...
})
```

### 3.4 e2e 操作序列

没有 "录制按键序列然后回放" 的机制。测试通过**直接调用 App 方法**来模拟操作序列,而非注入 KeyEvent。

测试模式示例 (`app/tests.rs`):

```rust
#[tokio::test]
async fn test_approval_flow() {
    let tui = test_support::make_test_tui().unwrap();
    let mut app = make_test_app().await;
    let mut app_server = start_embedded_app_server_for_picker(&config).await.unwrap();

    // 模拟: 用户消息
    app.handle_event(&mut tui, &mut app_server,
        AppEvent::CodexOp(Op::UserTurn { text: "list files".into(), attachments: vec![] })
    ).await.unwrap();

    // 模拟: Agent 回复 + 命令执行请求
    app.handle_event(&mut tui, &mut app_server,
        AppEvent::CodexOp(Op::ServerRequest(ResolvedAppServerRequest::ExecApproval(...)))
    ).await.unwrap();

    // 验证审批弹窗出现
    assert!(app.chat_widget.has_pending_approval());
}
```

EventBroker 用于测试事件流注入 (`tui/event_stream.rs`):

```rust
// 测试专用: 直接推送 TuiEvent
event_broker.send(TuiEvent::Key(KeyEvent::from(KeyCode::Char('?'))));
```

### 3.5 关键测试文件清单

| 文件 | 行数 | 内容 |
|------|------|------|
| `src/app/tests.rs` | 6263 | App 级编排测试,含模型选择/审批/backtrack/目标设置 |
| `src/app/tests/model_catalog.rs` | — | 模型目录测试 |
| `src/app/tests/plugin_catalog.rs` | — | 插件目录测试 |
| `src/app/tests/session_summary.rs` | — | 会话摘要测试 |
| `src/app/tests/startup.rs` | — | 启动流程测试 |
| `src/bottom_pane/chat_composer.rs` tests | ~3000+ | Composer 渲染快照 + 输入行为测试 |
| `src/bottom_pane/list_selection_view.rs` tests | — | 选择列表渲染快照 |
| `src/bottom_pane/approval_overlay.rs` tests | — | 审批弹窗渲染快照 |
| `src/bottom_pane/mcp_server_elicitation.rs` tests | — | MCP 引出渲染快照 |
| `src/bottom_pane/request_user_input/mod.rs` tests | — | 用户输入请求渲染快照 |
| `src/app/agent_status_feed_tests.rs` | — | agent 状态 feed 测试 |
| `src/markdown_render_tests.rs` | 1797 | markdown 渲染测试 |
| `src/history_cell/tests.rs` | 2624 | 历史 cell 渲染测试 |
| `src/diff_render.rs` tests | — | diff 渲染测试 |
| `src/wrapping.rs` tests | — | 换行逻辑测试 |
| `src/status/tests.rs` | 2038 | 状态栏测试 |
| `tests/suite/vt100_history.rs` | 164 | VT100 历史行插入 E2E |
| `tests/suite/vt100_live_commit.rs` | 43 | live commit 集成测试 |
| `tests/suite/resize_reflow.rs` | 567 | resize 重排 E2E |
| `tests/suite/status_indicator.rs` | 24 | ANSI escape 清理测试 |
| `src/streaming/controller.rs` tests | — | 流式控制器单元测试 |
| `src/config_update_tests.rs` | — | 配置更新测试 |
| `src/managed_new_thread_defaults_tests.rs` | — | 新线程默认值测试 |

### 3.6 test-tui SKILL.md 方法论摘要

> 注: `.codex/skills/test-tui/SKILL.md` 文件在仓库中不存在(可能是迁移前的文件或未创建)。以下是基于代码事实推导的方法论:

从代码中逆向出来的测试原则:

1. **每个影响 UI 的变更必须附带 insta snapshot 覆盖** — 这是 TUI 测试的核心契约
2. **Widget 级别测试**: 构造 widget → 调用 `terminal.draw()` → `insta::assert_snapshot!`
3. **App 级别测试**: 构造 `App` + mock `AppServerSession` → 通过 `AppEvent/AppCommand` 通道注入事件 → 验证内部状态
4. **行插入测试**: 使用 `VT100Backend` 验证实际终端输出,包括换行/emoji/CJK/ANSI 样式
5. **避免 mock KeyEvent 序列代替完整协议事件流** — 测试通过 AppEvent 通道而非按键序列驱动
6. **Model/Config 使用离线预设**,不依赖网络
7. **异常路径**: 直接构造 error 变体的协议事件(如 `TurnError::CodexError`)来验证错误显示

## 4. 可视化示意

### 4.1 正常对话中 — 空 composer (流式就绪状态)

```
"                                                                                                    "
"› Ask Codex to do anything                                                                          "
"                                                                                                    "
"                                                                                                    "
"                                                                                                    "
"                                                                                                    "
"                                                                                                    "
"                                                                                                    "
"  ? for shortcuts                                                                100% context left  "
```
> 来源: `bottom_pane/snapshots/...empty.snap`
>
> 使用的 widget: `ChatComposer` (输入框 + placeholder) + `Footer` (快捷键提示 + 上下文百分比)
> 状态: 用户未输入,等待输入

### 4.2 工具审批对话框

```
  Would you like to run the following command?

  Thread: Robie [explorer]

  $ echo hi

› 1. Yes, proceed (y)
  2. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel or o to open thread
```
> 来源: `bottom_pane/snapshots/...approval_overlay_cross_thread_prompt.snap`
>
> 使用的 widget: `ApprovalOverlay` (选项列表, 粗体标题, 命令显示)
> 使用的底层 widget: `ListSelectionView` (选择菜单)
> 状态: 用户被要求审批执行 `echo hi`,通过 `ListSelectionView` 渲染选项

### 4.3 模型选择 overlay

```

  Select Model and Effort

› 1. gpt-5.1-codex (current)  Optimized for Codex. Balance of reasoning
                              quality and coding ability.
  2. gpt-5.1-codex-mini       Optimized for Codex. Cheaper, faster, but less
                              capable.
  3. gpt-4.1-codex            Legacy model. Use when you need compatibility
                              with older automations.
```
> 来源: `bottom_pane/snapshots/...list_selection_model_picker_width_80.snap`
>
> 使用的 widget: `ListSelectionView` (通用选择列表, 带 subtitle 和 description)
> 状态: 用户按 `Ctrl+P` 触发模型选择,处于 overlay 模式,可上下选择

### 4.4 权限审批弹窗

```
  Would you like to grant these permissions?

  Reason: need workspace access

  Permission rule: network; read `/tmp/readme.txt`; write `/tmp/out.txt`

› 1. Yes, grant these permissions for this turn (y)
  2. Yes, grant for this turn with strict auto review (r)
  3. Yes, grant these permissions for this session (a)
  4. No, continue without permissions (d)

  Press enter to confirm or esc to cancel
```
> 来源: `bottom_pane/snapshots/...approval_overlay_permissions_prompt.snap`
>
> 使用的 widget: `ApprovalOverlay` (多选项, 粗体/斜体样式)
> 状态: 用户被要求审批权限请求(network+文件读写),有 4 个选项

### 4.5 @mention 弹窗

```
"› @sa                               "
"> Sample Plugin  Plugin with skills and an MCP server                         Plugin"
"  enter insert · esc close · ←/→ switch search modes     [All Results]   Filesystem Only    Plugins "
```
> 来源: `bottom_pane/snapshots/...default_unified_mention_popup.snap`
>
> 使用的 widget: `ChatComposer` (内嵌 mention popup, 含 tab 切换)
> 状态: 用户输入 `@sa`,正在搜索匹配的插件/文件/skill

### 4.6 网络审批弹窗

```
  Do you want to approve network access to "example.com"?

  Reason: network request blocked

› 1. Yes, just this once (y)
  2. Yes, and allow this host for this conversation (a)
  3. Yes, and allow this host in the future (p)
  4. No, and tell Codex what to do differently (esc)

  Press enter to confirm or esc to cancel
```
> 来源: `bottom_pane/snapshots/...network_exec_prompt.snap`
>
> 使用的 widget: `ApprovalOverlay` (带 Buffer::Debug 级别的 style 信息)
> 状态: 用户被要求审批网络访问权限

## 5. 对 xylitol 的启示

### 5.1 可直接复用的积木 (ratatui-core/widgets 已提供的)

由于 xylitol 使用 `ratatui-core` + `ratatui-widgets`(子 crate 拆分发),而 codex 使用 umbrella `ratatui`,两者共享相同的 core API:

| 积木 | 说明 |
|------|------|
| **`ratatui_core::buffer::Buffer`** | 渲染缓冲 — 完全一致 |
| **`ratatui_core::layout::Rect/Layout/Constraint/Offset/Position/Size`** | 布局 — 完全一致 |
| **`ratatui_core::backend::Backend` trait + `TestBackend`** | 后端抽象 + 测试缓存 — 完全一致 |
| **`ratatui_core::style::Style/Stylize`** | 样式 API — 完全一致 |
| **`ratatui_core::text::Line/Span/Text`** | 文本 — 完全一致 |
| **`ratatui_widgets::Paragraph/Block/Clear/Wrap`** | 基础 widgets — 完全一致 |
| **`ratatui_core::widgets::WidgetRef`** | WidgetRef trait — 完全一致 |
| **`ratatui_core::terminal::Terminal`** | Terminal struct — 完全一致 |

### 5.2 必须自建的 widget/架构

xylitol 无法直接引用 codex 的代码,但完全可以**移植设计模式**:

| 组件 | 理由 | 设计参考 |
|------|------|---------|
| **`Renderable` trait** | ratatui 没有原生 height negotiation | 仿照 `renderable.rs` 实现 `desired_height()` + `cursor_pos()` |
| **`FlexRenderable`** | ratatui 没有原生 flex 布局 | 仿照, 传入 `(flex_grow, Renderable)` 对 |
| **`BottomPaneView` trait** | View 栈的关键抽象 | 仿照, 直接对应 xylitol 的弹窗需求 |
| **`StreamController` / `StreamCore`** | 双区域流式渲染核心算法 | 仿照, 特别是 `stable_region`/`tail_region` 分区和 `commit_tick` |
| **`MarkdownStreamCollector`** | 增量 markdown 解析 | 仿照处理逻辑,但 markdown parser 可以换 |
| **`ChatComposer`** | 多行输入+mention+paste burst | 核心输入组件,设计最值得移植 |
| **`HistoryCell` enum system** | 消息类型枚举 + 每类型渲染 | 直接移植设计模式 |
| **`custom_terminal::Terminal`** | OSC 8 超链接 + 视图区 + 行插入 | 如果需要超链接或行插入,必须自建 |
| **`ApprovalOverlay`** | 审批弹窗 | xylitol 需要类似的工具执行审批 |
| **`StatusIndicatorWidget`** | 任务进行中 spinner | 基本但重要的 widget |
| **`Shimmer`** | 呼吸闪烁效果 | 如果要显示处理中效果 |

### 5.3 测试 Harness 可借鉴的模式

| 模式 | codex 做法 | xylitol 可借鉴度 |
|------|-----------|-----------------|
| **Snapshot 测试** | `insta::assert_snapshot!` + `TestBackend` | **极高** — 完全一样的模式,rataui core 提供了 TestBackend |
| **VT100 后端** | `CrosstermBackend<vt100::Parser>` 包装 | **高** — 如果需要验证换行/光标位置/CJK |
| **AppEvent 通道驱动** | 通过 channel 注入 `AppEvent` 而不是模拟按键 | **极高** — 更可控,E2E 更稳定 |
| **离线模型/配置** | `LazyLock<Vec<ModelPreset>>` + `ConfigBuilder` | **高** — 避免网络依赖 |
| **组件隔离测试** | 构造单一 widget → draw → snapshot | **极高** — 直接适用 |
| **错误路径测试** | 直接构造 `TurnError` 等协议结构体 | **极高** — 无需 mock 完整 server 行为 |
| **resize 测试** | `VT100Backend` + `set_size()` → 验证 reflow | **中** — 如果需要终端 resize 验证 |
| **EventBroker 注入** | 测试用方法直接 push `TuiEvent` | **中** — EventBroker 本身轻量 |

### 5.4 风险/陷阱

1. **ratatui 版本差异**: codex 使用 `0.29.0` (patched fork), xylitol 使用更新的版本或不同的 fork。检查 `WidgetRef`/`scrolling-regions`/`unstable-backend-writer` 等 API 在 xylitol 版本是否可用。
   - 风险等级: **高** — 特别是 codex 依赖的 `unstable-*` feature 是 Nightly API

2. **codex 的 `custom_terminal` 是 fork 扩展**: `custom_terminal.rs` 改写了 `ratatui::Terminal`,增加了 `set_viewport_area()`, `insert_history_lines()`, OSC 8 hyperlink 支持。这是核心差异化代码,不能直接复用。
   - 风险等级: **高** — 如果 xylitol 需要历史行插入或超链接功能

3. **`Renderable` + `FlexRenderable` 是自定义架构**: codex 没有使用 ratatui 的标准 `Constraint` 布局,而是自建了一套高度协商系统。这增加了跨 crate 复用难度。
   - 风险等级: **中** — 只是设计模式参考

4. **测试依赖 `codex_core`**: TUI 测试严重依赖 `legacy_core`(即 `codex-core` 的配置/模型抽象)。xylitol 需要构建自己的 mock 层。
   - 风险等级: **中**

5. **`vt100::Parser` 的兼容性**: codex 使用 `vt100` crate 来 mock 终端屏幕。xylitol 需要确认此 crate 的兼容性。
   - 风险等级: **低** — `vt100` 是成熟 crate

6. **Streaming 控制的复杂性**: `StreamController` 有 1800+ 行实现,包含 resize reflow、table holdback、commit animation queue 等复杂逻辑。简单移植会有大量工程工作。
   - 风险等级: **高** — 需要认真评估 xylitol 是否真的需要流式控制,或可以使用简化的逐行追加

7. **codex 的代码量**: TUI crate 约 177K 行 Rust,其中测试约 20K+ 行。这只是"可借鉴的设计",不应尝试"搬代码"。
   - 风险等级: **中** — 注意不要 scope creep

8. **proc-macro 使用限制**: codex 使用 `ratatui-macros`。xylitol 如果使用子 crate 拆分发,需确认 macro 可用性。
   - 风险等级: **低**

### 关键结论

**最低可行移植路径** (按优先级):

1. **Snapshot 测试 harness** — 直接用 `TestBackend` + `insta`,只需几行代码
2. **`Renderable` trait** — 核心自定义抽象,约 50 行 trait 定义
3. **`BottomPaneView` trait + 栈架构** — 优雅的弹窗管理,约 50 行 trait + 100 行栈管理
4. **`ChatComposer` 简化版** — 多行文本输入 + paste burst + mention 弹出,约 500 行核心
5. **`HistoryCell` enum 系统** — 消息类型枚举 + 每个类型独立的 render 方法
6. **`StreamController` 简化版** — 如果需要流式渲染,从简单的 `StreamState` 开始
7. **`ApprovalOverlay`** — 基于 `ListSelectionView` + `Renderable` 构建
