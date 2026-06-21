---
depends_on: []
---

# c82-provider-attribution: Provider 专属 HTTP 头 + 显示名称映射

## Why
pi 的 `provider-attribution.ts`（109 LOC）在向 OpenRouter / NVIDIA NIM / Cloudflare / Vercel AI Gateway / OpenCode 发送请求时，自动注入必要的 attribution header（如 `HTTP-Referer: https://pi.dev`、`X-BILLING-INVOKE-ORIGIN: Pi`）。`provider-display-names.ts`（35 LOC）维护了 27+ 个 provider 的人类可读名称映射。

xylitol 完全缺失这两块，导致：
- OpenRouter 请求缺 attribution headers → 可能影响排行榜统计
- CLI/log 中 provider 名称显示为原始 id（如 `openrouter`）而不是「OpenRouter」

## What Changes
- 新增 `src/agent/provider/attribution.rs`：
  - `merge_provider_attribution_headers(model, session_id) -> Option<HashMap<String, String>>`
  - 对 OpenRouter / NVIDIA / Cloudflare / Vercel / OpenCode 自动注入 headers
  - 对 OpenCode 附加 `x-opencode-session` 和 `x-opencode-client`
  - 静态常量 `BUILT_IN_PROVIDER_DISPLAY_NAMES: HashMap<&str, &str>`
  - `provider_display_name(provider_id: &str) -> &str` — 查表，fallback 到原始 id
- 集成到 `agent/resolver.rs`：模型请求时调用 `merge_provider_attribution_headers`
- 集成到 CLI/RPC 输出时使用 `provider_display_name()` 格式化

## Capabilities
- provider-integration

## Impact
- 非破坏性：新增模块 + resolver 注入逻辑。
- 无新依赖。
- c81 (config-value-resolver) 可增强 header 值的解析，但本变更先使用纯字面量 headers。
