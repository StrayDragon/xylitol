---
depends_on: [c15-add-cli, c25-add-agent-loop]
blocks: []
---

# c87-add-rpc-mode

## Why

PRD 定义三种运行模式：Print、Interactive(TUI)、JSON-RPC。前两种已有对应 change（c30、c80），JSON-RPC over stdio 用于 IDE 集成场景，是完整模式覆盖的必要部分。

## What Changes

1. 在 `src/interface/rpc.rs` 实现 JSON-RPC 2.0 over stdio
2. 请求/响应序列化（serde_json）
3. 与 agent loop 事件流集成
4. IDE 集成友好的协议设计（参考 codex-rs JSON-RPC）

### JSON-RPC 协议

```json
// 请求
{"jsonrpc": "2.0", "method": "agent/prompt", "params": {"text": "..."}, "id": 1}

// 响应（流式）
{"jsonrpc": "2.0", "method": "agent/text_delta", "params": {"text": "..."}}
{"jsonrpc": "2.0", "method": "agent/tool_call", "params": {"tool": "read", "args": {...}}}
{"jsonrpc": "2.0", "result": {"status": "done"}, "id": 1}
```

## Capabilities

- `rpc-mode`: JSON-RPC 2.0 over stdio 模式

## Impact

- `src/interface/rpc.rs` 从占位变为实际实现
- 无额外重量级依赖（serde_json 已在依赖中）
- **始终编译**：无 feature flag，作为 IDE 集成的标准交互模式
