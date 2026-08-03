---
depends_on:
  - c1860-refactor-context-token-settlement
---

# OTel LLM / infra 车道（obs.lane + 门闸压缩）

> **规划壳**：架构心智已写入 [`docs/architecture/进程内观测.md`](../../../docs/architecture/进程内观测.md) 与 [`docs/roadmaps/OTEL与Langfuse观测.md`](../../../docs/roadmaps/OTEL与Langfuse观测.md)；本 change 为落地 **M-lane**（及文档侧 Collector 示例）的实现载体。
> **融合**：原 delayed c1490 已删除；禁止平行提案。姊妹 UX：**c1875**（本 change 不做文案）。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

今日 fastrace → 单一 OTLP 常直连 Langfuse，**凡导出 ≈ 进 Langfuse**。manual force compact 在 `prepare` 前开 `agent.compaction`，早退仍产生独立 ERROR 根（无 `llm.request`），污染 LLM 工作区与数据集。

需要可扩展基础：

1. **多总线正交**：`XyEvent` / hooks / fastrace 并列，禁止「万能 signal」兼 UI。
2. **观测只走 fastrace → 单一 OTLP**；禁止第二套 OTel/`tracing` span 栈。
3. **消费分流属性** `xylitol.obs.lane=llm|infra`（弃用草案名 `xylitol.signal`）；Collector 过滤或直连时应用侧降噪。
4. **otel19「实际执行」** = 过 `prepare_compaction` 之后；manual 与 auto 均 prepare-first。

证据 session：`460ad16e-874f-418f-8ca0-dabc58f89320`（2026-08-03 Langfuse 独立 ERROR compact 根）。

## What Changes

- 收紧 `infra-otel` otel19 + 新增 lane / 直连降噪合约；CollectingReporter 单测；不扩可执行 BDD。
- manual `compact`：**prepare-first**；过闸后建 span 且 `xylitol.obs.lane=llm`；早退不建 `agent.compaction` OTLP（`XyEvent` Compaction* 面语义本 change 可保持或仅随代码最小对齐，UX 文案归 c1875）。
- 主路径 llm span（turn / iteration / llm.request / tool / 真·compaction）携带 `lane=llm`（实现选统一 helper）。
- 运维：Collector 按 `xylitol.obs.lane` 过滤的示例（`configs/examples/` 或 docs 片段）；应用**不做**双 exporter。
- 同步 architecture「理想 vs 现状」在本 change 归档时把已兑现段迁清（apply/archive 时）。

## Capabilities

- `infra-otel`
- `domain-compaction`（仅当需写明「何时算实际执行 / 是否开 span」）

## Impact

- Langfuse 更干净；失败体验仍可经宽 OTel / 日后 Tempo。
- 运维可选加 Collector；默认 opt-in 与坏配置不挡主路径不变。

## Out of scope

- 应用内双 exporter；Tempo/Grafana 托管部署
- 把 `obs.lane` 接到 `XyEvent` / hooks
- c1875 force 文案 / 重压 summary
- 采样实现、`obs.domain`、M5 子进程出站、OTel Metrics
- 复活 delayed c1490

## Ethics

- risk_level: low
- prohibited_actions: 第二套 span 栈；lane 兼 UI；默认全量敏感载荷出站；偷渡 c1875
- required_evidence: prepare 早退不进 Langfuse agent 语义可测；过闸 compact 仍有 span+lane；默认 none/opt-in
- refusal_contract: 不做「凡失败都灌 Langfuse」
- escalation_policy: 无（Collector 已锁定为外部分流；非双 exporter）
