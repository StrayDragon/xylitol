# Tasks — c25-fix-compact-overflow-placeholder

## 测试 seam（复用既有 harness，不新发明）

1. **wire 投影**：`protocol::wire::event` 往返单测（既有 `*_roundtrips_through_wire_event` 测试族）。
2. **compaction 领域**：`find_cut_point` / `prepare_compaction` / `compact_session` / orchestrator 单测（in-memory `SessionManager` + fake model，`src/agent/compaction/*` 既有测试族）；`tests/bdd/bindings_domain_compaction.rs` 既有 @executable 绑定随 spec 场景同步。
3. **token 估算**：`token_estimator` 表驱动单测（provenance 分流）。
4. **driver 层**：in-process / remote driver `estimate_context_tokens` 行为测试（`src/app/core/driver/` 既有测试位）。

## Tasks

### T1 wire：CompactionEnd 载荷往返（已完成）

- `Event::CompactionEnd` 增加 `result: Option<String>` / `aborted: bool` / `reason: String` / `will_retry: bool` / `error_message: Option<String>` / `summary: Option<String>` / `tokens_before: Option<u64>`（serde 缺省兼容）。
- `to_wire_event` / `TryFrom<&Event> for XyEvent` 全字段投影；删除反序列化侧「伪成功」重建。
- 往返单测：成功（summary+tokens）、失败（error_message）、aborted 三形态；旧无字段 JSON 仍可解码为 None 载荷不 panic（`compaction_end_payload_roundtrips_through_wire_event` / `legacy_bare_compaction_end_decodes_to_default_payload`）。
- 撑 spec：`protocol-app`（CompactionEnd 载荷往返 req，pa-wire3）。

### T2 估算：fixed_context 计入 + 三处 settlement 同源（已完成）

- `EstimateOpts.fixed_context: Option<FixedRequestContext>`（system prompt + tool schemas）；Heuristic / LocalTokenizer 路径计入伪行，Api 锚点路径不叠加（`fixed_context_folds_into_non_api_only` 表驱动断言）。
- 陈旧锚点丢弃：usage 锚点时间戳不新于最新 CompactionEntry 时回落 per-message 计量（`usage_anchor_not_newer_than_compaction_is_dropped`）。
- 接线：`settle_turn_context` / `try_turn_end_compaction`（ReActConfig 的 system_prompt + tool_schemas 构造 `FixedRequestContext`）；orchestrator `emit_after_compaction_settlement`（占位值 = summary 行 + 保留尾 + fixed_context）；in-process driver `estimate_context_tokens`。
- AfterCompaction 占位值端到端：`after_compaction_settlement_reports_post_cut_placeholder`（92fa9adf 形态 → settlement ≈ 19.2k < window，非陈旧 90k；fake model 仅摘要调用）。
- 撑 spec：`domain-compaction` c16 / c26 修订。

### T3 切点：keep 预算 clamp（已完成）

- `effective_keep_budget(settings, window, fixed_overhead)` helper；`prepare_compaction` / `compact_session` 共用（签名加 `context_window` / `fixed_overhead_tokens`）。
- orchestrator 三入口（force / threshold / overflow）把 `context_window` 与 fixed_context 下探到 compact_session；window=0 或无 fixed_context 时退化 `keep_recent_tokens`（`keep_budget_clamp_degenerates_without_window_or_overhead`）。
- 回归单测：`keep_budget_clamp_moves_cut_off_first_entry_on_small_windows`（92fa9adf 形态 → 旧口径 prepare 报 nothing-to-compact（bug 复现），clamp 后切点移出首条）。
- 撑 spec：`domain-compaction` c8 修订（clamp 子句）。

### T4 remote 同源：EstimateContext 命令 + leaf_changed settlement（已完成）

- `Command::EstimateContext`（serde tag `estimate_context`，四象限 unary Readonly/Result/Queued）+ REGISTRY 行 + exec_class 守卫 + in-process 分发（`DispatchOutcome::EstimateContext`）+ server `outcome_to_value`。
- remote driver `estimate_context_tokens` 改走该命令；删除客户端 GetMessages + 本地估算路径。
- host 在 `switch_session`（resume / 激活）尾发一次 `ContextTokenSettlement(reason=leaf_changed)`（`AgentCapabilities::emit_leaf_changed_settlement`，含 fixed_context）。
- `kick_footer_token_refresh` 改走 `XyDriver::estimate_context_tokens` seam（本地/remote 同源）；顺带修 c1135 测试对长 checkout 路径的脆弱断言（footer cwd 截断的 `...` 与本测试无关）。
- 撑 spec：`protocol-app`（pa-map5）、`server-core`（sr-est1）。

### T5 门禁收口（已完成）

- `just fmt` / `just lint` / `cargo test -p xylitol --lib --all-features`（1911+ 通过；仅环境性 flaky 已修）/ `just test-tui` 全绿；`llman sdd validate c25-fix-compact-overflow-placeholder --strict --no-interactive` 全绿。
