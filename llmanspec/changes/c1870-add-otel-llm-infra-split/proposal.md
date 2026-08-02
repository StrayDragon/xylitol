---
depends_on:
  - c1860-refactor-context-token-settlement
---

# OTel LLM / infra 分流（融合升格 c1490）

> **草案 → 正式化中**：c1860 已归档；本 change 为唯一 SSOT。
> **融合**：原 delayed `c1490-add-otel-non-agent-backend` 正文已并入下方；**已删除** `llmanspec/delayed-changes/c1490-…`，禁止再开平行提案。
> **姊妹草案**：force compact UX 见 [`c1875-update-force-compact-ux`](../c1875-update-force-compact-ux/proposal.md)（本 change **不**做 UX 文案）。

## 交接说明（给接手人）

本变更解决 **观测拓扑**，不是「要不要记失败」。产品心智：

| 层 | 角色 | 典型消费者 |
|---|---|---|
| **OTel（广）** | 中间事件标准：用户失败体验、client/系统、MCP/bootstrap、门闸早退 | Grafana Tempo / Collector；本地 lifecycle / `provider-trace.jsonl` |
| **Langfuse（窄）** | LLM/agent：turn、generation、tool、**真·**compaction（调了摘要模型或 turn 内 auto/overflow） | 数据集、对话分析、agent 优化 |

今日 xylitol `[otel]` 常 **直连 Langfuse OTLP**，没有 Collector，因此「凡 fastrace 导出」≈「进 Langfuse」。在未部署分流前，应用侧也必须避免把 **纯门闸失败** 标成 Langfuse 友好的 `agent.compaction` ERROR 根 span。

## Background（证据与现状）

### 复现证据（2026-08-03）

- **Session**：`460ad16e-874f-418f-8ca0-dabc58f89320`（本地 `~/.xylitol/sessions/<id>.jsonl`）。
- Langfuse（`LANGFUSE_HOST`）该 session：约 27 traces / 410+ observations。
- 同日两次 idle 手动 compact：独立根
  `c11969b24fa9e149a8429e9843af0a9f`、`38dcd925aec7905bd179dbd9f0e0142f`
  均为 `agent.compaction` ERROR、`Nothing to compact (session too small)`，**无**嵌套 `llm.request`。
- 对照：07-29 长 turn `a128da5e717e5956e7d4d70bbbe93071` 下大量 **子** `agent.compaction`（threshold/overflow）——应留在 Langfuse。

### 代码路径（读这些再改）

1. Force：`CompactionOrchestrator::compact`（`src/agent/compaction/orchestrator.rs`）——先 `CompactionStart` + `AgentCompactionSpan::start("manual")`，再 `prepare_compaction`；失败则 span finish + `CompactionEnd`。
2. Prepare：`prepare_compaction`（`src/agent/compaction/mod.rs`）——与 reserve/~% **无关**。
3. Span：`AgentCompactionSpan`（`src/agent/compaction/obs.rs`）；合约 `infra-otel` **otel19**。
4. 历史：c1475/c1480 导出；归档 **c1700** 引入 compaction spans；**c1860** 降噪 token.estimate。

### 原 c1490 分流意向（已吸收，勿另开 change）

```text
xylitol (fastrace → 单一 OTLP/HTTP)
        │
        ▼
 OpenTelemetry Collector
        ├─ filter: xylitol.signal=llm / gen_ai.* / langfuse.*
        │     → Langfuse (/api/public/otel)
        └─ 其余（infra）
              → Grafana Tempo（或 Jaeger）
```

默认仍可直连 Langfuse；需要 infra 观测时把 endpoint 指 Collector（或双 exporter——propose 时锁定一种）。

### 为何 footer ~20% 会误导

Force **不读** reserve 闸。`460ad16e` tip 附近已多次 compaction；too-small = cut 后无可摘要历史。产品误解归 **c1875**。

## Why

- 直连 Langfuse 时 prepare 早退的独立 ERROR root 污染 LLM 工作区、无数据集价值。
- 宽 OTel 仍应能表达「用户点了 compact 但门闸拒绝」——供 Grafana / 失败体验；不应假装成 Langfuse agent span。
- 需要 `xylitol.signal` + 收紧 otel19「实际执行」语义；原 c1490 仅写了 Collector，未覆盖门闸边界。

## What Changes

- **Collector 分流文档 + 示例**（或双 exporter，design 锁定一种）；fastrace → 单一 OTLP；禁止第二套 tracing 栈。
- **`xylitol.signal=llm|infra`**（名以实现为准）；Collector filter 优先属性。
- **compaction 观测边界**：
  - prepare 早退 / 无可摘要历史的 manual 失败 → **infra**（直连 Langfuse 时不建 `agent.compaction` 或等价不进 LLM 语义）；
  - 进入摘要 LLM，或 turn 内 threshold/overflow → **llm/agent**。
  - 收紧 **otel19**；`CollectingReporter` 单测覆盖。
- 观测闸关闭仍为零/近零开销；默认 opt-in 导出不变。

## Capabilities（意向）

- `infra-otel` — 分流属性、otel19、文档/Collector
- `domain-compaction` — 仅当需写明「何时算实际执行 / 是否开 span」
- 文档：[`docs/roadmaps/OTEL与Langfuse观测.md`](../../../docs/roadmaps/OTEL与Langfuse观测.md) 与/或 `docs/architecture/进程内观测.md`

## Impact

- Langfuse 更干净；失败 UX 仍可在宽 OTel 检索。
- 运维可选加 Collector。

## Out of scope

- 部署 Tempo/Grafana（文档即可）
- 默认开启远程导出
- force 文案 / 重压 summary（**c1875**）
- 自研 Inspect UI

## Related history

| id | 关系 |
|---|---|
| c1490（原 delayed） | **已融合删除**；内容见上「原 c1490」段 |
| c1475 / c1480 | OTLP→Langfuse 主路径 |
| c1700 | 引入 `agent.compaction`；本 change 收紧「何时建 span」 |
| c1860 | 已归档；settlement 降噪 |
| c1875 | UX；观测归属跟本 change |

## Ethics

- risk_level: low
- prohibited_actions: 默认全量敏感载荷出站；未确认付费云为唯一路径；把 c1875 UX 塞进本 change；复活 delayed c1490 目录
- required_evidence（propose/apply）: 分流或直连降噪可测；prepare 早退不进 Langfuse agent 语义；默认仍 none/opt-in
- refusal_contract: 不做「凡失败都灌 Langfuse」
- escalation_policy: Collector vs 双 exporter 须确认后再写死 design
