# Prompt caching：权威惯例（2026-10-31）

来源：SAP 帮助文档、`@anthropic-ai/sdk` 类型、LiteLLM、pi-ai README、Azure OpenAI 文档。

## Anthropic

- 每请求最多 4 个 `cache_control(ephemeral)` 断点；前缀层级 tools→system→messages。
- agent 惯例：system 末尾 + 最后一个 tool + 最后一条消息文本块（当前实现一致，留 1 个余量）。
- TTL 默认 5m；`ttl:"1h"` 需 `anthropic-beta: extended-cache-ttl-2025-04-11`（已实现）。
- 最小可缓存 token 1,024–4,096（按模型），短前缀会被忽略。

## OpenAI

- 全自动：≥1,024 token 且首 1,024 一致即缓存，无需标注。
- prompt_cache_key 由调用方自选稳定标识（非前缀哈希）；同前缀 + key 超 15 req/min 会 miss。
- `prompt_cache_retention` 取 `in_memory`|`24h`；SDK 注释：无 ZDR 组织默认 24h、有 ZDR 默认 in_memory；gpt-5.5 起仅 24h。
- pi-ai compat 由 sessionId 派生 cache affinity，默认关；xylitol generic 默认发 key（可配置关闭），deepseek 关闭。

## 差异

- retention 默认不传（交服务端默认）与官方一致。
- prompt_cache_key 默认发送是 xylitol 的选择（generic），与 pi-ai 默认关不同；端点接受性已实测（halogen 200）。
