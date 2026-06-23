# xylitol TUI PRD — 高性能流式 Agent 终端界面

> 状态: Draft v1
> 适用范围: `src/interface/tui/`（新增）
> 依赖前置: c225（core 层）已完成；无新增分层依赖

## 1. 背景与目标

### 1.1 背景

xylitol 已完成 pi-mono 核心的 Rust 对齐（25 模块，454 lib 测试 + 83 BDD）。`_HANDOFF.md` 明确 **TUI 不在移植范围内**。现有 UI 只有:

- `print.rs` — 非交互，流式输出到 stdout
- `rpc.rs` — JSONL-over-stdio RPC（供编辑器集成）
- `diff_review/cli.rs` — 843 行基于 ratatui 的全屏 diff 审查（immediate-mode，同步 `event::poll` 循环）

缺失交互式 agent TUI（对话流 + 工具调用展示 + 输入编排）。本 PRD 定义这个 TUI 的设计。

### 1.2 目标

| # | 目标 | 衡量标准 |
|---|---|---|
| G1 | 流式不卡顿 | 1000-token assistant 回复期间，终端写入保持在 60fps 节流内，无明显延迟尖峰 |
| G2 | 不重复造轮子 | 渲染/差分/键盘解析/图片协议/editor 全部用生态 crate，自研代码集中在"xylitol 特有"层 |
| G3 | 可扩展 | 扩展能注入 footer widget、自定义消息渲染器、自定义 editor，通过 `Box<dyn Component>` + Action 回发 |
| G4 | 对齐 Codex 交互范式 | 单列 transcript + 底部 composer，Esc/Esc 回退、Tab 排队、bang-shell 特例 |
| G5 | 可测试 | composer 逻辑、键盘绑定、markdown 缓存、事件 reduce 全部可纯单测，不依赖真实终端 |

### 1.3 非目标

- 不移植 pi-tui 的字符串行差分引擎（ratatui Buffer diff 已覆盖）
- 不实现 pi-tui 的全部 12 个组件（只实现 agent TUI 真正需要的）
- 不做 Web UI（`diff_review` 的 Web backend 是独立议题）
- 不引入 JS/TS 扩展运行时（与 pi 的扩展系统不兼容是设计选择，见 §10.4）

## 2. 关键设计决策（先读这一节）

### 2.1 两层 retained，而非全局组件框架

```
┌────────────────────────────────────────────────┐
│  Layer A — 会话状态（领域对象，常驻）           │
│  Transcript / Composer / FocusCtx / ToolTree   │  ← 唯一真正的 retained state
└──────────────────┬─────────────────────────────┘
                   │ 每帧纯函数投影
┌──────────────────▼─────────────────────────────┐
│  Layer B — 视图层                               │
│  ┌──────────────────────────────────────────┐  │
│  │ MarkdownView（retained 排版缓存）         │  │  ← 唯一真正的 retained widget
│  │ Transcript / Composer / StatusBar / Overlays │ ← 其余 immediate
│  └──────────────────────────────────────────┘  │
└────────────────────────────────────────────────┘
                   │
         ratatui Buffer diff（免费，cell 级）
                   │
              终端字节流
```

**决策**: 不要求所有组件实现 `Component { is_dirty, mark_clean }`。Layer A 状态常驻（它本来就是领域对象），Layer B 视图每帧从 Layer A 全量投影，markdown 缓存是例外。dirty flag 只控制"是否调度一次 redraw"，不控制"frame 内画什么"。

**理由**: pi-tui 自研字符串 diff 是因为它跑在没有差分能力的裸 stdout 上。ratatui 的 `Terminal::draw` 已经持有上一帧 Buffer 做 cell 级 diff。在 ratatui 之上再造一套组件级 dirty diff，是把差分层做了两遍，还引入"某组件忘 mark dirty 就不画"的画面撕裂风险。

### 2.2 事件 reduce，而非组件持有 agent 引用

TUI 不是"组件树直接访问 agent"，而是把两个事件源 reduce 成 Layer A 状态:

```
AgentEvent stream (AgentLoop::run → impl Stream<Item=AgentEvent>)
        │
        ▼
   Transcript::apply(evt)   ← 纯函数状态机
        │
        ▼
   render::project(&transcript) → ratatui widgets
```

