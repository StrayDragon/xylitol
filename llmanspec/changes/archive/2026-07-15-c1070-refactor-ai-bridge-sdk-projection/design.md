# Design — c1070 SDK + LlmMessage 组合投影（开闭）

## 分层（normative）

```text
┌──────────────────────────────────────────────────────────────┐
│  AgentMessage（session SSOT）                                  │
│    = Llm(LlmMessage) | Env(EnvMessage)                         │
│    LlmMessage = user / assistant / toolResult（仅 LLM 回合）   │
│    EnvMessage = bash / compact / branch / custom（环境元信息） │
└───────────────────────────┬──────────────────────────────────┘
                            │ project_for_llm → Vec<LlmMessage>
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  bridge AiBridge* DTO（从 LlmMessage 映射；无 Env 平行角色）   │
└───────────────────────────┬──────────────────────────────────┘
                            │ dialect adapter（开闭）
              ┌─────────────┴─────────────┐
              ▼                           ▼
     async-openai Client          Anthropic（reqwest 兜底）
     (+ middleware hooks)         (+ HeaderBag hooks)
```

**组合（非继承）**：Rust 无类继承；用 `AgentMessage::{Llm, Env}` 表达「LLM 回合 ∪ 环境元信息」。
**线格式**：外层 `#[serde(untagged)]`，内层各自 `tag = "role"`，session JSONL 的 `role` 形状保持不变。

**开**：新兼容端 = 新 `AdapterKind` / 配置。
**闭**：ReAct / 应用面不因新网关而改；环境角色折叠策略只落在 `project_for_llm`。

## 为什么不是「Agent 嵌 AiBridge」

- `domain` MUST NOT 依赖 `xylitol-ai-bridge`。
- 公共 LLM 形状归 **domain `LlmMessage`**；bridge 只做字段映射（可逐步与 `LlmMessage` 同构）。

## 投影规则

| 输入 | 输出 |
|---|---|
| `AgentMessage::Llm(m)` | `m` 原样进入 |
| `Env::Bash`（非 exclude） | 折叠为 `LlmMessage::User` 文本 |
| `Env::Compaction` / `Branch` | 折叠为 user 摘要文本 |
| `Env::Custom` | 按 content 折叠或跳过 |

## SDK 迁移策略

| 能力 | 目标 |
|---|---|
| OpenAI Completions | `async-openai` **Client** + middleware hooks |
| OpenAI Responses + `input_tokens` | 同一 Client；**流式**用 BYOT `Value`（按 `type` 匹配），勿绑死 typed `ResponseStreamEvent`（兼容端常缺字段） |
| Anthropic Messages + count_tokens | **reqwest 兜底**（官方 Rust SDK 未成熟；DTO 已收窄，hooks 仍 HeaderBag/JSON） |

切片：domain 组合 → Completions Client → Responses → RemoteCount → Anthropic。

## 与旧方向

| 旧 | 现 |
|---|---|
| 主仓直接用 AiBridge* 消灭 AgentMessage | **保留** AgentMessage；内嵌 domain `LlmMessage` |
| JSON roundtrip 双全量 enum | `project_for_llm` + 显式 map |
| `AgentMessage(AiBridgeMessage)` | **禁止**（依赖方向反了） |
