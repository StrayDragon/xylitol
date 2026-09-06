# Design: c2570 DeepSeek Prompt Cache 读数

## Decision

沿用现有 `PromptCacheRead` 三态与 Responses 映射函数，只改 DeepSeek **轮廓开关**和 Completions **读数字段**。不新开 usage DTO，不把命中率写成产品 MUST。

## Options considered

| 选项 | 取舍 |
|---|---|
| A. `prompt_cache_usage=true` + 既有 `cached_tokens` 映射 | 官方 Responses 今晚已回报 `input_tokens_details.cached_tokens`；改动最小 |
| B. 为 DeepSeek 单独解析 `prompt_cache_hit_tokens` 而轮廓仍 false | Completions 需要，Responses 没有该顶栏字段；轮廓 false 会让 Responses 继续 NotApplicable |
| C. 新 `Compat` / 新 provenance | 过度；与 pab19 命名轮廓冲突 |

选 **A + Completions 补 B 的字段优先级**：Responses 走 A；Completions `prompt_cache_hit_tokens` 优先，否则 `prompt_tokens_details.cached_tokens`。缺字段 → `NotReported`，禁止 `Tokens(0)` 硬编码。

## Non-goals

- 不保证分叉命中；不改 session JSONL / Host 组装
- 不打开 DeepSeek 的 encrypted include 或 `previous_response_id`
