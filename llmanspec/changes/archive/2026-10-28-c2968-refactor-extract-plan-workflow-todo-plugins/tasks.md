# Tasks

- [x] 1. **specs**（spec）：`runtime-config` 增 `todo` / `workflow` / `plan` 顶层节与旧键拒绝；`workspace-assembly` 记录能力插件边界；`agent-tools` / `agent-runtime` / `agent-session` 更新工具闭集与注册场景；`src/AGENTS.md` 同步。测：`llman-sdd validate --all --strict`。
- [x] 2. **todo 插件**（plugin）：新建 crate，迁移 `TodoProgressMiddleware` / `TodoReminderMiddleware` / `TodoCheckpointMiddleware`、`todo_tools`、`SessionAgentTodoGateway` 与 `TodoConfig`。测：注册/卸载、工具归属。
- [x] 3. **workflow 插件**（plugin）：新建 crate，迁移 `WorkflowMiddleware` 与 `WorkflowConfig`。测：注册/卸载。
- [x] 4. **plan 插件**（plugin）：新建 crate，迁移 `PlanModeMiddleware`、`plan_tools`、`SessionPlanGateway` 与 `PlanConfig`；todo 交接缺失时结构化降级。测：注册/卸载、缺 todo 降级。
- [x] 5. **瘦身 loop-example / tools**（plugin）：`loop-example` 只留示例、自愈、评审、工具收窄；`tools-builtin` 只留核心工具与 `ask`。测：工具名闭集。
- [x] 6. **配置与接线**（config/kernel）：schema/example 新段；旧键拒绝并提示迁移；kernel 按配置装配中间件与工具集。测：schema 漂移、旧键错误、默认 todo+plan on / workflow off。
- [x] 7. **BDD**（tests）：注册场景迁到各自插件；默认开关断言。测：`cargo test --all-features --test bdd`。
- [x] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
