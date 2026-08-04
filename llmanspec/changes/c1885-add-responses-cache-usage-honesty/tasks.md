# Tasks: c1885-add-responses-cache-usage-honesty

> Specs landing 须在 `change start` / attach 之后。

## 0. Review 门

- [x] 0.1 深挖：三态 / 枚举+派生 cache_read / 默认 prompt_cache_usage=true / 观测不做 TUI / 无 prompt_cache_key
- [x] 0.2 测试 seam：包内单测 + toon `feature: false`；不扩 BDD step
- [x] 0.3 延后 TUI draft：`c1940-add-tui-prompt-cache-footer`

## 1. Specs landing（Branch binding 后）

- [ ] 1.1 `package-ai-bridge`：修订 pab19（仅 `prompt_cache_usage` 默认 true）+ 场景
- [ ] 1.2 `package-ai-bridge`：新增三态映射 req + 观测诚实 req（`feature: false` 场景）
- [ ] 1.3 **跳过** `runtime-config` / TUI chrome / `infra-observability` 扩 valid_scope（观测落在 bridge trace）
- [ ] 1.4 产品/research 短句：更新「cache_read 恒 0」过时表述（若触及）

## 2. 实现（apply）

- [ ] 2.1 DTO：`PromptCacheRead`（名以实现为准）接入 `AiBridgeUsage`；派生 `cache_read`
- [ ] 2.2 usage：Responses `from_responses_usage_with_policy` 按闸门填三态；`PROMPT_CACHE_USAGE` 默认 true
- [ ] 2.3 观测：`ProviderRequestTrace::attach_usage` 透出三态；禁止 NotReported/NotApplicable 写伪 cache_read
- [ ] 2.4 单测：Tokens / NotReported / NotApplicable / default policy / attach_usage

## 3. 校验

- [ ] 3.1 `llman sdd validate`（change + package-ai-bridge）`--strict --no-check`
- [ ] 3.2 `cargo test -p xylitol-ai-bridge`（usage / wire_policy / trace）+ 触及面 fmt/lint
