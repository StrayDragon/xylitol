# 完整字段统计（源仓 TOML，替代未拉全的 api.json）

> 生成自 `anomalyco/models.dev@dev` shallow clone：`providers/**/{provider.toml,models/*.toml}`。
> **不是**运行时 `https://models.dev/api.json` 字节级副本；发布 JSON 由上游构建流水线生成，字段应对齐 schema。
> 官方端点已交叉校验：见 `api-json-official-cross-check.md` / `api-json-field-stats.json`（providers=181 对齐；nested models 6231 ≫ TOML 2913）。

## 规模

| 项 | 值 |
|---|---|
| providers | 181 |
| model TOML | 2913 |
| providers 带 `api` URL | 155 |
| parse errors | 2 |

## Provider 顶栏字段频次

```
{
  "name": 181,
  "env": 181,
  "npm": 181,
  "doc": 181,
  "api": 155
}
```

## npm 分布（Top）

```
{
  "@ai-sdk/openai-compatible": 143,
  "@ai-sdk/anthropic": 9,
  "@ai-sdk/openai": 4,
  "@ai-sdk/azure": 2,
  "@aihubmix/ai-sdk-provider": 1,
  "@ai-sdk/amazon-bedrock": 1,
  "@ai-sdk/cerebras": 1,
  "ai-gateway-provider": 1,
  "@ai-sdk/cohere": 1,
  "@ai-sdk/deepinfra": 1,
  "gitlab-ai-provider": 1,
  "@ai-sdk/google": 1,
  "@ai-sdk/google-vertex": 1,
  "@ai-sdk/google-vertex/anthropic": 1,
  "@ai-sdk/groq": 1,
  "merge-gateway-ai-sdk-provider": 1,
  "@ai-sdk/mistral": 1,
  "@openrouter/ai-sdk-provider": 1,
  "@ai-sdk/perplexity": 1,
  "@qvac/ai-sdk-provider": 1
}
```

## Model 字段频次（含 cost.*）

```
{
  "cost": 2732,
  "cost.input": 2732,
  "cost.output": 2732,
  "limit": 2153,
  "name": 1889,
  "modalities": 1856,
  "last_updated": 1697,
  "cost.cache_read": 1678,
  "release_date": 1656,
  "reasoning": 1651,
  "description": 1650,
  "attachment": 1618,
  "tool_call": 1601,
  "open_weights": 1584,
  "temperature": 1390,
  "family": 1349,
  "base_model": 1346,
  "structured_output": 948,
  "knowledge": 833,
  "cost.cache_write": 637,
  "interleaved": 407,
  "cost.tiers": 185,
  "status": 154,
  "provider": 151,
  "base_model_omit": 61,
  "cost.input_audio": 52,
  "cost.reasoning": 47,
  "experimental": 23,
  "cost.output_audio": 12
}
```

## reasoning_options.type

```
{
  "effort": 1050,
  "toggle": 467,
  "budget_tokens": 349
}
```

## effort.values 出现次数

```
{
  "high": 1033,
  "low": 945,
  "medium": 927,
  "max": 377,
  "none": 361,
  "xhigh": 327,
  "minimal": 155,
  "null": 2
}
```

## 对 xylitol 的含义

1. `effort.values` 是 `thinking_levels` 建议原料的主来源；务必前加精确 `off`（c1970/c1990）。
2. 仅 `budget_tokens` 的模型 → suggestion：手写档或 map，禁止瞎编 STANDARD。
3. `npm` 多样性大 → 映射表必须小而显式；未知 npm → suggestion-only。
4. 有 `api` URL 的 provider 更可能是 openai-compatible 方言端点候选。