**理由**: `print.rs` 已经是这个模式的纯文本实现（`while let Some(event) = stream.next()`）。TUI 只是把投影从"write 到 stdout"换成"画到 ratatui"。这让 Layer A 可纯单测（喂 `Vec<AgentEvent>`，断言 `Transcript` 状态），与终端完全解耦。

### 2.3 单 select! 循环，三路输入

```rust
loop {
    tokio::select! {
        Some(evt) = agent_rx.recv() => { transcript.apply(evt); schedule_redraw(); }
        evt = input_rx.recv()       => { dispatch(input_layer.translate(evt)?); schedule_redraw(); }
        _ = frame_timer.tick()      => { if dirty { draw(&mut terminal, &app)?; dirty = false; } }
    }
}
```

agent 事件和终端输入都通过 channel 进入同一循环；`schedule_redraw` 只置 `dirty: bool`；真正的 draw 在 frame_timer 分支（节流到 ~60fps）。

**理由**: 现有 `diff_review/cli.rs` 用同步 `event::poll(50ms)` 阻塞循环，无法与 async agent stream 共存。新 TUI 必须 async。三路 select 是 async TUI 的标准形态。

## 3. 生态复用分析（避免重复造轮子）

| 能力 | pi-tui 自研成本 | Rust 生态方案 | 复用程度 |
|---|---|---|---|
| 终端渲染 + Buffer 差分 | `tui.ts` ~600 行 | **ratatui 0.30**（已选） | 完全复用，零自研 |
| 终端原始 IO / raw mode / alt screen | `terminal.ts` ~200 行 | **crossterm 0.29**（已选，含 `event-stream`） | 完全复用 |
| **Kitty keyboard protocol** | `keys.ts` 1400 行 | **crossterm 0.29** 内置 `PushKittyKeyboardEnhancementFlags` + 解析后的 `KeyEvent` | 高度复用；只剩"序列拼接 / context 绑定"自研 |
| **stdin 批次重组** | `stdin-buffer.ts` 434 行 | crossterm `EventStream` 已逐事件投递 | 完全复用，零自研 |
| **CJK 宽度** | `get-east-asian-width` | **unicode-width** crate | 完全复用 |
| **图素分段** | 自研 | **unicode-segmentation** crate | 完全复用 |
| **多行编辑器** | `editor.ts` 2307 行 | **tui-textarea**（word nav / undo / kill ring / 选区） | 高度复用；xylitol 特有的 queue/esc/bang 语义做成 wrapper |
| **模糊匹配** | `fuzzy.ts` 137 行 | **fuzzy-matcher** crate | 完全复用 |
| **内联图片（Kitty/iTerm2）** | `terminal-image.ts` 488 行 | **ratatui-image** crate | 完全复用 |
| **同步输出（CSI 2026）** | 自研 | crossterm `BeginSynchronizedOutput`/`EndSynchronizedOutput` | 完全复用 |
| **Markdown 解析** | `marked` | **pulldown-cmark**（spec 已定） | 完全复用 |
| **语法高亮** | 无 | **syntect**（已是可选 dep） | 复用 |
| **Markdown → ratatui Line** | `markdown.ts` 852 行 | 无 turnkey 方案；自研 thin renderer | **部分自研**（见 §6.3） |
| 终端能力探测 / OSC 11 取色 | `terminal-colors.ts` 73 行 | 自研（简单） | 自研 |
| Apple Terminal / Termux / Windows VT quirks | `terminal.ts` 散布 | 部分靠 crossterm；少数需自研补丁 | 多数复用 |

**净结论**: pi-tui 12K 行里，**~80% 的能力在 Rust 生态有成熟 crate 直接覆盖**。真正需要自研的是:
1. Markdown → 样式化 ratatui `Line<'static>` 的渲染器（无 turnkey crate 满足 agent 场景）
2. xylitol 特有的交互编排（Tab 排队、Esc 回退、bang-shell、approval overlay 接线）
3. Layer A 的 `Transcript` 状态机（xylitol 独有，无对应 crate）

自研代码预计 **2000–3000 行**（含测试），对比 pi-tui 的 12K。

## 4. 模块结构

