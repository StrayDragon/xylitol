# PRD: Multi-Agent Workspace — 平铺式多 Agent 协作终端

> 状态: Idea v1
> 优先级: P2 (探索性)
> 预计工作量: 3-4 周
> 依赖: TUI PRD (tui-prd.md) 完成后

## 1. 概述

### 1.1 愿景

将 xylitol 从单一对话式 agent 扩展为**平铺式多 Agent 工作区**，允许用户同时运行多个独立的 agent 实例，每个实例占据一个可调整大小的窗格，支持并行任务处理和实时协作。

### 1.2 核心价值

| 场景 | 当前痛点 | 解决方案 |
|------|----------|----------|
| 多文件重构 | 单 agent 处理慢，上下文切换频繁 | 多个 agent 并行处理不同文件 |
| 代码审查 | 无法同时查看多个 diff | 一个窗格显示 agent 对话，另一个显示 diff |
| 学习模式 | 想同时问多个问题 | 多个对话窗格，各问各的 |
| 复杂调试 | 需要同时查看日志和代码 | agent 窗格 + 日志窗格 + 代码窗格 |

### 1.3 灵感来源

- **ratatui-hypertile** 的平铺窗口管理
- **Zellij** 的终端多路复用
- **tmux** 的窗格分割
- **VS Code** 的多编辑器布局

## 2. 功能设计

### 2.1 核心概念

```
┌─────────────────────────────────────────────────────────────┐
│                    Multi-Agent Workspace                     │
├─────────────────────────┬───────────────────────────────────┤
│                         │                                   │
│   Agent Pane 1          │      Agent Pane 2                 │
│   (Code Refactor)       │      (Code Review)                │
│   ┌─────────────────┐   │   ┌───────────────────────────┐   │
│   │ User: 重构 auth  │   │   │ User: 审查 PR #123       │   │
│   │ Agent: 好的...   │   │   │ Agent: 我来分析...       │   │
│   │ [工具调用中...]   │   │   │ [查看 diff...]           │   │
│   └─────────────────┘   │   └───────────────────────────┘   │
│                         │                                   │
├─────────────────────────┴───────────────────────────────────┤
│   Shared Context Pane (文件树/日志/终端)                      │
├─────────────────────────────────────────────────────────────┤
│  Footer: [Pane 1: running] [Pane 2: idle] [Shared: active]  │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 窗格类型

#### Agent Pane (对话窗格)
- 独立的 agent 会话
- 独立的上下文和工具权限
- 可配置不同的 model/provider
- 支持"临时 agent"模式（一次性任务）

#### Shared Context Pane (共享上下文窗格)
- 文件浏览器
- 日志查看器
- 终端模拟器
- 代码编辑器 (与 agent 联动)

#### Diff Review Pane (差异审查窗格)
- 复用 xylitol 的 diff_review 组件
- 实时显示 agent 的代码修改
- 支持接受/拒绝操作

### 2.3 交互模式

#### 焦点切换
```
Ctrl+1, Ctrl+2, ...  → 切换到指定窗格
Ctrl+Arrow           → 方向切换窗格
Ctrl+Space           → 循环切换窗格
```

#### 窗格操作
```
Ctrl+\               → 垂直分割当前窗格
Ctrl+-               → 水平分割当前窗格
Ctrl+W, Arrow        → 调整窗格大小
Ctrl+W, x            → 关闭当前窗格
Ctrl+W, z            → 最大化/还原当前窗格
```

#### Agent 间通信
```
# 在 Agent 1 中
@agent2 请帮我检查这个函数的性能问题

# 或者使用共享上下文
@shared 将这个文件的内容同步到所有 agent
```

### 2.4 临时 Agent (Ephemeral Agent)

**概念**: 为特定任务创建的一次性 agent，任务完成后自动销毁。

**使用场景**:
```bash
# 快速问答
xylitol ephemeral "Python 的 GIL 是什么？"

# 一次性代码生成
xylitol ephemeral --tool bash "列出当前目录的所有 Python 文件"

# 临时调试助手
xylitol ephemeral --model claude-haiku --max-turns 3 "这个错误是什么意思？"
```

**在 TUI 中的体现**:
- 按 `Ctrl+E` 打开临时 agent 面板
- 输入任务描述
- 自动创建新窗格运行
- 任务完成后窗格变灰，可保留或关闭

## 3. 技术架构

### 3.1 集成 ratatui-hypertile

```rust
// Workspace 管理器
struct AgentWorkspace {
    layout: Hypertile,           // 平铺布局引擎
    panes: HashMap<PaneId, Pane>, // 窗格内容
    active_pane: PaneId,         // 当前焦点窗格
    shared_context: SharedCtx,   // 共享上下文
}

// 窗格类型
enum Pane {
    Agent(AgentPane),
    DiffReview(DiffReviewPane),
    FileExplorer(FileExplorerPane),
    Terminal(TerminalPane),
}

// Agent 窗格
struct AgentPane {
    session_id: String,
    agent_loop: AgentLoop,
    transcript: Transcript,
    composer: Composer,
    config: AgentConfig,
}
```

### 3.2 事件路由

```rust
// 事件分发器
struct EventRouter {
    workspace: AgentWorkspace,
}

impl EventRouter {
    fn route_event(&mut self, event: Event) {
        match event {
            // 全局快捷键
            Event::Key(key) if is_global_shortcut(key) => {
                self.handle_global_shortcut(key);
            }
            // 路由到当前焦点窗格
            Event::Key(key) => {
                if let Some(pane) = self.workspace.active_pane_mut() {
                    pane.handle_input(key);
                }
            }
            // Agent 事件路由到对应窗格
            Event::Agent { pane_id, event } => {
                if let Some(pane) = self.workspace.pane_mut(pane_id) {
                    pane.handle_agent_event(event);
                }
            }
        }
    }
}
```

### 3.3 共享上下文管理

```rust
struct SharedContext {
    // 文件系统状态
    file_tree: FileTree,
    open_files: HashMap<PathBuf, FileContent>,

