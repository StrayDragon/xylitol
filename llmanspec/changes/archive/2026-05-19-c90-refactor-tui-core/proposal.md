---
depends_on: []
---

# c90-refactor-tui-core

## Why

当前 TUI 实现 (`src/interface/tui/`, 11 文件 ~2265 行) 是一次性提交 (9805c1d) 的产物，存在**架构性缺陷**：

- **无组件抽象**：各组件是 ad-hoc struct，无共享 trait、无 dirty flag，每次 `render()` 全量重绘
- **O(n²) 渲染**：每帧对所有消息重新解析 markdown → 重新构造 `Line` 对象
- **手写 Markdown**：`find()` 关键字匹配，不支持列表/链接/blockquote，代码块未闭合时不渲染
- **单行输入**：无 multi-line、无 readline 快捷键 (`Ctrl+K/U/W`)、无历史搜索
- **覆层未接入**：`ApprovalOverlay`/`DiffPreview` 实现了但从未被 App 调用
- **选择器是桩**：session/model/theme 均返回 `["default"]`
- **Session 硬编码**：无切换能力
- **无异步输入**：Agent 运行时输入被禁用
- **鼠标开启但不用**：`EnableMouseCapture` 调了但不处理事件
- **死依赖**：`termimad` 列入但从未 import

**`diff_review/` 模块 (815+843 行) 有良好结构和测试，保持不动。**

### 目标体验（参考 Claude Code / pi coding agent / codex）

```mermaid
flowchart TD
    subgraph "理想 TUI 交互流"
        A[启动 xylitol] --> B[显示主界面]
        B --> C[用户输入 prompt]
        C --> D[Agent 开始执行]
        D --> E{状态}
        E --> F[流式渲染 markdown]
        E --> G[显示 tool call 卡片]
        E --> H[显示 thinking 块]
        F & G & H --> I[Step 完成]
        I --> J[用户可随时输入下一个 prompt]
        J --> C
    end

    subgraph "渲染质量"
        K[仅渲染 dirty 组件] --> L[同步输出 CSI ?2026h/l]
        L --> M[30fps 帧率限制]
        M --> N[流畅无闪烁]
    end
```

## What Changes

### 架构概览

```mermaid
classDiagram
    class Component {
        <<trait>>
        +render(frame, area)
        +is_dirty() bool
        +mark_clean()
        +handle_event(event)
    }

    class OverlayStack {
        +push(Box~Component~)
        +pop()
        +route_key(key) bool
        +render_all(frame, area)
    }

    class App {
        +chat: ChatComponent
        +input: InputComponent
        +markdown: MarkdownRenderer
        +status_bar: StatusBar
        +overlays: OverlayStack
        +running: bool
        +queued_prompts: Vec~String~
        +update(event) Action
        +render(frame)
    }

    App *-- ChatComponent : owns
    App *-- InputComponent : owns
    App *-- MarkdownRenderer : owns
    App *-- StatusBar : owns
    App *-- OverlayStack : owns
    ChatComponent ..|> Component : implements
    InputComponent ..|> Component : implements
    StatusBar ..|> Component : implements
```

### 事件循环

```mermaid
flowchart TD
    subgraph async_events["tokio::select! 事件源"]
        A[agent_rx: AgentEvent 通道]
        B[key_rx: crossterm 键盘事件]
        C[tick: 250ms 帧定时器]
    end

    D{select!} --> A
    D --> B
    D --> C

    A --> E["App::handle_agent(event)"]
    E --> F["App::update() → Action"]
    F --> G["执行 Action\n(启动agent/中断/清屏)"]

    B --> H["转换 crossterm::Event → KeyEvent"]
    H --> I{"OverlayStack 已激活?"}
    I -->|"是"| J["路由到当前覆层"]
    I -->|"否"| K["App::handle_key(key)"]
    K --> F

    C --> L["仅触发重绘"]

    F & G & L --> M["检查 agent 是否完成"]
    M --> N["terminal.draw(f)\n只渲染 dirty 组件"]
    N --> D
```

### 渲染管线

