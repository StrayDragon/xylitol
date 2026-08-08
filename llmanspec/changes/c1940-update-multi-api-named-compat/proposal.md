---
depends_on: []
branch: sdd/c1940-update-multi-api-named-compat
base_sha: b0a3aa0cc399d6861b0bc1173dfdde12407b08c4
checkpointed: false
---

# 三协议族 + 命名 compat（Completions 一等公民；Zen / DeepSeek）

> **Rename**：原 id `c1940-remove-openai-completions` 误导（曾拟删 Completions）；本 change **保留并一等支持** `openai-completions`，与 Responses / Anthropic Messages 并列。
> **Pi 对照**：`api` = 协议族；`compat` = 同族 quirk；auth ≠ model table；勿 URL 自动探测大表。详见 `design.md`。

## Why

- Zen `deepseek-v4-flash-free` → `https://opencode.ai/zen/v1/chat/completions`：**必须** Completions（别名 `deepseek-v4-flash-free-zen`）。
- DeepSeek 官方 `deepseek-v4-flash` → Responses（用户指定；`store` 恒 false、无 `previous_response_id`/`include`）：**方言** Responses。
- 同 upstream 多通道：YAML 键 = 显示 id，后缀 `*-zen` / `*-anthropic`；`model:` 字段可共享。
- 自建 llama.cpp（Ornith）已跑 Responses：保持。
- 静默把 Completions 吞成 Responses 会错路由 Zen free。

## What Changes

- **`openai-completions` 为一等 `api`**（与 `openai-responses` / `anthropic-messages` 并列）；**禁止**从产品/合约面删除或降级 Completions。
- **分层**：bridge `provider/native` = L1 第一语言实现；`provider/dialect` = L2 命名方言增量（首版 `deepseek`）。
- **新增** `models.*.compat`（命名轮廓 → `WirePolicy`）：首版 `generic` | `deepseek`。
- **新增** `models.*.api_key`（可选；支持 `{{ secret.* }}`），解决 Zen / DeepSeek / 全局 `OPENAI_API_KEY` 冲突。
- Responses × `compat: deepseek`：不发 `include: reasoning.encrypted_content`；保持 `store:false`。
- Completions × `compat: deepseek`：thinking 体对齐 pi `thinkingFormat: deepseek`。
- Anthropic × `compat: deepseek`：`thinking.type=enabled`，不发 `budget_tokens`（DeepSeek Anthropic 兼容端）。
- 用户配置：Zen free Completions + DeepSeek 官方 Responses + DeepSeek 官方 Anthropic（`-anthropic` 别名）。

## Capabilities

- `package-ai-bridge` / `infra-provider` / `runtime-model-registry` / `runtime-config`

## Out of scope

- models.dev 自动拉全表 / pi 式远程 catalog（可后置）
- Gemini / Zen 全量模型矩阵
- `previous_response_id` 产品化
- 把 llama.cpp 改回 Completions（用户现网已是 Responses）

## Decisions

1. `api` 仅三值：`openai-responses` | `openai-completions` | `anthropic-messages`（全称 kebab，不改点号）。
2. `compat` 正交于 `api`；默认 `generic`；DeepSeek 官方 Responses / Zen DeepSeek 形 Completions 用 `deepseek`。
3. 鉴权：优先 `models.*.api_key`，否则回落 kind 级 env（`OPENAI_API_KEY` / `ANTHROPIC_API_KEY`）。
4. 不引入 URL 自动探测表（pi 的 detect 表 xylitol 不抄）。

## Further Notes

- MCP 公开名：`mcp__{server}__{tool}`（`MCP_PUBLIC_DELIMITER`）；含 `-` 时的行业做法见 [`docs/research/mcp-tool-public-naming-hyphen-2026.md`](../../../docs/research/mcp-tool-public-naming-hyphen-2026.md)（[MCP tool naming research](daf6214f-51be-41e0-8c62-648428d34014)）。
