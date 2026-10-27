# Tasks

- [ ] 1. **spec**（spec）：`agent-session-store` 新增 `r2941`「plan checkpoint entry」；`agent-runtime` 新增 `r2942`「plan-checkpoint 中间件」，并在 `r2925` 补注册场景。测：`llman sdd validate c2960-add-loop-plan-checkpoint --strict`。
- [ ] 2. **SDK 工件**（sdk）：新增 `session/checkpoint`（`PlanCheckpoint` / `PlanCheckpointStep` / `PlanResumeCursor` / `latest_plan_checkpoint` / `to_custom_entry_shell`）。测：往返、最新选取、损坏忽略、next step 单测。
- [ ] 3. **core seam**（sdk/runtime）：`BeforeModelCtx` / `AfterTurnCtx` 增 `session_id`；`run_react_loop` 与 `after_turn_or_fault` 填入。测：`cargo test -p xylitol-runtime --lib`。
- [ ] 4. **中间件**（plugin）：新增 `PlanCheckpointMiddleware`（order 180，`before_model` / `after_tool` / `after_turn`）；`LoopExampleConfig` 增 `plan_checkpoint`；插件解析 `dyn XySessionStore` 并注册。测：中间件单测（持久化、恢复注入一次、完成后不注入、失败 attempt、无 store 降级）+ 注册/卸载单测。
- [ ] 5. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 补 `plan_checkpoint`。测：config schema 漂移单测。
- [ ] 6. **BDD**（tests）：新增注册场景 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_plan_checkpoint_registers`。
- [ ] 7. **文档**（docs）：`src/AGENTS.md` 补「todo 给人看，checkpoint 给恢复用」。测：`just qa-doc-check`。
- [ ] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
