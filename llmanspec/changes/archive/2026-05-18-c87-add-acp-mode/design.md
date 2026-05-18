# c87-add-acp-mode — Design

## Context

- ACP (Agent Client Protocol): Zed/JetBrains 制定的标准协议，定位为 "AI coding agent 的 LSP"
- 传输层: JSON-RPC 2.0 over stdio（editor spawn agent subprocess）
- Rust SDK: `agent-client-protocol` crate (v0.12.2, Apache 2.0)
- 依赖关系见 proposal.md frontmatter（depends_on / blocks 为 SSOT）

## Goals / Non-Goals

### Goals

- 实现 ACP Agent 端协议（xylitol 被 IDE 作为 subprocess 启动）
- 会话管理（session/new, session/prompt, session/cancel）
- AgentEvent → ACP SessionNotification 转换
- IDE 即插即用（支持 Zed, JetBrains, VS Code, Neovim 等）

### Non-Goals

- 不实现 ACP Client 端（xylitol 不是 editor）
- 不实现 HTTP/WebSocket transport（仅 stdio）
- 不实现 ACP Registry 注册
- 不实现 IDE 插件（仅提供协议层）

## Decisions

### Decision 1: 使用 ACP Rust SDK

**选择**: 使用 `agent-client-protocol` crate 的 `Agent` trait 和 `Builder` 模式，不手写 JSON-RPC 帧处理。

**理由**:
- SDK 处理 JSON-RPC 2.0 帧构建、消息解析、传输生命周期、版本协商
- Agent trait 方法直接映射到 ACP 协议方法
- 减少维护负担，协议更新跟随 SDK

```mermaid
sequenceDiagram
    participant IDE as IDE (ACP Client)
    participant ACP as ACP Agent<br/>(agent-client-protocol SDK)
    participant Xylitol as xylitol<br/>(acp.rs handler)
    participant Loop as AgentLoop

    IDE->>ACP: initialize
    ACP->>Xylitol: on_initialize()
    Xylitol-->>ACP: AgentInfo{name, version, capabilities}
    ACP-->>IDE: AgentInfo

    IDE->>ACP: session/new {cwd, mcpServers}
    ACP->>Xylitol: on_session_new(cwd, config)
    Xylitol-->>ACP: Session{id}
    ACP-->>IDE: Session

    IDE->>ACP: session/prompt {text}
    ACP->>Xylitol: on_session_prompt(session, text)
    Xylitol->>Loop: run(prompt)
    Loop-->>Xylitol: TextDelta("I'll read...")
    Xylitol-->>ACP: SessionUpdate{AgentMessageChunk{TextContent}}
    ACP-->>IDE: notification: session/update

    Loop-->>Xylitol: ToolCallStart("read", ...)
    Xylitol-->>ACP: SessionUpdate{ToolCall}
    ACP-->>IDE: notification: session/update

    Loop-->>Xylitol: StepComplete
    Xylitol-->>ACP: TurnEnd
    ACP-->>IDE: response: result

    Note over IDE,ACP: 取消场景

    IDE->>ACP: session/cancel
    ACP->>Xylitol: on_session_cancel(session)
    Xylitol->>Loop: cancel()
    Xylitol-->>ACP: TurnEnd{cancelled}
    ACP-->>IDE: response: cancelled
```

### Decision 2: feature flag `infra-acp`

**选择**: `infra-acp` feature flag（默认启用），控制 `agent-client-protocol` crate 是否编译。

```toml
[features]
infra-acp = ["dep:agent-client-protocol", "dep:agent-client-protocol-schema"]
```

**理由**:
- 遵循项目 `infra-*` 命名约定（infra-lsp, infra-dap, infra-skills）
- ACP SDK 引入额外依赖，feature flag 允许最小化构建时剔除
- 默认启用确保标准构建支持 IDE 集成

### Decision 3: acp.rs 模块结构

**选择**: `src/interface/acp.rs` 单文件模块。

**职责**:
- `run_acp_mode(config: AppConfig) -> Result<()>` — 主入口
- ACP SDK 连接设置
- `AgentEvent → SessionNotification` 转换器
- 会话生命周期管理

### Decision 4: AgentEvent 映射

**选择**: `AgentEvent` 枚举不变（c25 负责），转换逻辑在 acp.rs 中完成。

| AgentEvent (c25) | ACP SessionUpdate |
|------------------|-------------------|
| `TextDelta(text)` | `AgentMessageChunk` with `ContentChunk` + `TextContent` |
| `ToolCallStart(name, args)` | `ToolCall` {id, name, arguments} |
| `ToolCallEnd(result)` | `ToolCallUpdate` {result} |
| `StepComplete` | `TurnEnd` |
| `Error(msg)` | `AgentMessageChunk` with error text |

ACP 内容格式复用 MCP ContentBlock，与 c65-add-skills-mcp 可共享类型定义。

## Risks / Trade-offs

| 风险 | 等级 | 缓解 |
|------|------|------|
| ACP SDK 版本更新 breaking change | 中 | 锁定 SDK 版本；ACP spec 尚在 0.x 阶段 |
| ACP spec 尚未 stable | 中 | 跟随主流实现（Zed/JetBrains），beta 阶段即集成 |
| MCP ContentBlock 类型与 c65 重叠 | 低 | 实现时统一类型引用，避免重复定义 |

### 待确认问题

- 无
