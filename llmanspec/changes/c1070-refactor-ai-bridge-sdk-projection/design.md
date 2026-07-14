# Design — c1070 SDK + projection（开闭）

## 分层（normative）

```text
┌─────────────────────────────────────────────────────────┐
│  domain AgentMessage                                      │
│  = LLM 回合内容 + 环境元信息（bash / compact / branch / …）│
└───────────────────────────┬─────────────────────────────┘
                            │ project_for_llm（主仓 infra）
                            ▼
┌─────────────────────────────────────────────────────────┐
│  bridge Llm* DTO（仅模型可见形状）                         │
└───────────────────────────┬─────────────────────────────┘
                            │ dialect adapter（开闭）
              ┌─────────────┴─────────────┐
              ▼                           ▼
     async-openai Client          Anthropic official SDK
     (+ middleware hooks)         (+ middleware / 等价钩子)
```

**开**：新兼容端 = 新 `AdapterKind` / 配置（base_url、headers、feature）或薄 wrapper。
**闭**：`AgentMessage`、ReAct、session JSONL、应用面不因新网关而改。

## SDK 迁移策略

| 能力 | 目标 |
|---|---|
| OpenAI Completions | `async-openai` **Client**（不仅 types） |
| OpenAI Responses + `input_tokens` | 同上 Client 的 responses / input_tokens API |
| Anthropic Messages + count_tokens | 官方 Rust SDK（若阻塞则 reqwest 兜底，但接口形状对齐 SDK） |
| Hooks | SDK middleware ↔ 现有三缝语义（HeaderBag / body Value） |

切片：先 Completions Client → Responses → RemoteCount → Anthropic；每步可测、可回滚。

## 投影规则（初稿，实现时落单测）

| AgentMessage 变体 | 投影 |
|---|---|
| User / Assistant（text/thinking/toolCall） | 进入 Llm 消息 |
| ToolResult | 进入 Llm tool 结果 |
| BashExecution | 默认折叠为 user 文本（或按 exclude_from_context 跳过） |
| CompactionSummary / BranchSummary | 折叠为 user/system 摘要文本 |
| Custom | 按 display/content 策略折叠或跳过 |

**MUST NOT** 在 bridge 再定义 `bashExecution` 等 session 角色的平行 enum。

## 与旧 c1070（已撤）

| 旧 | 新 |
|---|---|
| 主仓直接用 AiBridge* 消灭 AgentMessage | **保留** AgentMessage；bridge 收窄为 LLM 投影 |
| 抽公共 types 包 | 不抽 |
| JSON roundtrip 双全量类型 | 显式投影函数 |

## Hooks

可移植缝不变（`src/AGENTS.md`）。换 SDK = 换 middleware 适配器，不改脚本 hook 合约。
