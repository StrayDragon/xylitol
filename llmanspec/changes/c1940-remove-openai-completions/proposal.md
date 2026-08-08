---
depends_on: []
---

# 移除 OpenAI Completions：OpenAI 兼容族仅 Responses（客户端持态）

> **Explore 结论**：官方 Responses 状态策略取 **②** `store:false` + 完整 output Item / `thinkingSignature` 回放（本地 SSOT / 多厂商 / resume）；**①** `previous_response_id`（需官方账号可 store）本波不做。参考 `openai-developers` → `developers.openai.com`（Migrate / Conversation state / Reasoning / Prompt caching）。
> **前置**：c1880 曾把 Completions 留作显式遗留档；本 change **撤销**该逃生口，一步删净。

## Why

- Completions 与 Responses 是两套协议族（消息布局、流式事件、reasoning 跨轮语义均不同）。双路径迫使 thinking / hooks / 观测 / 测试矩阵分叉，阻碍「DeepSeek / Qwen 等方言端统一吃 `/v1/responses`」的简化目标。
- 官方推荐新项目走 Responses；Completions 跨轮丢 reasoning。xylitol 已以 Assembler + encrypted replay 实现 ②，Completions 不再提供产品价值。
- Pre-1.0、未发布：可硬切，不做兼容 shim / 迁移警告。

## What Changes

- **删除** OpenAI Chat Completions 适配实现、选型、`async-openai` 的 `chat-completion` feature 依赖面、相关单测 / BDD 场景 / 文档逃生说明。
- **OpenAI 兼容装配**：唯一协议族 `openai-responses`（省略 `api` 时仍默认）；配置中残留 `api: openai-completions`（或其它未识别 OpenAI 族字符串）**静默按省略处理** → 装配 Responses，**不**为此单独报错或告警。
- **钉死 ②**：继续 `store:false` + full-replay；`WirePolicy.previous_response_id` 默认 false；产品不交付 ①。
- **保留** Anthropic Messages（独立第一语言）。
- **叙事**：OpenAI 兼容多 provider = 同一 Responses 形状 + `compat=generic`；不把 Completions 当方言逃生。

## Capabilities

- `package-ai-bridge`
- `infra-provider`
- `runtime-model-registry`
- `agent-hooks`（去掉 Completions 接线条款）
- `package-ai-bridge-accounting`（去掉 Completions-only 措辞）
- 文档：`docs/architecture/多厂商模型.md`、`配置与档案.md`、`configs/example.yaml`、bridge `AGENTS.md`

## Impact

- 用户 YAML 若仍写 `openai-completions`：开发阶段行为变为走 Responses（用户自行改配置；产品不专项报错）。
- 仅支持 Responses 形状的兼容端成为 OpenAI 族唯一路径；Anthropic 不变。
- 测试与观测矩阵缩小一档。

## Out of scope

- 实现 / 产品化 `previous_response_id` 链式（①）
- 打开 `prompt_cache_key` 产品旋钮（可后续 change）
- 新增 `reasoning.context`（`all_turns`）字段（可后续）
- 砍 Anthropic
- 削 `XyChunk` / `LlmAdapter` 镜像层（可并行 quick / 另 change）
- 为废弃 `api` 字符串加用户可见警告 / 迁移工具

## Decisions

1. 状态机：**仅 ②**；① 延期且文档写「暂不支持」。
2. Anthropic：**保留** Messages。
3. 废弃 `api`：**静默回落** Responses（等同省略），不报错。
4. Completions：**物理删除**，无 fallback 实现。
