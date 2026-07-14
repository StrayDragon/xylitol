# Tasks — c990-add-test-hooks-wiring-bdd

- [x] 1. `llman sdd validate c990-add-test-hooks-wiring-bdd --no-interactive`
- [x] 2. `src/lib.rs`（及 crate 文档）：精选导出 `XyHookBus`、`XyHookOutcome`、`NoopHookBus`；`embed` 模块注释提及 hook 库端口
- [x] 3. 新增 `tests/features/hooks-wiring.feature`：三类大纲骨架；启用「确保新会话」→`session_start` 观察例子
- [x] 4. `tests/bdd.rs`：操作字典 + 录制/脚本断言步骤；`make_agent_with_store` 注入 `hook_bus`；未知操作名可读失败
- [x] 5. Smoke：`InProcessDriver` + 孤儿 session + `session_tree`（或等价库 API）；断言 hook 调用与 `reason`；禁止裸 `dispatch` 冒充
- [x] 6. design 表含 c996/c998 预留；feature 注释未接线行
- [x] 7. `cargo test --test bdd -- --test-threads=1`；既有 hooks + wiring smoke 绿
- [x] 8. `llman sdd validate c990-add-test-hooks-wiring-bdd --strict --no-interactive`；勾选本清单
