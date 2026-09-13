# Tasks — c25-fix-compact-overflow-placeholder

## 测试 seam（复用既有 harness，不新发明）

1. **wire 投影**：`protocol::wire::event` 往返单测（既有 `*_roundtrips_through_wire_event` 测试族）。
2. **compaction 领域**：`find_cut_point` / `prepare_compaction` / `compact_session` / orchestrator 单测（in-memory `SessionManager` + fake model，`src/agent/compaction/*` 既有测试族）；`tests/bdd/bindings_domain_compaction.rs` 既有 @executable 绑定随 spec 场景同步。
3. **token 估算**：`token_estimator` 表驱动单测（provenance 分流）。
4. **driver 层**：in-process / remote driver `estimate_context_tokens` 行为测试（`src/app/core/driver/` 既有测试位）。

## Tasks

### T1 wire：CompactionEnd 载荷往返

- `Event::CompactionEnd` 增加 `result: Option<String>` / `aborted: bool` / `reason: String` / `will_retry: bool` / `error_message: Option<String>` / `summary: Option<String>` / `tokens_before: Option<u64>`（serde 缺省兼容）。
- `to_wire_event` / `TryFrom<&Event> for XyEvent` 全字段投影；删除反序列化侧「伪成功」重建。
- 往返单测：成功（summary+tokens）、失败（error_message）、aborted 三形态；旧无字段 JSON 仍可解码为 None 载荷不 panic。
- 撑 spec：`protocol-app`（CompactionEnd 载荷往返 req）。

### T2 估算：fixed_context 计入 + 三处 settlement 同源

- `EstimateOpts` 新增 `fixed_context: Option<FixedRequestContext>`（system prompt + tool schemas）；Heuristic / LocalTokenizer 路径计入伪行，Api 锚点路径不叠加（表驱动单测钉两种 provenance）。
- 接线：`settle_turn_context` / `try_turn_end_compaction`（ReActConfig 的 system_prompt + tool_schemas 下探）；orchestrator `emit_after_compaction_settlement`（占位值 = summary 行 + 保留尾 + fixed_context）；in-process driver `estimate_context_tokens`。
- AfterCompaction 占位值单测：compact 后 settlement tokens ≈ 保留尾 + overhead（不含被摘要历史）；不触发任何模型调用（fake model 计数为 0）。
- 撑 spec：`domain-compaction` c16 / c26 修订。

### T3 切点：keep 预算 clamp

- `blocked-by: T2`
- 新 helper：`effective_keep_budget(settings, window, fixed_overhead) -> u64`；`prepare_compaction` / `compact_session` 共用。
- orchestrator 三入口（force / threshold / overflow）把 `context_window` 与 fixed_context 下探到 compact_session；window=0 或无 fixed_context 时行为与现状一致（退化单测）。
- 回归单测：复刻 92fa9adf 形态（窗口 32768 / reserve 16384 / keep 20000 / chars/4 全量 15.7k / overhead ~6.5k）→ 切点 MUST 落在中后段（保留尾受 clamp 约束），compact 后投影 tokens 单调下降。
- 撑 spec：`domain-compaction` c8 修订（clamp 子句）。

### T4 remote 同源：EstimateContext 命令 + leaf_changed settlement

- `blocked-by: T2`
- `Command::EstimateContext`（unary，四象限）+ server 分发（host 计算，含 fixed_context + host tokenizer 映射）+ 方法表登记。
- remote driver `estimate_context_tokens` 改走该命令；删除客户端 GetMessages + 本地估算路径。
- host 在会话激活 / resume 完成时发一次 `ContextTokenSettlement(reason=leaf_changed)`；TUI footer 消费之。
- 撑 spec：`protocol-app`（Command 枚举 / 方法表）、`server-core`（EstimateContext unary 分发）。

### T5 门禁收口

- `blocked-by: T1, T2, T3, T4`
- `just fmt` / `just lint` / `just test` / `just test-tui` 全绿；`llman sdd validate c25-fix-compact-overflow-placeholder --strict --no-interactive` 全绿。