```
src/interface/tui/
├── mod.rs              // run_tui() 入口；select! 循环；terminal setup/restore
├── state/
│   ├── mod.rs          // App 聚合根（持有 Transcript/Composer/FocusCtx）
│   ├── transcript.rs   // AgentEvent → 状态 reduce；消息树；工具调用节点
│   ├── composer.rs     // 输入缓冲 + 排队 + bang-shell 标记
│   └── focus.rs        // 焦点上下文（Composer/Transcript/Overlay）
├── render/
│   ├── mod.rs          // project(&App) → 布局 + widget 组装
│   ├── transcript.rs   // 消息列表投影（含工具卡片、思考块）
│   ├── markdown.rs     // MarkdownView：retained 排版缓存（唯一 retained widget）
│   ├── composer.rs     // 底部输入区投影
│   ├── statusbar.rs    // 单行 Codex 风格 footer
│   └── overlay.rs      // OverlayStack + 已有 diff_review 复位为 overlay
├── input/
│   ├── decode.rs       // Layer 1: crossterm Event → InputKey（kitty 增强补丁）
│   ├── keymap.rs       // Layer 2: (InputKey, FocusCtx) → Option<Action>
│   └── action.rs       // Layer 3: Action enum + apply(&mut App, &mut AgentHandle)
├── ext/
│   └── component.rs    // pub trait Component（扩展契约，见 §9）
└── theme.rs            // Codex 调色板解析（OSC 11 + 用户 theme 覆盖）
```

**依赖方向硬约束**（对齐 c225 分层）:
- `state/` 只依赖 `crate::core` + `crate::agent::loop::AgentEvent`，**不依赖 ratatui**
- `render/` 依赖 `state/` + ratatui
- `input/` 依赖 `state/`（通过 Action）+ crossterm
- `ext/` 依赖 ratatui + `state/`（通过 Action sink）

这保证 `state/` 和 `input/keymap` 可纯单测，不需要终端。

## 5. 事件流与状态机

### 5.1 Transcript 状态机

```rust
pub struct Transcript {
    pub entries: Vec<TranscriptEntry>,
    pub last_streaming_idx: Option<usize>,   // 正在流式的那条
    pub tool_nodes: HashMap<String, ToolNode>, // 按 id 索引
    pub stats: Stats { tokens, turns, ... },
}

pub enum TranscriptEntry {
    User { text: String, images: Vec<...> },
    Assistant { view: MarkdownView, thinking: Option<MarkdownView> },
    ToolCall { id: String, node: ToolNode },
    Compaction { summary: String },
    Error { msg: String },
}

pub struct ToolNode {
    pub id: String,
    pub name: String,
    pub args: Value,
    pub status: ToolStatus,           // Running / Success / Failed
    pub output: String,
    pub expanded: bool,
}

impl Transcript {
    /// 纯函数：消费一个 AgentEvent，更新状态。无 IO。
    pub fn apply(&mut self, evt: AgentEvent) { ... }
}
```

`AgentEvent` 到状态变更的映射:

| AgentEvent | Transcript 变更 |
|---|---|
| `TurnStart { .. }` | 计数器 +1 |
| `MessageStart { role: "assistant" }` | push `Assistant { view: MarkdownView::empty(), thinking: None }`，记 `last_streaming_idx` |
| `TextDelta(s)` | `entries[last].view.append(s)`（内部失效缓存） |
| `ThinkingDelta(s)` | 确保 `thinking` 存在，append |
| `MessageEnd { .. }` | 清 `last_streaming_idx` |
| `ToolExecutionStart { id, name, args }` | push `ToolCall`，`tool_nodes[id] = Running` |
| `ToolExecutionUpdate { id, output }` | `tool_nodes[id].output += output` |
| `ToolExecutionEnd { id, result, .. }` | `tool_nodes[id]` = Success/Failed |
| `CompactionStart/End` | push `Compaction` 条目 |
| `Error(msg)` | push `Error` |
| `AgentEnd { .. }` | 清流式标记，最终统计 |

**第二个事件源**（`EventBus` 的 `AgentLifecycleEvent`）用于: approval 触发、queue 计数显示、auto-retry 进度。TUI 通过 `session.subscribe()` 订阅，转成 channel 喂入同一 select!。`AgentEvent`（stream）和 `AgentLifecycleEvent`（bus）职责区分:
- stream = **内容流**（要画进 transcript 的）
- bus = **控制信号**（不直接画进 transcript，但影响 footer/overlay/审批）

### 5.2 Composer 状态机

