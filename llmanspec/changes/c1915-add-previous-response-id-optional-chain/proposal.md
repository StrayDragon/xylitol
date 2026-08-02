---
depends_on:
  - c1880-update-responses-first-api-boundary
  - c1890-add-responses-context-policy-assembler
  - c1920-add-context-epoch-freeze
---

# previous_response_id 可选链式续跑（配置开启）

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md) §5。
> **波次**：Wave D（依赖 `c1880`+`c1890`+`c1920`；建议 Wave C 主要项稳定后再开）
> **自包含**：默认仍全量重放；链式为 capability + 配置 opt-in。**断链条件含 context epoch bump**（以 `c1920` 为准）。

## Why

Responses 支持用 `previous_response_id` 只传增量 input，可减重复传长轨迹。但兼容端参差、compact/换模/fork/工具世代会断链。应先保证公共全量路径极致（`c1880`–`c1910`），再把链式当**可选优化**。

## What Changes

- 落盘并使用 `response_id`（今日常为 `None`）。
- capabilities + 配置打开时：连续 tool 环可发增量 + `previous_response_id`；`store` 策略按配置/文档（兼容端实测清单进 design）。
- **flavor 门闸**：仅当该 flavor 声明支持链式时允许开启；形似 Responses 但未验证的端点默认关，避免 DeepSeek/网关等静默丢上下文。
- **断链回退全量**：compact、换模、fork、**`c1920` context/tools epoch bump**、配置关闭、缺 id、网关错误、flavor 不支持。
- 观测：标明本轮 `full_replay` vs `chained` 与当前 flavor。
- 首版**不**自动探测是否支持。

## Capabilities（意向）

- `package-ai-bridge` / Responses adapter
- `agent-runtime`（何时可链）
- capabilities 配置（`c1880`）

## Impact

- 长 tool 环在官方 OpenAI 上可降重复传。
- 兼容端默认不受影响（关）。

## Out of scope

- 默认开启链式
- 自动发现网关能力
- 多模态专用 store 策略

## Parallel / depends

- **硬依赖**：`c1880`、`c1890`、`c1920`
- 建议不与 Wave C 抢同一 adapter 文件；或 Wave C 合并后再开本 change

## Open Questions

- `store: true` 与本地 session SSOT 的隐私/磁盘—— propose 时钉默认 false + 链式是否仍可能。

## Ethics

- risk_level: medium
- prohibited_actions: 断链后静默丢上下文；默认全员开链式
- required_evidence: 断链回退全量测；配置关时行为与今日一致
- refusal_contract: 不承诺一切兼容端支持 id 链
- escalation_policy: 开启 store 持久化上游侧须用户确认
