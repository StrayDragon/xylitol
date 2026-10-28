# Tasks

- [ ] 1. **specs**（spec）：`runtime-config` 改为 `todo` / `workflow` / `plan` 段；`workspace-assembly` 记录新 crate 与依赖方向；`agent-todo` / `agent-workflow` / `agent-plan` 改归属措辞；`src/AGENTS.md` 同步。测：`llman-sdd validate --all --strict`。
- [ ] 2. **todo 插件**（plugin）：新建 crate，迁移 `TodoProgressMiddleware` / `TodoReminderMiddleware` / `TodoCheckpointMiddleware`、`todo_tools`、`SessionAgentTodoGateway` 与配置。测：注册/卸载、工具归属。
- [ ] 3. **workflow 插件**（plugin）：新建 crate，迁移 `WorkflowMiddleware` 与 workflow 工具/配置。测：注册/卸载、工具归属。
- [ ] 4. **plan 插件**（plugin）：新建 crate，迁移 `PlanModeMiddleware` 与 plan 工具/配置；从 `Ctx` 解析 `AgentTodoGateway` / `AskUserGateway`，缺失时结构化降级。测：服务缺失降级。
- [ ] 5. **瘦身 loop-example / tools**（plugin）：`loop-example` 只留示例、自愈、评审、工具收窄；`tools-builtin` 只留核心工具与 `ask`。测：工具名闭集与旧注册移除。
- [ ] 6. **配置与接线**（config/kernel）：schema/example 新段；旧键拒绝并提示迁移；kernel 按配置装配与卸载。测：schema 漂移、旧键错误、默认 todo on / plan+workflow off。
- [ ] 7. **BDD**（tests）：注册场景迁到各自插件；默认开关断言。测：`cargo test --all-features --test bdd`。
- [ ] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