```rust
pub struct Composer {
    pub buf: String,          // 或 tui-textarea 的 TextArea（见 §6.2）
    pub queue: VecDeque<String>,
    pub bang_shell: bool,     // 首字符 '!'
}

pub enum ComposerAction {
    Type(char),
    Submit,                   // Enter（idle 时）
    QueueOrSubmit,            // Tab：running→queue，idle→submit（bang 例外）
    Clear,                    // Esc：清空
    BacktrackPrime,           // EscEsc：装载上一条 user 消息
    ...
}
```

**Codex 语义硬规则**（对齐交互范式）:
- idle + 非 bang + Tab/Enter → submit
- running + Tab → push 进 queue（footer 显示 `[Q:n]`），不 submit
- idle + bang 草稿 + Tab → **不 submit**（bang 草稿只在 Enter 时作为 shell 执行）
- 非空 + Esc → 清空草稿（若 overlay 开着则先关 overlay）
- 空 + Esc → prime backtrack
- 空 + Esc + Esc → 装载最后一条 user 消息进 composer

这些规则全部在 `input/keymap.rs` 用 `(InputKey, FocusCtx, AgentState) → Action` 表达，**纯函数，可穷举单测**。

## 6. 视图层详细设计

### 6.1 整体布局（单列流式）

```
┌─────────────────────────────────────────────┐
│  Transcript（滚动区，占满除底部外全部）        │  ← 投影 entries[]
│  user: ...                                   │
│  assistant: markdown...                      │
│    ▸ bash  ✓  (折叠卡片，Enter 展开)          │
│  ...                                         │
├─────────────────────────────────────────────┤
│  composer（底部，动态高度，2–8 行）            │  ← tui-textarea
├─────────────────────────────────────────────┤
│  footer（单行：running/idle | model | Q:n）    │  ← Codex 风格
└─────────────────────────────────────────────┘
```

不画 `Borders::ALL` box（对齐 Codex 视觉约束，footer r35/r36）。仅用 default/dim/cyan/magenta 调色板。

### 6.2 Composer 渲染

用 **tui-textarea** 的 `TextArea` 作为 composer 内部缓冲。它自带:
- 多行、Shift+Enter 换行
- readline 快捷键（Ctrl+K/U/W/A/E）
- word navigation、undo、选区、yank

xylitol 特有语义（queue/bang/esc）**不进 TextArea**，而是由 `input/action.rs` 在 dispatch 层决定，再调用 `TextArea::insert_yanked_char` 等。TextArea 只管文本编辑，编排逻辑在 wrapper。

### 6.3 MarkdownView（唯一 retained widget，性能关键）

```rust
pub struct MarkdownView {
    source: String,
    layout: Option<MarkdownLayout>,  // 失效则重算
    width_cache: u16,                 // 上次排版宽度
}

struct MarkdownLayout {
    lines: Vec<Line<'static>>,        // 已染色、已切行
    code_lang: Vec<Option<String>>,   // 供 syntect 延迟高亮
}

impl MarkdownView {
    pub fn append(&mut self, delta: &str) {
        self.source.push_str(delta);
        self.layout = None;           // 失效
    }
    pub fn lines(&mut self, width: u16, theme: &Theme) -> &[Line<'static>] {
        if self.layout.is_none() || self.width_cache != width {
            self.layout = Some(render_md(&self.source, width, theme));
            self.width_cache = width;
        }
        self.layout.as_ref().unwrap().lines.as_slice()
    }
}
```

`render_md` 流水线: `pulldown-cmark` 解析 → 遍历 event 流 → 按样式决策表（标题 bold + magenta / 行内代码 dim bg / 链接 cyan underline / 列表符号 / 引用块 dim prefix）生成 `Span` → 按宽度切行（用 `unicode-width`）→ 代码块交给 `syntect` 高亮（可异步/延迟，避免首帧阻塞）。

**样式决策表**是 pi-tui `markdown.ts` 852 行里唯一值得参考的部分（颜色规则），不是代码。

**性能特性**:
- 流式 `TextDelta` 只让最后一条 assistant 的 `MarkdownView` 失效
- 滚动时宽度不变 → 缓存命中，不重排
- 帧率节流（draw 在 frame_timer 分支）保证 markdown 最多按 60fps 重排，不按 token 速率

### 6.4 工具调用卡片

每个 `ToolCall` entry 投影为一个折叠块:

```
  ▸ bash   ✓   3 lines hidden          (折叠)
  ▾ edit   ●running                     (展开)
      args: { path: "...", ... }
      output: ...
```

