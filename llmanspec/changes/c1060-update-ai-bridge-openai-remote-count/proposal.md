---
change_id: c1060-update-ai-bridge-openai-remote-count
title: "ai-bridge：OpenAI Responses RemoteCount 全量接线"
status: purpose-draft
priority: 1060
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1060-update-ai-bridge-openai-remote-count

> **status: purpose-draft** — 产品优先 **Api > RemoteCount > LocalTokenizer > Heuristic**；
> 本地 tokenizer 也要等 turn end / 锚点才稳，不能替代远程精确 count。

## Why

c1030 accounting 优先级含 `RemoteCount`，首期以 Anthropic `count_tokens`（或 stub）满足合约。
OpenAI / 兼容端在无稳定 usage 锚点、或本地 BPE 与真实计费不一致时，仍应走 **API 侧 count**（优先于 tiktoken），与 Anthropic 对称。

本地计算（tiktoken / Heuristic）只作降级：往往要到 leaf / turn end 才有可靠落点，**不能**当作与 Api/RemoteCount 同级的「精确」来源。

## Purpose

1. 对 OpenAI Responses（及文档化的兼容端）实现 RemoteCount 路径，并接入 estimate 装配（可配置开关，失败降级）。
2. registry 标明哪些 model/api 支持 RemoteCount；失败降级到 LocalTokenizer/Heuristic，provenance 诚实。
3. 单测/集成测覆盖成功与降级；与 Anthropic `RemoteCounter` 共用 port 形状。

## Capabilities（promote 时）

- modify `package-ai-bridge-accounting` / `package-ai-bridge`（remote_count）

## Out of scope

- 改变优先级顺序（仍 Api → RemoteCount → LocalTokenizer → Heuristic）
- 抽公共 llm-types 包；把 AgentMessage 再套一层

## Ethics

- risk_level: low–medium
- prohibited_actions: 把 RemoteCount 失败标成 Api；默认静默对每次 TextDelta 打 count API
- required_evidence: 成功路径 provenance=RemoteCount；失败降级非 Api

## Depends

- **c1030-add-package-ai-bridge**（已归档）
