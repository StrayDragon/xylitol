# c2924 实施任务

- [x] 1. `agent-runtime.feature`：新增 `@req` 规则 `runtime-phase-transition-table` + 场景大纲 `phase-transition-matrix`（9 行：7 正转移 + 2 守卫相），列 from/event/guard/to
- [x] 2. `llman-sdd validate agent-runtime`：check_transitions E1-E4 过（E2 需 guard 列消解同 from+event 冲突）
- [x] 3. `tests/bdd/steps_agent_runtime.rs`：三步（given 初态+双 run_id 预置 / when 事件+守卫 / then phase 观测），RunCoordinator 直驱
- [x] 4. `tests/bdd/bindings_agent_runtime.rs`：sync fn outline binding
- [x] 5. `cargo test --test bdd`：9 case 全绿；`cargo test --lib` state 单测不回归
- [x] 6. 门禁：fmt/lint + `llman-sdd validate --all --strict`，tick checkbox，finalize
