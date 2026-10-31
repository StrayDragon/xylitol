# 结构化输出与能力元数据：权威惯例（2026-10-31）

来源：models.dev `api.json`、Azure OpenAI 文档、`@anthropic-ai/sdk` 类型、LiteLLM、pi-ai README。

## models.dev 字段

- 每模型：`reasoning`、`reasoning_options`、`tool_call`、`structured_output`（布尔）、`modalities.input/output`、`limit.context/output`、`cost`（含 `cache_read` / `cache_write` 定价）、`temperature`、`release_date`。
- 缓存能力只以定价编码，无独立布尔；结构化输出有显式布尔。
- 样例：`claude-sonnet-4-5` `limit.output=64000`；`gpt-5-nano` `limit.output=128000`。

## OpenAI 结构化输出

- Chat Completions：`response_format={type:"json_schema",strict:true}`；Responses：`text.format`。
- strict 要求：所有字段 required、`additionalProperties:false`、JSON Schema 子集（root 不可 anyOf）。
- 与并行工具调用冲突：structured outputs 不支持 parallel function calls，须 `parallel_tool_calls:false`。

## Anthropic 结构化输出

- 原生 `output_format`（beta 头 `structured-outputs-2025-11-13`）；LiteLLM 将 `response_format` 翻译为 `output_format`，无原生支持退化为 json_tool_call。

## max output tokens

- Responses `max_output_tokens` 可选（含 reasoning tokens）；Chat `max_completion_tokens`；Anthropic `max_tokens` 必填。
- 目录默认来自 models.dev `limit.output`；config 覆盖。

## 对 xylitol 的影响

- 三族映射与惯例一致。
- 缺口：设置 `structured_output` 且带 tools 时未发 `parallel_tool_calls:false`，OpenAI 族可能拒绝；需新 SDD change。
- Anthropic 侧 `structured-outputs-2025-11-13` beta 头未发送；DeepSeek 兼容端实测忽略 `output_config.format`。
