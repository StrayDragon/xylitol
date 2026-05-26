---
depends_on:
  - c104-refactor-core-traits
---

# c105-refactor-agent-loop

## Why

完成 c103（provider）和 c104（core traits）后，xylitol 对 adk-rust 的依赖仅剩：
- `adk-runner`：ReAct loop 编排（Runner + LlmAgentBuilder）
- `adk-agent`：Agent 构建器
- `adk-session`：InMemorySessionService（运行时 session backend）
- `adk-core`：通过 compat adapter 间接使用

xylitol 的 ReAct loop 需求很简单：单 agent、tool dispatch、streaming、max iterations。
不需要 adk-rust 的多 agent 编排、graph workflow、A2A、sandbox 等重量级功能。
自建 ReAct loop 约 500-800 行代码，换来完全的自主可控。

## What Changes

1. 新增 `src/agent/runner.rs`：自建 `XyRunner`，实现 ReAct loop
   - prompt 组装 → `XyModel::generate_stream` → parse tool calls → `XyTool::execute` → loop
   - max_iterations 限制
   - streaming event 发射（复用 `AgentEvent`）
2. 新增 `src/agent/session.rs`：`XySession` trait + `InMemorySession` 实现
   - 会话历史管理（message list）
   - 与 `infra/session/` snapshot 系统对接
3. 修改 `src/agent/loop.rs`：用 `XyRunner` 替换 `adk_runner::Runner` + `adk_agent::LlmAgentBuilder`
4. 删除 `src/agent/compat.rs`（不再需要 adk 兼容层）
5. 从 `Cargo.toml` 移除 `adk-runner`、`adk-agent`、`adk-session`、`adk-core`

## Capabilities

- agent-runtime（执行循环、session、事件系统）

## Impact

- **高工作量**：需重写核心执行循环（~500-800 行）
- **中等风险**：ReAct loop 是关键路径，需充分测试
- **最高收益**：完全去除 adk-rust 依赖，实现零外部 agent 框架绑定
