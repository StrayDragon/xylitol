# Tasks: c2921-refactor-hooks-wiring-scenario-outline

## T1 spec 大纲化：r1815 矩阵 9 场景 → 2 大纲 + 1 单例

- [x] T1.1 `llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature` r1815：`session-start`/`session-tree`/`session-shutdown`/`user-bash`/`model-select`/`thinking-select` → `场景大纲: wiring-observe-matrix`（event/op/context_key，6 行）；`tree-cancel`/`switch-cancel`/`user-bash-block` → `场景大纲: wiring-cancel-matrix`（event/op/reason，3 行）；`unknown-op` 原样保留。
- [x] T1.2 验证：`llman-sdd validate --specs` 无新增错误。

## T2 steps 参数化 + bindings 收敛 [blocked-by: T1]

- [x] T2.1 `tests/bdd/steps_hooks.rs`：新增 `#[then("hook 被调用且上下文含键 {key}")]`（strip 后复用 `_t_hook_called` + `_t_hook_context_has_key`）与 `#[given("注册了匹配 {event} 的 hook 且返回 block {reason}")]`（复用 `_g_hook_registered` + `_g_hook_returns` 组装 block JSON）；旧 5 then + 3 given 变体保留。
- [x] T2.2 `tests/bdd/bindings_misc.rs`：删除该组 7 个 binding，新增 `test_hooks_wiring_observe(agent, event, op, context_key)` 与 `test_hooks_wiring_cancel(agent, event, op, reason)`。
- [x] T2.3 验证：`cargo test --test bdd -- test_hooks_wiring` 行展开 9 用例全绿；`test_hk6_react`/`test_hk8_model`/`test_hk9_tree`/`test_hk10_user`（agent-hooks 侧）不回归。

## T3 收口验证

- [x] T3.1 `cargo test --test bdd` 全量通过（用例总数 1020）。
- [x] T3.2 `llman-sdd validate --all --strict` 通过。
