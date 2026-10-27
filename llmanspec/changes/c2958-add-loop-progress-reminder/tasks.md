# Tasks

- [ ] 1. **spec**（spec）：`agent-runtime` 新增 `r2939`「进度提醒」；`r2925` 补「进度提醒启用后注册提醒中间件」场景。测：`llman sdd validate c2958-add-loop-progress-reminder --strict`。
- [ ] 2. **core seam**（sdk/runtime）：`BeforeModelCtx` 增 `todo: Option<TodoList>`；`react/mod.rs` 把状态栏读取提前并在 `before_model` 填入 todo。测：`cargo test -p xylitol-runtime --lib loop_middleware`。
- [ ] 3. **提醒中间件**（plugin）：`LoopExampleConfig` 增 `progress_reminder`；新增 `ProgressReminderMiddleware`（`before_model` + `after_tool`）；插件在开启时注册并一并注销。测：中间件单测（陈旧/完成宣称/失败/无 todo 不注入）。
- [ ] 4. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 注释补 `progress_reminder`。测：config schema 漂移单测。
- [ ] 5. **BDD**（tests）：新增注册场景的 given/then 与 binding。测：`cargo test --all-features --test bdd test_lm_progress_reminder_registers`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
