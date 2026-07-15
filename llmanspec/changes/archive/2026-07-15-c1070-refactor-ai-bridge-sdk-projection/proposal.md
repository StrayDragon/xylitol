---
change_id: c1070-refactor-ai-bridge-sdk-projection
title: "ai-bridge：厂商 SDK 接线 + AgentMessage→LLM 投影（开闭）"
status: full
priority: 1070
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1070-refactor-ai-bridge-sdk-projection

> 取代已撤销的「主仓直接采用 AiBridge* / 消灭 AgentMessage」方向。
> 与 **c1060**（OpenAI RemoteCount）衔接：本 change 将 RemoteCount 默认实现迁到 SDK Client。

## Why

1. **拆包原因**：各 LLM provider 差异极大，但交付面收敛为 **OpenAI-like** 与 **Anthropic-like**；应用层不应吞厂商细节。
2. **SDK 优先**：手写 reqwest/SSE 重复造轮子，且难吃到官方兼容面。厂商官方路径（`async-openai` Client、Anthropic 官方 Rust SDK）覆盖大部分兼容端；个别网关用配置/薄继承适配即可（开闭）。
3. **概念分层**：`AgentMessage`（session/agent 真源）= **LLM 回合内容 + 环境元信息**（bashExecution、compactionSummary、branchSummary、custom…）。Bridge DTO 只应是 **发给模型的投影**，不是 session 的平行拷贝。今天的孪生 `AiBridgeMessage` + JSON 往返违反该分层。

## Purpose

1. **Provider 接线**：Completions / Responses / RemoteCount（及 Anthropic Messages / count_tokens）**逐步**改为官方 SDK Client；hooks 经 SDK **middleware**（或等价扩展点）接入，保持可移植 `HeaderBag` + JSON body 语义。
2. **组合投影（主仓 domain）**：`AgentMessage = Llm(LlmMessage) | Env(EnvMessage)`（Rust 组合，非继承）；`project_for_llm(&[AgentMessage]) -> Vec<LlmMessage>`；bash/compaction/branch/custom 折叠或剔除；线格式经 untagged 保持 `role` JSONL。
3. **收窄 bridge DTO**：仅映射 `LlmMessage`；**MUST NOT** 为环境角色维护平行 enum；**MUST NOT** `AgentMessage` 内嵌包内 `AiBridgeMessage`（依赖反了）。
4. **开闭**：新兼容端 = 新 adapter/配置；**MUST NOT** 为网关改 ReAct / `LlmMessage` 形状（环境折叠只改投影）。

## What Changes

- `packages/xylitol-ai-bridge` provider：SDK Client 迁移（可分 PR/任务切片）
- bridge dto 收窄；主仓 `infra/provider` 投影函数替换 JSON roundtrip map
- AGENTS（根 / `src` / bridge）分层规则强化
- delta：`package-ai-bridge`、`infra-provider`、`domain-message`、`layer-architecture`

## Capabilities

- `package-ai-bridge`（modify）
- `infra-provider`（modify）
- `domain-message`（modify）
- `layer-architecture`（modify）

## Out of scope

- 抽第三个 `xylitol-llm-types` crate
- `AgentMessage` 内嵌 / newtype 包内 `AiBridgeMessage`（domain→bridge 依赖反转）
- 改变 accounting 优先级（仍 Api → RemoteCount → Local → Heuristic）
- 一次 PR 迁完所有网关边角（允许任务切片，但合约一次定稿）

## Ethics

- risk_level: high
- prohibited_actions: 用孪生全量 DTO「假装」分层；domain/agent 依赖 vendor SDK 类型；domain 嵌套 AiBridgeMessage
- required_evidence: 投影单测（含 bash 不泄漏为独立 LLM role）；SDK 路径 wiremock/集成；arch_guard（domain 无 vendor）
- escalation_policy: Anthropic 官方 Rust SDK 若不可用/许可证不合，升级确认后保留 reqwest 实现但 **DTO 仍须收窄**

## Depends

- **c1030-add-package-ai-bridge**（已归档）
- 实施顺序建议：先归档或稳住 **c1060** 合约，再在本 change 把 input_tokens 接到 SDK Client