spinner / 勾 / 叉 由 `ToolStatus` 决定。Enter 在选中工具卡片时 toggle `expanded`。卡片是 transcript 列表的一项，**不是独立面板**（对齐 G4 单列约束）。

### 6.5 OverlayStack

```rust
pub struct OverlayStack {
    layers: Vec<Layer>,  // z-order，末尾在最上
}
struct Layer {
    kind: OverlayKind,   // Help / Selector / Approval / DiffPreview / Transcript / HistorySearch
    rect: Rect,          // 每帧重算（随终端尺寸）
    capturing: bool,     // 是否独占键盘焦点
}
```

overlay 是 immediate widget（每帧重画），只在"激活时"参与渲染。键盘焦点路由: `select!` 输入分支先问 `overlay_stack.top_capturing()`，捕获则把 key 喂给该 overlay 的 handler，否则走 composer。

**diff_review 复位**: 现有 `diff_review/cli.rs` 的全屏 review 模式应重构为一个 `Layer`（`OverlayKind::DiffReview`），复用其 `types.rs`（`DiffHunk`/`DiffLine`/`ReviewVerdict`）。`render_app` 那套 widget 代码迁移到 `render/overlay.rs::draw_diff_review`。

## 7. 键盘三层模型

```
raw crossterm Event
   │
   ▼  input/decode.rs (Layer 1)
InputKey { key, mods, kitty_flags }     ← crossterm 已解析 + 少量 kitty 增强
   │
   ▼  input/keymap.rs (Layer 2)
Option<Action>                          ← (InputKey, FocusCtx, AgentState) → Action 查表
   │
   ▼  input/action.rs (Layer 3)
状态变更（Composer / Transcript / overlay / agent handle）
```

**为何三层**:
- Layer 1 的 kitty quirks 不该污染 composer 语义
- Layer 2 的绑定可纯表驱动、可配置、可单测
- Layer 3 的 dispatch 集中处理副作用，便于审计（尤其 approval 这种涉及 agent handle 的）

**RuntimeKeymap**（对齐 pi-tui 的可配置键位）: Layer 2 的查表不是硬编码 `match`，而是 `Keymap` 结构，支持用户配置覆盖（从 `~/.xylitol/keymap.toml` 加载）。默认表 + 用户覆盖合并。绝不硬编码 `if key == Ctrl+X`（对齐项目 AGENTS.md "Never hardcode key checks"）。

## 8. 性能模型

### 8.1 性能预算

| 场景 | 预算 | 手段 |
|---|---|---|
| 单 token delta → 屏幕更新 | < 16ms（60fps 内） | frame_timer 节流；markdown 缓存只失效最后一条 |
| 100 条历史消息滚动 | 无重排 | MarkdownView 按 width 缓存；滚动只改 viewport offset |
| 工具并发输出（5 个并行 bash） | 不互相阻塞 | 每个工具节点独立累积 output，渲染时投影 |
| 大段 paste（>10 行） | 单次更新 | crossterm `bracketed-paste` 已处理（feature 已开） |

### 8.2 渲染节流

`schedule_redraw()` 只置 `dirty = true`，不立即画。`frame_timer` 以 ~16ms tick，检查 dirty 才 `terminal.draw`。这样:
- token 风暴（一帧内多个 delta）只触发一次 draw
- 无事件时不画（CPU 静止）

ratatui Buffer diff 保证 draw 本身只输出变化的 cell，配合 `unstable-backend-writer` 批量写 + `BeginSynchronizedOutput`/`EndSynchronizedOutput` 防闪烁（feature `scrolling-regions` 已开，追加滚动用滚动区，不全屏重绘）。

### 8.3 异步 markdown 高亮

syntect 首次高亮一个大代码块可能 >16ms。策略:
1. 首帧先用无高亮 plain 渲染（保证不卡）
2. 在 frame_timer 之外的 task 异步高亮，完成后 invalidate 对应 `MarkdownView`
3. 缓存高亮结果（按 `code + lang` hash）

## 9. 扩展契约

```rust
/// 扩展可注入的自定义组件。
pub trait Component: Send + 'static {
    fn render(&mut self, f: &mut Frame, area: Rect, ctx: &RenderCtx);
    fn handle_event(&mut self, evt: &InputKey) -> bool { false }
}

pub struct RenderCtx<'a> {
    pub theme: &'a Theme,
    pub width: u16,
    pub action_sink: &'a ActionSink,   // 组件通过它回发 Action，不直接持 agent
}

pub type WidgetFactory =
    Arc<dyn Fn(Arc<Theme>) -> Box<dyn Component> + Send + Sync>;
```

