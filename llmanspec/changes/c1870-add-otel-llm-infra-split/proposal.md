---
depends_on:
  - c1860-refactor-context-token-settlement
---

# OTel LLM / infra 分流（融合升格 c1490）

> **草案 · 后置**：`depends_on` c1860 归档后再 `llman-sdd-propose`。
> **升格融合**：吸收并取代 delayed [`c1490-add-otel-non-agent-backend`](../../delayed-changes/c1490-add-otel-non-agent-backend/proposal.md)；**勿**再单独实施 c1490。
> **姊妹草案**：force compact 文案 / 重压产品见 [`c1875-update-force-compact-ux`](../c1875-update-force-compact-ux/proposal.md)（本 change **不**做 UX 文案）。

## 交接说明（给接手人）

本草案要解决的是 **观测拓扑**，不是「要不要记失败」。产品心智：

| 层 | 角色 | 典型消费者 |
|---|---|---|
| **OTel（广）** | 中间事件标准：用户失败体验、client/系统、MCP/bootstrap、门闸早退 | 日后 Grafana Tempo / Collector；本地 lifecycle / `provider-trace.jsonl` |
| **Langfuse（窄）** | LLM/agent 场景：turn、generation、tool、**真·**compaction（调了摘要模型或 turn 内 auto/overflow） | 数据集、对话分析、agent 优化 |

今日 xylitol `[otel]` 常 **直连 Langfuse OTLP**，没有 Collector，因此「凡 fastrace 导出」≈「进 Langfuse」。在未部署分流前，应用侧也必须避免把 **纯门闸失败** 标成 Langfuse 友好的 `agent.compaction` ERROR 根 span。

## Background（证据与现状）

### 复现证据（2026-08-03）

- **Session**：`460ad16e-874f-418f-8ca0-dabc58f89320`（本地 `~/.xylitol/sessions/<id>.jsonl`）。
- Langfuse（`LANGFUSE_HOST`，例 `http://127.0.0.1:3000`）该 session：约 27 traces / 410+ observations。
- 同日两次 idle 手动 compact：独立根 trace
  - `c11969b24fa9e149a8429e9843af0a9f`
  - `38dcd925aec7905bd179dbd9f0e0142f`
  均为 `agent.compaction`、`level=ERROR`、`statusMessage=Nothing to compact (session too small)`，**无**嵌套 `llm.request`。
- 对照：07-29 长 turn `a128da5e717e5956e7d4d70bbbe93071` 下大量 **子** `agent.compaction`（threshold/overflow）——属 agent 路径，应留在 Langfuse。

### 代码路径（读这些再改）

1. Force：`CompactionOrchestrator::compact`（`src/agent/compaction/orchestrator.rs`）——先 `CompactionStart` + `AgentCompactionSpan::start("manual")`，再 `prepare_compaction`；失败则 `finish(..., Some(err))` + `CompactionEnd`。
2. Prepare 门闸：`prepare_compaction`（`src/agent/compaction/mod.rs`）——与 reserve/~% **无关**；英文串 `Nothing to compact (session too small)` / `Already compacted`。
3. Span helper：`AgentCompactionSpan`（`src/agent/compaction/obs.rs`）；合约 `infra-otel` **otel19**（`llmanspec/specs/infra-otel/spec.toon`）。
4. 既有导出：c1475/c1480 系 + 归档 **c1700**（compaction spans）。c1700 意图是「实际执行」可观测；prepare 早退是否算「实际执行」是本 change 要收紧的语义缺口。

### 为何 footer ~20% 会误导接手人

Footer `used … · xx%/window` 是 **自动** compact 的 reserve 语境。Force `/session-compact` **不读**该闸。session `460ad16e` tip 附近已多次 compaction，~30k 多为 keep 窗内 summary；prepare 报 too-small 是「cut 后无可摘要历史」，不是「再等阈值」。产品误解归 **c1875**；本 change 只规定这类失败的 **观测归属**。

## Why

- c1490 已写 Collector→Tempo 意向，但未覆盖 **门闸失败 vs 真·agent 压缩**，也未落地 `xylitol.signal`。
- 直连 Langfuse 时，idle prepare 失败的独立 ERROR root **污染** LLM 工作区、对数据集无意义，却仍占用 session 时间线。
- 宽 OTel 仍应能表达「用户点了 compact 但门闸拒绝」——供日后 Grafana / 失败体验分析；只是 **不应**假装成 Langfuse agent span。

## What Changes

- **融合 c1490**：fastrace → 单一 OTLP；推荐 **Collector 分流**（llm → Langfuse，infra → Tempo/Grafana）；文档 + 示例 `otel-collector.yaml`（或 docs 片段）；Collector vs 双 exporter 在 propose 时用户确认后写死。
- **显式信号**：低频 span 带 `xylitol.signal=llm|infra`（名以实现为准）；Collector filter 优先属性 / instrumentation scope。
- **compaction 观测边界（必含）**：
  - `prepare` 早退 / 无可摘要历史的 manual 失败 → **infra**（或直连 Langfuse 时不建 `agent.compaction` / 不标 LLM observation）；
  - 进入摘要 LLM，或 turn 内 threshold/overflow 的成功·可重试路径 → **llm/agent**（可挂 turn 子 span）。
  - 收紧 **otel19**「实际执行」措辞，使与上表一致；单测 `CollectingReporter` 覆盖。
- **默认**：未配 Collector 仍可直连 Langfuse；应用侧 MUST 避免纯门闸失败灌进 Langfuse 语义。观测闸关闭仍为零/近零开销。
- **禁止**为 infra 再引入第二套 `tracing` 栈。

## Capabilities（意向）

- `infra-otel` — 分流属性、otel19、文档/Collector
- `domain-compaction` — 仅当需写明「何时算实际执行 / 是否开 span」
- 文档：[`docs/roadmaps/OTEL与Langfuse观测.md`](../../../docs/roadmaps/OTEL与Langfuse观测.md)（或升格到 `docs/architecture/`）补「通用 traces 后端」段

## Impact

- Langfuse session 更干净；失败 UX 仍可在宽 OTel 检索。
- 运维可选加 Collector；默认 opt-in 导出不变。

## Out of scope

- 部署 Tempo/Grafana（文档 + 推荐即可）
- 默认开启远程导出
- force 文案 / 「带 focus 重压 summary」（**c1875**）
- 自研 Inspect UI；改 ReAct 主路径合约（除非 otel19 措辞必需）

## Related history

| id | 关系 |
|---|---|
| c1490（delayed） | **被本 change 取代**；目录留索引桩 |
| c1475 / c1480 | OTLP→Langfuse 主路径（已归档系） |
| c1700 | 引入 `agent.compaction`；本 change 收紧「何时建 span」 |
| c1860 | settlement / token.estimate 降噪；**依赖归档后**再正式化本草案 |
| c1875 | UX 文案与重压产品；观测归属跟本 change，实现禁止偷渡 |

## Ethics

- risk_level: low（草案）
- prohibited_actions: 默认全量敏感载荷出站；未确认付费云为唯一路径；c1860 未归档前 apply；把 c1875 UX 塞进本 change
- required_evidence（propose/apply）: 分流或直连降噪可测；prepare 早退不进 Langfuse agent 语义；默认仍 none/opt-in
- refusal_contract: 不做「凡失败都灌 Langfuse」
- escalation_policy: Collector vs 双 exporter 须用户确认后再写死 design
