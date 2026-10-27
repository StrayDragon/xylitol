# Tasks

- [x] 1. **spec 场景**（spec）：在 `agent-runtime.feature` 的 `r2925` 下新增「计划模式启用后注册计划中间件」，并补一句「示例插件可经配置启用 plan-as-state 中间件」。测：`llman sdd validate c2957-add-loop-example-plan-state --strict`。
- [x] 2. **配置字段**（code）：`LoopExampleConfig` 新增 `plan_state: bool`（缺省 false）；更新构造点。测：`cargo test -p xylitol-official-plugin-loop-example`。
- [x] 3. **中间件实现**（code）：新增 `PlanStateMiddleware`（`before_turn` / `after_tool` / `decide_continue` / `after_turn`），解析 `todo_rewrite` / `todo_update` 结果维护进度；插件在 `plan_state` 开启时注册并一并注销。测：中间件单测 + 插件注册/卸载单测。
- [x] 4. **配置产物**（config）：`just gen-config-schema` 重新生成 `configs/config.schema.json`；`configs/example.yaml` 注释补 `plan_state`。测：config 插件的 schema 漂移单测。
- [x] 5. **BDD 覆盖**（tests）：新增 given/then step 与 scenario binding；断言注册表含 `loop-example` 与 `plan-state`。测：`cargo test --all-features --test bdd test_lm_plan_state_registers`。
- [x] 6. **全量门禁**（repo）：`just qa`；确认全绿且测试/BDD 计数符合预期。测：`just qa`。
