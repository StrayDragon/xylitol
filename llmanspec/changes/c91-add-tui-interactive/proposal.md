---
depends_on: [c90-refactor-tui-core]
---

# c91-add-tui-interactive

## Why

c90 建立了 Core TUI 的基础架构（Component trait、markdown、输入、渲染管线），但缺少交互层的关键能力：

- Agent 运行时用户无法输入（输入被禁用）
- 切换模型/会话/主题需退出 TUI 修改配置
- 无模糊搜索、无外部编辑器集成
- 纯键盘操作缺少鼠标辅助
- 焦点切换无视觉反馈

```mermaid
flowchart TD
    subgraph "c91 交互增强"
        A[Async Queue] -->|"Agent 运行时\n可继续输入"| B[消息排队]
        B --> C[Agent 完成后\n自动提交]
    end

    subgraph "选择器"
        D["/model /session /theme"] --> E[读取 AppConfig]
        E --> F[模糊搜索列表]
        F --> G[切换生效]
    end

    subgraph "编辑增强"
        H["Ctrl+G"] --> I["$EDITOR 打开"]
        I --> J[保存后读回输入]
        K["Ctrl+R"] --> L["模糊历史搜索"]
        L --> M[选择后加载到输入]
    end

    subgraph "鼠标 & 视觉"
        N[滚动滚轮] --> O[Chat 滚动]
        P[点击区域] --> Q[切换焦点]
        R[Tab 切换] --> S[边框高亮]
    end
```

## What Changes

### 1. 异步输入队列

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Running : Submit prompt / Enter
    Running --> Queued : 用户输入 + Enter (agent 运行中)
    Queued --> Running : 当前 step 完成 → 自动提交排队消息
    Running --> Idle : 所有排队消息消费完 / Ctrl+C
    Queued --> QuitPrompt : 队列非空时 Ctrl+C
    QuitPrompt --> Queued : 保留队列
    QuitPrompt --> Idle : 清空队列
```

### 2. 选择器 + 真实配置数据

```mermaid
sequenceDiagram
    participant User
    participant TUI as App
    participant Sel as SelectorOverlay
    participant Cfg as AppConfig
    participant SS as SessionService

    User->>TUI: /model
    TUI->>Cfg: read agent.profiles keys
    Cfg-->>TUI: [gpt-4o, claude-opus-4, ...]
    TUI->>Sel: push with model list
    Sel->>User: render list
    User->>Sel: type fuzzy search
    Sel->>User: filter results live
    User->>Sel: Enter to select
    Sel->>TUI: update ResolvedProfile
    TUI->>User: status bar updates model name

    User->>TUI: /session
    TUI->>SS: list_sessions()
    SS-->>TUI: [session-1, session-2, ...]
    TUI->>Sel: push with session list
    User->>Sel: select
    TUI->>SS: load session history
    TUI->>Chat: display session messages
```

### 3. 文件清单（相对于 c90 新增/修改）

**修改 `src/interface/tui/`：**
- `app.rs` — 异步队列逻辑、/model /session 路由、鼠标事件、焦点高亮
- `input.rs` — Ctrl+G 编辑器启动、Ctrl+R 历史搜索触发、斜杠命令路由
- `status_bar.rs` — 队列指示器 `[Q: N]`
- `overlays/selector.rs` — 接收真实 AppConfig 数据；模糊搜索

**新增：**
- `src/interface/tui/overlays/history_search.rs` — Ctrl+R 交互式历史搜索覆层

### 4. 按键补充

| 按键 | 动作 |
|------|------|
| 鼠标滚轮 | Chat 滚动 |
| 鼠标左键点击 | 切换焦点到点击区域 |
| Tab | 循环切换焦点 (Chat → Input → Status → ...) |
| `Ctrl+G` | 在 `$EDITOR` 中编辑当前输入 |
| `Ctrl+R`（输入模式） | 打开交互式历史搜索覆层 |
| `Ctrl+Up/Down` | 调整 Chat/Tool 区域大小 |

## Capabilities

- `ui-tui`: 异步输入队列、真实数据选择器、鼠标交互、焦点高亮、编辑器集成、历史搜索

## Impact

- 仅修改 `src/interface/tui/`（c90 已建好的文件）
- `diff_review/` 不受影响
- 无新增依赖
