# Tasks: c2920-refactor-compaction-trigger-scenario-outline

## T1 spec 大纲化：r1406 触发阈值 5 场景 → 2 场景大纲

- [ ] T1.1 `llmanspec/specs/domain-compaction/domain-compaction.feature`：将 `need-compact` / `no-compact` / `disabled-no-compact` 收敛为 `场景大纲: threshold-reserve-gate`，`例子:` 表列 `tokens / enabled / expect_compact`；将 `floor-threshold-holds` / `floor-cross-triggers` 收敛为 `场景大纲: floor-aware-threshold`，`例子:` 表列 `window / reserve / keep / overhead / tokens / expect_compact`。步骤文本复用既有 given/when/then（`会话消息估算使用 <tokens> 个 token` 等占位符替换形态）。
- [ ] T1.2 验证：`llman-sdd validate c2920-refactor-compaction-trigger-scenario-outline --strict` 通过；`llman-sdd validate --specs` 无新增错误。

## T2 bindings 与 steps 收敛 [blocked-by: T1]

- [ ] T2.1 `tests/bdd/steps_compaction.rs`：新增 `#[given("compaction enabled 为 {flag}")]"`（`flag` 经 `.parse::<bool>()`），内部复用 `_g_comp_enabled_true/_false` 逻辑；原两条无参 given 保留。
- [ ] T2.2 `tests/bdd/bindings_domain_compaction.rs`：删除 `test_compaction_need` / `test_compaction_not_needed` / `test_compaction_disabled` / `test_comp_floor_holds` / `test_comp_floor_cross` 五个 binding，新增两个 outline binding `test_compaction_threshold_gate(agent, tokens, flag, expect, ws)` 与 `test_comp_floor_outline(agent, tokens, expect, ws)`（签名参数名=Examples 列名）。
- [ ] T2.3 验证：`cargo test --test bdd -- test_compaction test_comp_floor` 行展开用例数 = 5 且全绿（3 + 2）。

## T3 收口验证

- [ ] T3.1 `cargo test --test bdd` 全量通过（用例总数 ≥ 基线 1020）。
- [ ] T3.2 `llman-sdd validate --all --strict` 通过。
