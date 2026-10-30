---
depends_on: []
title: 输出预算接线（配置 → ModelMeta → 生成选项）
needs_specs_change: true
branch: sdd/c3014-add-output-budget-wiring
base_branch: main
base_sha: 150ada85cd2230c943ef85489713efaa446062d1
---

# 输出预算接线（配置 → ModelMeta → 生成选项）

## Why

输出预算整条链是断的：`XyModelEntryConfig` 没有输出上限字段，`XyModelMeta.max_tokens` 恒为 0，`prepare_turn_binding` 恒把 `max_output_tokens` 填 `None`。于是 Anthropic 永远发 `max_tokens: 8192`，OpenAI 族永远省略输出上限，用户无法按模型配置预算。压缩摘要路径会自行覆盖预算，但普通回合没有可控上限。

## What Changes

- `XyModelEntryConfig` 新增可选 `max_output_tokens`（正整数）；声明值 必须 写入 `XyModelMeta.max_tokens`，未声明 必须 保持 0（未知）；0 必须 被配置解析拒绝。
- 运行时构建生成选项时 必须 把非零 `max_tokens` 填入 `XyGenerateOptions.max_output_tokens`，0 必须 保持 `None`。
- 方言适配器 必须 优先使用该选项：OpenAI Responses 写 `max_output_tokens`，Completions 写 `max_completion_tokens`，Anthropic 写 `max_tokens` 并在未配置时保留安全缺省 8192。
- 配置 schema 产物 必须 随结构再生成；示例配置模板 必须 增加该字段说明。
- 配置值 必须 优先于外部模型目录（models.dev 一类）的同名字段：本仓库不抓取外部目录，只认配置。

## Capabilities

- `runtime-model-registry`：新增 r3015（输出预算元数据与生成选项接线）。
- `runtime-config`：新增 r3016（模型条目输出预算字段）。

## Impact

- `crates/xylitol-plugin-sdk`：`model_entry` 字段与 `to_model_meta` 映射。
- `crates/xylitol-runtime`：`prepare_turn_binding` 接线。
- `plugins/xylitol-official-plugin-config`：schema 产物再生成。
- 三个方言适配器只补 body 断言，不改请求形状（字段已存在）。

## 非目标

- 不抓取或内置 models.dev 目录。
- 不改压缩摘要的预算覆盖逻辑。
- 不做缓存配置（S3）。

## 不承诺

- `max_tokens` 语义按 provider 差异（Completions 的 `max_completion_tokens` 含 reasoning tokens；Anthropic 的 `max_tokens` 含 thinking 预算）由用户自行按模型理解。