**扩展注入点**（对齐 pi-tui 的 footer/widget/editor 工厂）:
- `register_footer(factory)` — 替换底部 footer
- `register_widget(key, factory, placement)` — 在 composer 上方/下方插 widget
- `register_message_renderer(custom_type, fn)` — 自定义消息渲染
- `register_editor(factory)` — 替换 composer（如 vim 模式 editor）

**两条硬约束**（pi-tui 教训，见 §10.4）:
1. 工厂收 owned 快照（`Arc<Theme>`），不收 `&App`/`&AgentSession`。组件影响 agent 只能通过 `ActionSink` 发 Action。
2. `Box<dyn Component>: Send + 'static`。redraw 在 frame_timer task，跨 await。

## 10. 风险与开放问题

### 10.1 tui-textarea 的 queue 语义

tui-textarea 没有"Tab 在 running 时排队"概念。需在 wrapper 层拦截 Tab，不让它进 TextArea。**风险**: TextArea 的 indent-on-Tab 行为与 Codex Tab-queue 冲突。**缓解**: composer wrapper 在 Layer 2 拦截 Tab，不传给 TextArea。

### 10.2 ratatui-image 的生命周期

pi-tui 在 `terminal-image.ts` 里管理 Kitty placement id 的分配/删除/resize 清理（488 行）。ratatui-image 是否完整覆盖 resize 时的图片清理需 spike 验证。**缓解**: 先在不带图片的 MVP 里跑通，图片作为 §11 phase 3。

### 10.3 crossterm kitty 键盘增强的覆盖度

crossterm 0.29 支持 `PushKittyKeyboardEnhancementFlags`，但 pi-tui `keys.ts` 里有大量终端 quirks（Apple Terminal 的 Shift+Enter、Termux 软键盘、Windows VT input）。**开放问题**: crossterm 在这些终端上的行为是否足够，是否仍需 `decode.rs` 的补丁层。**缓解**: spike 阶段在 macOS Terminal / iTerm2 / kitty / Windows Terminal / Termux 各测一次核心键位。

### 10.4 与 pi 扩展系统不兼容

pi-tui 扩展是运行时加载的 TS 模块，返回"活组件对象"。xylitol 用 Rust trait + 编译期注册，**不兼容 pi 的 TS 扩展**。这是设计选择（xylitol 本就是 Rust 重写），不是缺陷。需在文档明确。

### 10.5 同步 `event::poll` 循环的迁移

`diff_review/cli.rs` 现有 `run_app` 是同步 poll 循环。迁移到 overlay 后需改 async，或保留为独立全屏入口（不走主 select!）。**倾向**: 改 async overlay，统一架构；diff_review 作为 Ctrl+R overlay 进入。

## 11. 实施阶段

| Phase | 范围 | 验收 |
|---|---|---|
| **P0 骨架** | `mod.rs` select! 循环 + terminal setup/restore + 空 transcript/composer/footer。接 `AgentLoop::run`，`TextDelta` 能流式打出 | 能跑 `cargo run --features ui-tui -- tui`，输入回显，流式文本可见 |
| **P1 状态机** | `state/transcript.rs` + `state/composer.rs` 纯函数 + 单测 | `cargo test state` 全绿；喂 `Vec<AgentEvent>` 断言状态 |
| **P2 markdown** | `render/markdown.rs` MarkdownView + pulldown-cmark + 样式表 | 流式 1000-token 不卡（profile 验证）；代码块高亮（延迟异步） |
| **P3 键盘三层** | `input/{decode,keymap,action}.rs` + Codex 语义（Tab/Esc/bang）+ Keymap 配置 | composer 逻辑全单测；keymap 可配置 |
| **P4 工具卡片 + overlay** | 工具节点投影 + OverlayStack + approval overlay 接线（接 `SandboxVerdict`/审批 channel） | 工具调用可视化；edit 工具触发 approval modal |
| **P5 diff_review 迁移** | 现有 `diff_review/cli.rs` 复位为 overlay，复用 `types.rs` | Ctrl+R 进 diff review overlay；旧全屏入口废弃 |
| **P6 图片 + 高级** | ratatui-image 接入；transcript overlay（Ctrl+T）；history search（Ctrl+R 已被 diff 占用，需重新分配，见 §11 注） | 图片在 kitty/iTerm2 显示；overlay 全套 |

