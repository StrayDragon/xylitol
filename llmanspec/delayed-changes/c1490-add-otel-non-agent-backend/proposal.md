---
change_id: c1490-add-otel-non-agent-backend
title: 非 agent/LLM 时间线的 OTEL 后端选型与分流
status: purpose-draft
priority: 1490
apply_band: P9-deferred
depends_on:
  - c1475-add-otel-export
author: agent
---

# c1490-add-otel-non-agent-backend

> **⚠️ 已融合升格** → 活跃草案 [`llmanspec/changes/c1870-add-otel-llm-infra-split/`](../../changes/c1870-add-otel-llm-infra-split/proposal.md)（2026-08-03）。
> 本文件仅作 delayed 索引保留；**勿**再按 c1490 / P9-deferred 单独实施此处正文。
> c1870 含：Collector 分流（原 c1490）+ `xylitol.signal` + manual `prepare` 早退不得污染 Langfuse；证据与交接说明见该 proposal「Background / 交接说明」。

## Why（历史）

agent/LLM 观测进 Langfuse；非 LLM 时间线需独立 OTel track（Collector → Tempo/Grafana）。全文意向见 **c1870**。

## Purpose（历史 · 已融合）

原 purpose-draft；现并入 c1870（分流架构 + compaction 门闸失败边界）。
