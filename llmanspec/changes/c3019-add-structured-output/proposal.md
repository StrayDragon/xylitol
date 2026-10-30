---
depends_on: []
title: 结构化输出（三族请求形状）
needs_specs_change: true
---

# 结构化输出（三族请求形状）

## Why

三个方言都没有结构化输出入口：调用方无法要求模型按 JSON Schema 产出，eval 复用与工具参数强约束只能靠提示词。三族协议均已支持该能力（Responses 的 `text.format`、Completions 的 `response_format`、Anthropic 的 `output_config.format`），缺的只是生成选项与请求映射。

## What Changes

- 生成选项新增 `structured_output`（`name` / `schema` / `strict` / `description`）；未设置时 不得 改变任何请求体。
- OpenAI Responses 必须 写 `text.format = {type: json_schema, name, schema, strict}`。
- OpenAI Completions 必须 写 `response_format = {type: json_schema, json_schema: {name, schema, strict}}`。
- Anthropic Messages 必须 写 `output_config.format = {type: json_schema, schema}`；Anthropic 形状无 `name` / `strict`，只消费 `schema`。
- 本 change 只交付能力面；产品默认路径不设置该选项，行为不变。

## Capabilities

`infra-provider`：新增 r3023（结构化输出请求形状）。

## Impact

- `crates/xylitol-plugin-sdk`：`XyStructuredOutput` 与 `XyGenerateOptions.structured_output`。
- 三个方言适配器的请求组装与单测。
- 无配置面变化（不是模型条目字段）。

## 非目标

- 不做 `tool_choice` / `parallel_tool_calls`（无当前需求）。
- 不做响应侧 `parsed_output` 解析；结构化结果仍是文本，由调用方自行解析。
- 不把结构化输出接进产品默认回合。

## 不承诺

- `strict` 仅透传；OpenAI 对 schema 子集的约束（`additionalProperties: false`、全字段 required）由调用方保证。
- Anthropic 结构化输出可能仍属 beta 字段，未做能力探测。
