---
depends_on: []
---

# 供应商会话键策略(缓存优先)

## Why

「会话」在供应商眼里靠一组**会话键**路由/持态:OpenCode Zen 网关的 `x-opencode-session` header、`prompt_cache_key`(OpenAI 兼容族)、`previous_response_id`(Responses API)。今日实现把**书签 UUID** 直接当这些键:OpenCode header = 当前书签,`prompt_cache_key` / `previous_response_id` 产品默认关。

分叉在产品语义上是「共享树干前缀的新书签」,前缀匹配型缓存(DeepSeek 磁盘前缀、llama.cpp 槽内 KV)本可继续命中;但书签当键等于每次 fork 向供应商宣告全新会话 → 冷缓存。前缀匹配本身不是主风险;**把书签泄漏进供应商键 / 稿首**才是。

已拍板(2026-09-07,自 c2580 伞):**缓存优先**——分叉后尽量让前缀/网关缓存仍有效,而不是「新书签 = 必冷」;供应商差异用**命名策略**分流(`compat` / WirePolicy 轮廓一类),**禁止**为「未来插件」新开 trait 市场。

## What Changes

- 供应商会话键取值改由**命名策略**决定(按供应商/轮廓分流),不再默认绑书签:
  - `x-opencode-session`:fork 后的取值按策略算,子本不再必然换新 header 断开网关会话
  - `prompt_cache_key` / `previous_response_id`:打开时按策略取值,保证同树干共享
- 硬短路(违反即错):
  - 禁止书签 UUID 进模型稿(稿首 0 前缀炸弹)
  - 禁止书签 UUID 当 `prompt_cache_key`
  - 禁止子本继承父本**切点之后**的 `previous_response_id`
- 不保证命中率,只保证**不主动放弃**缓存(c2570 已保证「上游报了读数就不假装 NotApplicable」)

## Capabilities

- `package-ai-bridge`:WirePolicy / attribution 的键策略旋钮
- `agent-session-store` / `agent-session`:fork 与书签语义(只读对照;本刀不改 live spec)

## Impact

- 产品:分叉仍是「新书签 + 共享历史心智」;缓存与观测不再偷偷绑死书签
- 供应商:前缀匹配族(官方 DeepSeek、本机 llama.cpp)与「按 session header 路由」族分开叙事
- 不改 fork 拷贝路径、不动观测双 id 字段(那是 c2600)

## Further Notes

- 姊妹切片:观测双 id `c2600-add-obs-dual-session-identity` 先钉字段;本刀落地后按命名策略填满其 `llm_id`(本刀未落地时 `llm_id` 暂等于书签,字段不缺)。切片 A = `c2590-fix-obs-session-per-generate` 已归档。
- 原伞草案 `c2580-add-session-identity-split` 已移除:三切片全部物化(A=归档、C=c2600、B=本刀)。
- 调研笔记:`research/identities.md`(本目录,自 c2580 移入)。
