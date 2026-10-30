# 设计：Anthropic prompt caching 打点

## 配置形状

- `XyPromptCacheConfig { enabled: Option<bool>, ttl: Option<XyPromptCacheTtl> }`，`XyPromptCacheTtl` 取 `5m` / `1h`。
- `XyModelConfig.prompt_cache` 由 `XyModelEntryConfig.prompt_cache` 透传；适配器在构造期持有。
- 有效开关 = `config.enabled.unwrap_or(compat != Deepseek)`；TTL 缺省 `5m`。

## 断点算法

- `system` 存在 → 转成 blocks，末块打点（1 个）。
- `tools` 非空 → 最后一个 tool 打点（1 个）。
- 消息数组从尾部向前找第一个含 `text` 或 `tool_result` 的块并打点（1 个）。
- 合计最多 3 个，低于每请求 4 个上限。

## beta 头

- `ttl=1h` 且启用时，在请求头加 `anthropic-beta: extended-cache-ttl-2025-04-11`；`cache_control` 形状为 `{"type":"ephemeral","ttl":"1h"}`。

## 风险

- `cache_control` 打在 tool_result 块上时需保持块结构；合并相邻消息（`merge_consecutive_messages`）后再打点，避免断点落在被合并掉的块上。
- 断点过多会被 API 拒绝；算法固定 3 个，不随消息增长。
