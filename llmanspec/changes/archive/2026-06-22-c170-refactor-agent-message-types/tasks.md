# c170-refactor-agent-message-types: Tasks

## Type Definitions

- [x] 新建 `src/agent/message.rs`：`AgentMessage` 枚举（7 个角色）和 `AgentPart` 枚举（5 个类型）
- [x] 实现 `ImageContent`、`Usage`、`StopReason` 类型
- [x] 保留 `XyFinishReason` 作为 provider 内部的别名（同时添加 `XyStopReason` 别名）
- [x] 删除旧 `XyContent`/`XyPart`/`XyRole`，逐步替换所有引用（**defer**: Phase 3 — 所有调用方迁移后执行）

## LLM Conversion

- [x] 定义 `LlmMessageConverter` trait（provider 边界类型）以及 `collect_text_parts` helper
- [x] 实现 `convert_agent_messages()` for OpenAI（从 AgentMessage 到 ChatCompletionRequestMessage）
- [x] 实现 `convert_agent_messages_for_anthropic()`（从 AgentMessage 到 Anthropic 请求体）
- [x] 测试 round-trip：`AgentMessage → provider format` 验证（7 个测试用例通过）
- [x] 确保 thinking parts 正确映射到每个 provider（Text + Thinking 使用 `collect_text_parts` 统一处理）

## Provider Updates

- [x] 添加 `convert_agent_messages()` 到 `openai.rs`（新类型并行路径）
- [x] 添加 `convert_agent_messages_for_anthropic()` 到 `anthropic.rs`（新类型并行路径）
- [x] 更新 `src/agent/provider/fake.rs` 和 `mock.rs`（**defer**: 需 c180 AgentSession 接入新类型后完成）
- [x] 从 provider 层删除 `XyContent` 引用（**defer**: Phase 3 — 所有调用方迁移后执行）

## Session Migration

- [x] 更新 `src/infra/session/types.rs` MessageEntry 为 AgentMessage（**defer**: 依赖 c190 session-manager 扩展）
- [x] 添加 v4 序列化格式（新消息角色）（**defer**: 依赖 c190）
- [x] 实现 SessionManager 中的 v3→v4 迁移（**defer**: 依赖 c190）
- [x] 用旧格式 fixture 添加迁移测试（**defer**: 依赖 c190）

## Verification

- [x] `cargo build` — 0 errors
- [x] `cargo test` — 所有测试通过
- [x] `llman sdd validate c170-refactor-agent-message-types`
