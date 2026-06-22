---
id: c170-refactor-agent-message-types
title: "Refactor Agent Message Types — align with pi's AgentMessage type system"
depends_on: []
---

## Why

当前的消息类型系统（`XyContent`/`XyPart`/`XyChunk`/`XyRole`）过于简化，无法完整表达 pi coding agent 中的消息语义：

1. **缺失角色**：缺少 `bash_execution`、`custom`、`compaction_summary`、`branch_summary` 等消息角色
2. **缺失图片支持**：不支持图片附件（`ImageContent`）
3. **缺失使用量跟踪**：不能在消息级别携带 token 使用量（`Usage`）
4. **停止原因过简**：只有 `Stop`/`MaxTokens`，缺少 `error`/`aborted`/`toolUse`
5. **缺失温度等参数**：不能在消息级别配置生成参数

这些是 AgentSession 状态机正常运行的基础前提——所有上层功能都依赖消息在序列化、持久化和 provider 间正确传递。

## What Changes

1. 废弃 `XyContent`/`XyPart`/`XyRole`，引入 `AgentMessage` 枚举（对齐 pi 的 `AgentMessage`）
2. 新增消息角色：`BashExecution`、`Custom`、`CompactionSummary`、`BranchSummary`
3. 新增 `ImageContent` 类型，支持多图片附件
4. 新增 `Usage` 结构体（input/output/cacheRead/cacheWrite/totalTokens）
5. 扩展 `XyFinishReason` 为 `StopReason`：`stop`/`maxTokens`/`error`/`aborted`/`toolUse`
6. 引入 `ThinkingLevel` 作为独立枚举（已存在但需对齐 pi 的 `ThinkingLevel`）
7. 新增 `StopReason` 枚举 + `errorMessage` 字段（用于 error/aborted）
8. 提供 LLM 消息转换层（`convert_to_llm`）：`AgentMessage[]` → `Message[]`（provider 边界）
9. 保留向后兼容的 JSONL 序列化格式（session 文件 v3→v4 迁移）

## Capabilities

- agent-types

## Impact

- `src/agent/types.rs`：重写
- `src/agent/model.rs`：Provider message conversion 接口变更
- `src/agent/session.rs`、`src/agent/loop.rs`：消息类型引用更新
- `src/infra/session/types.rs`：SessionEntry 消息类型更新
- 所有 provider 实现（openai/anthropic/fake/mock）：适配新类型
- BDD tests：更新步骤定义以使用新类型

## Definition of Done

- [ ] `XyContent`/`XyPart`/`XyRole` 被 `AgentMessage` 枚举替代
- [ ] 所有缺失角色（bash、custom、compaction_summary、branch_summary）已实现
- [ ] `ImageContent` 支持可用
- [ ] `Usage` 在消息中持久化
- [ ] `StopReason` 完整（5 种变体）
- [ ] `convert_to_llm()` 至少为 OpenAI 和 Anthropic 实现
- [ ] 所有编译警告清零，`cargo test` 通过