    // 会话历史
    session_log: Vec<SessionEvent>,

    // 跨 agent 共享变量
    variables: HashMap<String, Value>,
}

// 文件同步机制
impl SharedContext {
    fn sync_to_agent(&self, agent_id: &str) {
        // 将共享上下文同步到指定 agent
    }

    fn collect_from_agent(&mut self, agent_id: &str) {
        // 从 agent 收集修改
    }
}
```

## 4. 实施阶段

### Phase 1: 基础平铺 (1 周)
- [ ] 集成 ratatui-hypertile 到 xylitol
- [ ] 实现基础的窗格分割和焦点切换
- [ ] 单个 agent 运行在平铺布局中

### Phase 2: 多 Agent 支持 (1 周)
- [ ] 实现 AgentWorkspace 管理器
- [ ] 支持多个独立的 agent 会话
- [ ] 窗格间的事件路由

### Phase 3: 临时 Agent (3 天)
- [ ] 实现 ephemeral agent 模式
- [ ] TUI 中的临时 agent 面板
- [ ] 自动清理机制

### Phase 4: 共享上下文 (1 周)
- [ ] 文件浏览器窗格
- [ ] 跨 agent 的文件同步
- [ ] Diff review 窗格集成

### Phase 5: 高级功能 (持续)
- [ ] Agent 间通信协议
- [ ] 工作区布局保存/恢复
- [ ] 插件系统扩展

## 5. 配置示例

```toml
# ~/.xylitol/workspace.toml

[workspace]
default_layout = "vertical"  # vertical | horizontal | grid
max_panes = 8
auto_cleanup = true

[workspace.panes.agent]
default_model = "claude-sonnet"
max_context = 200000
allow_tools = ["read", "write", "edit", "bash"]

[workspace.panes.ephemeral]
max_turns = 10
model = "claude-haiku"
timeout = "5m"

[workspace.shortcuts]
switch_pane = "Ctrl+{n}"
split_vertical = "Ctrl+\\"
split_horizontal = "Ctrl+-"
close_pane = "Ctrl+W,x"
```

## 6. 使用场景示例

### 场景 1: 代码重构
```
用户: 我需要重构这个模块，拆分成多个文件

操作:
1. Ctrl+\ 垂直分割
2. 左窗格: "帮我分析这个模块的职责"
3. 右窗格: "根据分析结果，创建新的文件结构"
4. 共享窗格: 实时查看文件变化
```

### 场景 2: 代码审查
```
用户: 审查这个 PR

操作:
1. 创建 Diff Review 窗格
2. 创建 Agent 窗格，输入 "审查这个 PR 的代码质量"
3. Agent 分析 diff，给出建议
4. 在 Diff Review 窗格中接受/拒绝修改
```

### 场景 3: 学习模式
```
用户: 学习 Rust 的生命周期

操作:
1. 创建多个临时 agent (Ctrl+E)
2. Agent 1: "解释 Rust 生命周期的基础概念"
3. Agent 2: "给出生命周期的代码示例"
4. Agent 3: "解释常见的生命周期错误"
```

## 7. 竞品分析

| 特性 | xylitol Workspace | Zellij | tmux | VS Code |
|------|-------------------|--------|------|---------|
| 平铺布局 | ✓ (ratatui-hypertile) | ✓ | ✓ | ✓ |
| 内置 AI Agent | ✓ | ✗ | ✗ | Copilot |
| 临时 Agent | ✓ | ✗ | ✗ | ✗ |
| 共享上下文 | ✓ | 部分 | ✗ | ✓ |
| 纯终端 | ✓ | ✓ | ✓ | ✗ |
| 插件系统 | ✓ (Rust) | ✓ (Wasm) | ✗ | ✓ (JS) |

## 8. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 性能问题 | 多个 agent 同时运行可能导致卡顿 | 实现智能节流，限制并发 agent 数 |
| 上下文冲突 | 多个 agent 同时修改同一文件 | 实现文件锁机制 |
| 内存占用 | 多个 agent 的上下文占用大量内存 | 实现上下文压缩和卸载 |
| 复杂性 | 用户难以理解多 agent 协作 | 提供模板和向导 |

## 9. 成功指标

| 指标 | 目标 | 衡量方式 |
|------|------|----------|
| 多 agent 并行成功率 | >95% | 测试用例 |
| 窗格切换延迟 | <50ms | 性能测试 |
| 内存占用 | <500MB (4 agents) | 监控 |
| 用户满意度 | >4.5/5 | 用户调研 |

## 10. 开放问题

1. **Agent 间通信协议**: 如何设计一个简洁而强大的通信协议？
2. **布局持久化**: 如何保存和恢复复杂的窗格布局？
3. **权限模型**: 不同窗格的 agent 应该有不同的权限吗？
4. **撤销机制**: 如何实现跨 agent 的撤销操作？

## 11. 参考资料

- [ratatui-hypertile](https://github.com/nikolic-milos/ratatui-hypertile) - 平铺布局引擎
- [Zellij](https://github.com/zellij-org/zellij) - 终端多路复用器
- [tmux](https://github.com/tmux/tmux) - 终端多路复用器
- [pi subagent](https://github.com/earendil-works/pi-mono/tree/main/packages/coding-agent/examples/extensions/subagent) - pi 的子 agent 实现
