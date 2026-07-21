---
change_id: c1210-refactor-compose-bridge-llm
title: 对齐 pi——组合 bridge LLM DTO；bash 嵌 message 落盘；去掉叶孪生 map
status: full
priority: 1210
depends_on: []
author: agent
branch: feat/c1210-refactor-compose-bridge-llm
base_sha: 92fe7048a544aab9c8e4d0ea59b0ea8206150b25
checkpointed: true
checkpoint_sha: 92fe7048a544aab9c8e4d0ea59b0ea8206150b25
---

# c1210-refactor-compose-bridge-llm

## Why

今日 `domain::LlmMessage` 与 `xylitol_ai_bridge::AiBridgeMessage` 近 1:1 孪生，靠 `infra/provider/map.rs` 手写叶映射；bang-bash 又以顶层 `SessionEntry::BashExecution` 落盘，与 pi（`type:message` + `role:bashExecution`）及「库压平类型由业务组合」的自然期望不一致。§F 曾冻结「禁 domain→bridge」，但该自限挡住了目标形态。本 change **解冻并改合约**：LLM 叶 SSOT 在 bridge；session `AgentMessage` 组合之；Env 折叠仍在 `project_for_llm`；新写入 bash 与 pi 同构。

## Purpose（已钉）

1. **组合**：`AgentMessage = Llm(AiBridgeMessage) | Env(EnvMessage)`（或等价命名）；domain **MAY** 依赖 bridge **DTO only**；**MUST NOT** 再维护平行 `LlmMessage` 叶 enum，**MUST NOT** 依赖 bridge HTTP/vendor SDK。
2. **投影**：发模型前 MUST `project_for_llm` → `Vec<AiBridgeMessage>`；Llm 臂 **passthrough**；Env（含 bash/compaction/branch/custom）折叠为 user 文本或跳过；**MUST NOT** 再靠全量叶↔叶 `From`/手写孪生表 + JSON 往返。
3. **bash 落盘**：新写入 MUST 为 `SessionEntry::Message`，`message.role = bashExecution`（字段集仍含 command/output/exit_code/cancelled/truncated/full_output_path/exclude_from_context）；**MUST NOT** 再写出顶层 `type:bashExecution`。读路径 MUST 将旧顶层 bash 提升为等价 Message+role（兼容既有 JSONL）。
4. **compaction/branch**：保持顶层 SessionEntry（与 pi 一致，便于扫描）；构建上下文时投影为 Env summary 消息。
5. **ReAct 种子**：加载多轮 history MUST 经统一「entry→AgentMessage」投影（含 Message 内 bash、compaction、branch_summary），并尊重 `exclude_from_context`；不得只过滤 `type:message` 而永久丢 bang/摘要。

## What Changes

- live specs：`domain-message`（dm3/dm6）、`infra-bash`（be4/be6）、`package-ai-bridge`（pab3/pab4）、`infra-provider`（pa20）、`layer-architecture`（la25；必要时 la1/la2 措辞）、对应 `.feature`
- 实现（apply）：`message.rs` / `llm_project.rs` / session append+load / `map.rs` 收缩 / `token_estimator` 去重 / AGENTS + `_TODO` §F 解冻
- 单测 + 相关 BDD；旧 JSONL 读兼容测

## Out of scope

- 抽第三 `*-types` crate 或迁出 `xylitol-domain` 包
- 改 vendor adapter 方言映射（OpenAI/Anthropic 内部）
- 改 Trust / Permission / MCP 产品语义
- 强制改写用户磁盘上旧 session 文件（只要求读兼容；新写入用新形状）

## Capabilities

- `domain-message`（modify）
- `infra-bash`（modify）
- `package-ai-bridge`（modify）
- `infra-provider`（modify）
- `layer-architecture`（modify）
- `agent-session`（modify as45）

## Ethics

- risk_level: high（改 session JSONL 新写形状 + 跨层类型归属）
- prohibited_actions: 让 Env 角色进入 bridge DTO；domain 依赖 bridge HTTP/SDK；新写入顶层 bashExecution 条目；静默丢弃旧顶层 bash 而不做读提升
- required_evidence: 新 bang JSONL 为 message+role；旧顶层 bash 可进上下文（除 exclude）；`project_for_llm` 后无 bash 角色；叶孪生 map 消失或仅 identity；相关 unit/BDD 绿
- escalation_policy: 用户已确认方案 A + 单 change 含 bash 落盘
