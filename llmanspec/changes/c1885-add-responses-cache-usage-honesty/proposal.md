---
depends_on: []
---

# Responses usage：Prompt Cache 诚实透出

> **调研底稿**：[`docs/research/responses-context-layout-and-cache-2026.md`](../../../docs/research/responses-context-layout-and-cache-2026.md)
> **自包含**：只改 usage 映射与产品诚实叙事；不依赖 Assembler / Responses-only 边界落地。

## Why

当前 `from_responses_usage` 将 `cache_read` 置 0，而 Completions 路径已映射 `cached_tokens`。没有尺子就无法验证「稳定前缀 / tool_search / 状态栏策略」是否真省、是否伤任务。目标是**可观测与诚实**，不是最大化命中率。

同为 Responses 形似端，**未必回报** `cached_tokens`（flavor 差异，见 `c1880` / research §5.1）。映射逻辑须按 capabilities/flavor：**有则透出，无则标未回报**，禁止对 DeepSeek/llama.cpp 等伪造官方字段语义。

## What Changes

- Responses usage 映射 `input_tokens_details.cached_tokens`（及官方文档存在的 write 类字段，若无则保持 0 并文档说明）。
- **按 flavor/capabilities 分支**：未声明 `prompt_cache_usage` 的端点 → 不解析「假字段」、footer 标不适用/未回报。
- Footer / 观测 / provenance：区分「读到缓存」vs「网关未回报」vs「不适用」；**禁止伪造** cache hit。
- 可选：当 capabilities 声明且配置提供时，请求可带 `prompt_cache_key`（本 change 可只做透传钩子 + 文档；键策略细节可后置；flavor 不支持则不发）。
- 单测：fixtures 覆盖 Responses usage JSON → `AiBridgeUsage`；至少一档「无 cache 细节的兼容端」诚实路径。

## Capabilities（意向）

- `package-ai-bridge`（usage）
- `domain-compaction` / token estimate 仅当 footer 同源读数需对齐时
- 产品：`压缩与上下文` / roadmap「上下文缓存」M1 叙事

## Impact

- 后续 ContextPolicy / tool_search 可用真实 cache 读数做对照。
- 用户与排障可见「这次是否吃到前缀缓存」。

## Out of scope

- 改变 system/tools 布局或状态栏（→ `c1890`/`c1895`）
- 强制开启某 cache 策略
- Anthropic `cache_creation_*` 主路径（桩即可）
- Eval 回归闸（后置）

## Parallel / depends

- `depends_on: []`
- 与 `c1880` 无硬依赖；建议同期合并评审文档用语

## Open Questions

- TUI footer 文案粒度（精确数字 vs 「有 cache 读」）—— propose 时与 chrome 词汇对齐。

## Ethics

- risk_level: low
- prohibited_actions: 伪造 cache hit；把启发式标成 Api cache
- required_evidence: Responses fixture 映射单测；未回报时诚实标注
- refusal_contract: 不把 hit rate 写成产品 KPI
- escalation_policy: 无
