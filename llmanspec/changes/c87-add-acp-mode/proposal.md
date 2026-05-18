---
depends_on: [c15-add-cli, c25-add-agent-loop]
---

# c87-add-acp-mode

## Why

IDE 集成需要一个标准化的 editor-agent 协议，而非自定义 JSON-RPC。ACP (Agent Client Protocol) 由 Zed/JetBrains 制定，定位为 "AI coding agent 的 LSP"——IDE 原生支持 ACP（Zed、JetBrains、VS Code、Neovim、Emacs），agent 只需实现 ACP 即可即插即用，无需为每个 IDE 写适配器。

ACP 使用 JSON-RPC 2.0 over stdio（与原 c87 自定义方案相同的传输层），但提供标准化的会话管理、流式输出、工具调用报告，且复用 MCP ContentBlock 格式。已有 Rust SDK（`agent-client-protocol` crate, Apache 2.0）。

## What Changes

1. 在 `src/interface/acp.rs` 实现 ACP Agent 端（editor 为 Client，xylitol 为 Agent）
2. 使用 `agent-client-protocol` Rust SDK（Agent trait + Builder），不手写 JSON-RPC 帧处理
3. 将内部 `AgentEvent` 流转换为 ACP `SessionNotification` 消息
4. 会话生命周期：`initialize` → `session/new` → `session/prompt` → streaming → response
5. 集成 CLI 分派（`--mode acp`）

### ACP 协议映射

| ACP 方法 | 处理 |
|----------|------|
| `initialize` | 返回 agent 信息、能力 |
| `session/new` | 创建新的 agent 会话（cwd, MCP 配置） |
| `session/prompt` | 运行 agent loop，流式返回事件 |
| `session/cancel` | 中断当前执行 |
| `session/close` | 清理会话 |

### 事件转换

| AgentEvent (c25) | ACP SessionUpdate |
|------------------|-------------------|
| `TextDelta` | `AgentMessageChunk` with `TextContent` |
| `ToolCallStart` | `ToolCall` |
| `ToolCallEnd` | `ToolCallUpdate` |
| `StepComplete` | `TurnEnd` |
| `Error` | `AgentMessageChunk` with error text |

## Capabilities

- `acp-mode`: ACP (Agent Client Protocol) over stdio 模式，IDE 即插即用

## Impact

- 新增 `agent-client-protocol` + `agent-client-protocol-schema` 依赖
- feature flag `infra-acp`（默认启用）
- `src/interface/acp.rs`（新文件，替代原 rpc.rs）
- 替代原 c87-add-rpc-mode 的自定义 JSON-RPC 方案
