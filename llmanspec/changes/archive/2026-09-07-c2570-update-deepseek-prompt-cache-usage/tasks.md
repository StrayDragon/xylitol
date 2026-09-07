# Tasks: c2570-update-deepseek-prompt-cache-usage

测试 seam：包内单测（`packages/xylitol-ai-bridge` 的 WirePolicy / usage 映射 / Completions JSON 解析）。不新增 BDD harness。不打网。

- [x] t1 specs：在 `package-ai-bridge` 增加 pab28（DeepSeek 轮廓 MUST 映射已出现的 cache 读数，MUST NOT 因方言标 NotApplicable）；`llman sdd validate` 结构绿
- [x] t2 WirePolicy：`Compat::Deepseek` 的 `prompt_cache_usage` 改为 true；单测改为断言 expects cache、仍省略 encrypted include
- [x] t3 usage 映射：Responses 三态在 DeepSeek 轮廓下走 Tokens；Completions 停止 `Tokens(0)` 硬编码，优先 `prompt_cache_hit_tokens` 否则 `prompt_tokens_details.cached_tokens`；字段缺失且 expects true → NotReported，不得写成命中 0
- [x] t4 门禁：`cargo test -p xylitol-ai-bridge` 相关单测 + `llman sdd validate c2570-update-deepseek-prompt-cache-usage --strict --no-check`