> 注: pi-tui 里 Ctrl+R 是 history search，本 PRD 里 diff preview 也想用 Ctrl+R。需在 P3 Keymap 设计阶段统一分配（建议 diff preview = Ctrl+Shift+D，history search = Ctrl+R）。

## 12. 测试策略

| 层 | 工具 | 覆盖 |
|---|---|---|
| `state/` | 纯单测 `#[test]` | AgentEvent reduce、composer 状态机、queue/bang/esc 规则 |
| `input/keymap.rs` | 纯单测 | `(InputKey, FocusCtx, AgentState) → Action` 穷举 |
| `render/markdown.rs` | 单测 + `insta` 快照 | 给定 md 源 → `Vec<Line>` 快照 |
| `render/` 整体 | `insta` 快照（ratatui `Buffer::content` 序列化） | 整帧渲染快照回归 |
| 集成 | tmux 驱动（对齐 pi 的 `pi-test.sh` 范式） | 真实终端交互 e2e |

state 与 keymap 的纯函数性是 G5 的保证，也是回归成本低的关键。

## 13. Feature Flag

新增 feature `ui-tui`（与现有 `ui-review` 平行）:

```toml
[features]
ui-tui = [
    "dep:ratatui",
    "dep:crossterm",
    "infra-syntax",      # syntect for md code highlight
]
```

`ui-review`（diff_review）保留，但 P5 后其渲染代码迁入 `ui-tui` 的 overlay 层；`ui-review` feature 可降级为"diff 采集逻辑"的数据层，渲染依赖 `ui-tui`。或合并为单一 `ui` feature，P5 决定。

`default` 是否包含 `ui-tui` 待定（倾向包含，对齐 `default` 含 `ui-review` 现状）。

## 14. 对比 pi-tui 的取舍总结

| 维度 | pi-tui | xylitol TUI | 取舍理由 |
|---|---|---|---|
| 渲染差分 | 自研字符串行 diff | ratatui Buffer cell diff | 不重复造，且 cell diff 更细 |
| 组件模型 | 全组件 retained + dirty | 仅 markdown retained，余 immediate | ratatui 已有差分，全 retained 冗余 |
| 键盘解析 | 自研 1400 行（含协议解析） | crossterm 解析 + 薄补丁 | crossterm 已覆盖协议 |
| Editor | 自研 2307 行 | tui-textarea + wrapper | 生态成熟 |
| 图片 | 自研 488 行 | ratatui-image | 生态成熟 |
| Markdown | marked + 自研 852 行渲染 | pulldown-cmark + 自研 ~400 行渲染 | 解析复用，渲染自研（无 turnkey） |
| 扩展 | TS 运行时活对象 | Rust trait + 编译期 + Action sink | 语言边界决定，规避借用/Send 地狱 |
| 交互范式 | pi 自有（chat/tools/input 分栏） | Codex 单列 + Tab/Esc 语义 | 对齐 G4，更适合 agent 场景 |

**净自研**: ~2000–3000 行（state 状态机 + markdown 渲染 + 键盘三层 + overlay + 交互编排），对比 pi-tui 12K。复用率约 75–80%。

---

## 附录 A: 与 `tui-interface` 旧 spec 的差异

旧 spec（已要求重设计，本 PRD 替代）的主要偏差:
- r1 要求所有组件实现 Component trait（全 retained）→ 本 PRD 仅 markdown retained（§2.1）
- r27 单层 RuntimeKeymap → 本 PRD 三层（§7）
- r17 chat/tool 分栏 → 本 PRD 单列流式（§6.1）
- 未明确 markdown 性能层 → 本 PRD §6.3 + §8 显式缓存 + 节流
- 未明确扩展生命周期约束 → 本 PRD §9 owned 快照 + Action sink

## 附录 B: 待 Spike 的开放技术问题

1. crossterm 0.29 在 Apple Terminal / Termux / Windows Terminal 的 kitty 键盘行为覆盖度（§10.3）
2. ratatui-image 在终端 resize 时的 placement 清理是否完整（§10.2）
3. tui-textarea 与 Tab-queue 的拦截点（§10.1）
4. `AgentEvent` stream 与 `EventBus` lifecycle 事件的去重（同一 message 可能两边都发）——倾向 stream 为内容权威，bus 仅控制信号
