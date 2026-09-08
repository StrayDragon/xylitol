# Tasks: c2610-fix-obs-session-idle-span-bleed

测试 seam：复用既有单测 seam——`ObsGateScope` / `SpanCollectScope` / `ObsSessionScope`（otel19–23 先例，CollectingReporter 收集）；不打网；不新扩 BDD step。

- [x] t1 specs：`infra-otel` 新增 otel24（单次处理全树 `langfuse.session.id` 快照一致，含 iteration / tool / compaction / error 与 summarizer 的 llm.request）与 otel25（观测槽仅 writer 绑定路径可写，reader 物化 MUST NOT 写）。otel23 / otel6 语义不变。`llman sdd validate` 结构绿
- [x] t2 `agent` 观测 span 快照化：`AgentIterationSpan` / `ToolExecuteSpan`（含 BarrierParallel 扇出）/ `react.error` / `tool.error` / `AgentCompactionSpan` 从调用链接收 `ObsSessionContext`；compaction summarizer 的 generate options 用调用方快照替换 `obs_session_context()`；无 run 上下文的 slash compaction 允许槽回退
- [x] t3 host reader 物化不写观测槽（`materialize_reader` / `new_reader_driver` 路径）；writer bind 行为不变
- [x] t4 单测：bind B 后把槽改写为 A（模拟 reader / 他路踩槽），B 的 iteration / tool / compaction / error span 的 `langfuse.session.id` 仍为 B；reader 物化前后槽不变。`cargo test` 相关单测 + `llman sdd validate c2610-fix-obs-session-idle-span-bleed --strict --no-check`
