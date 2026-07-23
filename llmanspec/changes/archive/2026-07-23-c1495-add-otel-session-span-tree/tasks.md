# Tasks: c1495-add-otel-session-span-tree

## Seams（实现与验收边界）

- fastrace `SpanRecord` 父子 / `trace_id`（收集 Reporter 或现有 file reporter 属性）
- agent obs API：`agent.turn` / `agent.iteration` / `tool.execute` + parent 参数
- bridge：`llm.request`（原 `ProviderRequestTrace`）接受 optional parent
- 既有闸：`provider_trace_active`；OTLP 仍仅 `[otel]` opt-in
- BDD：`infra-otel` / `infra-provider-trace` live feature 场景（结构门禁）；远端 Langfuse UI 为手工验收

## A — 命名与 API 壳

- [x] 将导出名改为 `agent.turn` / `agent.iteration` / `llm.request`；退役 `react.stream` / `react.turn` / `provider.request` 导出名
- [x] `token.estimate`：有 parent turn 则 child，否则独立 root + session id
- [x] 更新 `infra-otel` / `infra-provider-trace` 相关单测与 JSONL 窄读假设中的旧名

## B — 父子树接线

- [x] 在用户 turn 边界创建并持有 `agent.turn` 根 span
- [x] ReAct 每步 / tool / llm.request 经 `enter_with_parent`（或等价 `SpanContext`）挂到正确父节点；禁止活跃 turn 下 random 无关 root
- [x] bridge `ProviderRequestTrace::start`（或改名）接受可选 parent；agent 经参数传入，不破坏 agent↛infra
- [x] 单测：同 turn 下子 span 共享 `trace_id` 且 parent 链指向 turn/iteration

## C — 合约与文档

- [x] live specs 已含 otel11–13 与 ipt4/otel6–10 修订（本 propose）；feature 场景与实现对齐（实现后补齐 harness 若需要）
- [x] 更新 `docs/roadmaps/OTEL与Langfuse观测.md`（树 vs session；产品导出名）；必要时补 architecture 一句「已兑现树」意向（落地归档后再迁）
- [x] `llman sdd validate c1495-add-otel-session-span-tree --strict --no-interactive`（apply 完成后；propose 时 pending tasks 在 strict 下为 ERROR 属预期）
- [x] `cargo test -p xylitol-ai-bridge` + agent obs / react 相关单测；`just lint` 触及路径
- [x] 树结构验收：`agent::runtime::obs::tests::turn_iteration_llm_share_trace_id`（本地 SpanRecord 父子链；避免反复打远端 Langfuse 耗资源）