```mermaid
flowchart LR
    A[State Change] --> B[标记对应 Component dirty]
    B --> C[terminal.draw 前]
    C --> D{"CSI ?2026h/l\n同步输出支持?"}
    D -->|"是"| E["BeginSynchronizedUpdate"]
    D -->|"否"| F["直接 draw"]
    E --> G["遍历组件\n跳过 clean 组件"]
    F --> G
    G --> H["每个 dirty 组件\nrender(frame, area)"]
    H --> I["标记 clean"]
    I --> J["EndSynchronizedUpdate" / "flush"]
    J --> K["完成"]
```

### 覆层路由

```mermaid
flowchart TD
    A[键盘事件] --> B{OverlayStack 非空?}
    B -->|"是"| C["取顶层覆层"]
    C --> D["route_key(key) → consumed?"]
    D -->|"consumed=true"| E["覆层处理事件"]
    D -->|"consumed=false"| F["传递到下一层"]
    E --> G{"覆层返回\nOverlayAction::Dismiss?"}
    G -->|"是"| H["pop 覆层"]
    G -->|"否"| I[保留状态]
    B -->|"否"| J["App::handle_key()\n正常模式"]
```

### 组件树与布局

```mermaid
block-beta
    columns 1
    block["App (root)"]
        block:header["Header (1 行)"]
        block:chat["ChatComponent (flex)"]
            block:msgs["Message 列表"]
            block:toolcards["Tool 卡片 (可折叠)"]
            block:think["Thinking 块 (可折叠)"]
        end
        block:input["InputComponent (3-15 行, auto-expand)"]
        block:status["StatusBar (1 行)"]
    end
    block:overlay["OverlayStack (z-order)"]
```

### 技术栈

| Crate | 用途 | 状态 |
|-------|------|------|
| `ratatui 0.29` | TUI 框架 | 已有 |
| `crossterm 0.28` | 终端后端 | 已有 |
| `syntect 5` | 代码语法高亮 | 已有 |
| `pulldown-cmark 0.12` | **新增** — CommonMark 解析 → ratatui Spans | 替换手写解析器 |
| `tui-textarea 0.7` | **新增** — 多行输入 + readline 快捷键 + undo/redo | 替换手写 InputComponent |
| ~~termimad 0.34~~ | **删除** — 从未被引用 | 移除死依赖 |

### 文件清单

**删除 (整个 `src/interface/tui/`)：**
- `mod.rs`, `app.rs`, `chat.rs`, `input.rs`, `markdown.rs`, `tool_output.rs`, `approval.rs`, `diff_preview.rs`, `help.rs`, `selectors.rs`, `status_bar.rs`, `event.rs`

**新建 (~16 文件)：**
```
src/interface/tui/
  mod.rs             模块根, 导出 run_tui()
  app.rs             App 结构体 + event loop + Layout
  component.rs       Component trait + OverlayStack
  event.rs           TuiEvent + AppAction 枚举
  chat.rs            ChatComponent: 消息列表, 流式文本, 滚动
  input.rs           InputComponent: 包裹 tui-textarea
  markdown.rs        MarkdownRenderer: pulldown-cmark → ratatui Lines
  status_bar.rs      StatusBar: 模型/token/cost/上下文
  overlays/
    mod.rs           覆层栈管理
    help.rs          键盘快捷键一览
    selector.rs      通用模糊选择器
  history.rs         持久化输入历史 (~/.xylitol/history)
  slash.rs           斜杠命令解析 + tab 补全
```

**修改：**
- `Cargo.toml` — 添加 `pulldown-cmark`, `tui-textarea`；从 `ui-tui` feature 移除 `termimad`

**保持不动：**
- `src/interface/diff_review/` — 完整保留
- `src/interface/mod.rs`, `print.rs`, `acp.rs` — 不动
- `src/agent/` — 不动

## Capabilities

- `ui-tui`: 重写后的完整 TUI 模式，含 Component trait、多行输入、异步输入队列、pulldown-cmark、读行快捷键、持久化历史、斜杠命令、鼠标支持、同步输出防闪烁

## Impact

- 新增 2 个依赖：`pulldown-cmark 0.12`, `tui-textarea 0.7`
- 删除 1 个死依赖：`termimad` (从未被引用)
- -> `ui-tui` feature gate 保持独立
- 不影响 `ui-review`、`infra-acp` 等其他 feature
