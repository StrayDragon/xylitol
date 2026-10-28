# Tasks

- [x] 1. **spec**（spec）：`agent-session-store` 新增 `r2945`「plan 工件与重规划约束」；`agent-runtime` 新增 `r2946`「plan-and-execute 中间件」与注册场景。测：`llman sdd validate c2963-add-loop-plan-and-execute --strict`。
- [x] 2. **SDK 工件**（sdk）：新增 `session/plan`（`Plan` / `PlanStep` / `PlanStepStatus` / `latest_plan` / `validate_plan` / `runnable_step` / `blocked_step` / `downstream_ids` / `validate_replan`）。测：DAG 校验、runnable、下游闭包、重规划约束单测。
- [x] 3. **core seam**（sdk/runtime）：`AfterModelCtx` 增 `session_id`；`run_react_loop` 填入。测：`cargo test -p xylitol-runtime --lib`。
- [x] 4. **plan 块解析**（plugin）：解析 assistant 的 `<plan>{json}</plan>`，校验 DAG 与重规划约束后 append `CustomEntry`；非法块保留旧 plan 并记录错误。测：解析/校验/持久化单测（首次写、非法依赖、已完成锁定、失败后局部重规划）。
- [x] 5. **中间件**（plugin）：新增 `PlanExecuteMiddleware`（order 190，`before_model` / `after_model` / `after_tool` / `decide_continue`）；`LoopExampleConfig` 增 `plan_execute: PlanExecuteConfig`（`max_replans`）。测：状态机单测（runnable 注入、失败重规划、审批、完成总结、上限停止）+ 注册/卸载单测。
- [x] 6. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 补 `plan_execute`。测：config schema 漂移单测。
- [x] 7. **BDD**（tests）：新增注册场景 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_plan_execute_registers`。
- [x] 8. **文档**（docs）：`src/AGENTS.md` 补「plan 是持久工件，runtime 只是 plumbing」。测：`just qa-doc-check`。
- [x] 9. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
