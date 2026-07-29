---
change_id: c1700-add-otel-compaction-spans
title: 将 compaction 生命周期挂入 OTEL/Langfuse 过程树
status: designed
priority: 1700
depends_on: []
author: agent
branch: sdd/c1700-add-otel-compaction-spans
base_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
checkpointed: true
checkpoint_sha: 785db08a3bb45d7c7f7c8605587c17a7ea58b0aa
---

# c1700-add-otel-compaction-spans

> **背景**：c1630–c1680 已落地 reserve/auto/overflow/instructions/TUI%；c1690 预留本姊妹提案。
> **对照**：既有 `agent.turn` / `token.estimate` 父子挂载与 Langfuse observation 属性约定（`infra-otel`）。
> **设计**：见同目录 `design.md` / `tasks.md`。

## Why

Compaction 已通过 `XyEvent::CompactionStart/End` 驱动 TUI/print，但低频 fastrace 过程树**没有**对应 span。开启 OTLP→Langfuse 时，排障者看不到「何时压缩、为何压缩、是否 will_retry、摘要 LLM 挂在哪」，只能靠 UI 文案或本地日志拼凑。须把 compact **对齐为过程树上的可观测节点**（产品词汇 span，非改 wire 事件形状）。

## What Changes

- **Span**：观测闸激活且实际执行 compact（manual / threshold / overflow）时，导出 **`agent.compaction`**；`langfuse.observation.type=span`。
- **父子**：有活跃 `agent.turn` → 子 span；无 turn → 独立根 + `langfuse.session.id`（对齐 `token.estimate` / otel13）。
- **属性**：至少 `reason`；结束时诚实暴露 `will_retry` / `aborted` / 失败 status（或等价）；默认 **不**把摘要全文写入 observation I/O。
- **嵌套**：尽量使摘要路径上的 `llm.request` 挂在该 span 下（设 obs parent），避免与主对话 generation 混成无父散点。
- **闸**：与既有低频 span 同源（`provider_trace_active`）；关闸零/近零开销。
- **顺带**：修正 `test-bdd` `valid_scope` 中已不存在的 `tests/bdd.rs` → `tests/bdd/`。
- **不做**：改 wire `CompactionStart/End` 形状；默认甩摘要正文进 Langfuse I/O；子进程出站（roadmap M5）；改触发公式 / TUI%。

## Capabilities

`infra-otel`（主）· `infra-observability`（ipt4 词汇对齐）· `test-bdd`（valid_scope 卫生）

## Impact

- Langfuse / OTLP 过程树可见 compact 时段与 reason；本地 `provider-trace.jsonl` 同步出现该 span。
- `XyEvent` 面行为不变；观测失败仍不得阻断主路径。

## Ethics

- risk_level: low
- prohibited_actions: 默认导出完整摘要正文；为观测引入 tracing 双栈；改 compaction 触发语义；无限扩 observation I/O 档位
- required_evidence: CollectingReporter 单测（挂 turn / 独立根 / 关闸 noop / reason）；live `infra-otel` req 更新通过 validate
- refusal_contract: 不做「compact 专用第二套 exporter」
- escalation_policy: 若摘要 `llm.request` parent 竞态难一次收口，可先保证 `agent.compaction` 自身挂接，嵌套 generation 作 follow-up
