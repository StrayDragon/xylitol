# Tasks

- [x] 1. **specs**（spec）：新增 `agent-plan` capability（模式状态、计划文档、审批与 todo 交接、提示注入）；`agent-session-store` 增 `plan_doc` / `plan_mode` 条目与 md 视图；`protocol-app` 增 `set_plan_mode` 与 `planModeChanged`；`app-tui-commands` 增 `/plan`；`agent-runtime` 增计划模式中间件注册场景。测：`llman-sdd validate --all --strict`。
- [x] 2. **SDK 域模型**（sdk）：新增 `session/plan_doc.rs`（`PlanDoc` / `PlanDocStep` / `PlanDocStatus` / `latest_plan_doc` / `plan_docs` / markdown 渲染）与 `session/plan_mode.rs`（`PlanMode` / `PlanModeKind` / `latest_plan_mode`）；新增 `PlanGateway` 端口。测：往返、按 id 取最新、校验、md 渲染。
- [x] 3. **计划工具**（tools）：新增 `plan_write` / `plan_read` / `plan_list` / `plan_submit` / `plan_mode`；`SessionPlanGateway` 经 `XySessionStore` 落 CustomEntry 并写 `plans/<plan_id>.md`；`plan_submit` 经 ask 审批并调用 `AgentTodoGateway::rewrite` 交接 todo。测：写读列、批准/驳回/跳过、无 ask 网关、todo 交接失败。
- [x] 4. **模式中间件**（plugin）：新增 `PlanModeMiddleware`（order 185，`before_model` 注入 `PLAN_MODE_CONTRACT`）；`LoopExampleConfig` 增 `plan_mode` 开关；注册/卸载单测。
- [x] 5. **协议与事件**（protocol/wire/runtime/serve）：新增 `set_plan_mode` unary（toggle/plan/execute）与 `planModeChanged` 事件；runtime driver 写模式条目并发布事件；wire 双向映射；serve 路由。测：registry、事件往返、路由参数校验。
- [x] 6. **接线**（kernel）：`set_plan_mode` 绑定 driver；计划工具接入 ask/todo 网关；无 ask 场景降级。测：kernel 装配单测。
- [x] 7. **TUI 命令**（app-tui）：`/plan`、`/plan on`、`/plan off` 解析并调用 `set_plan_mode`；忙态策略与错误滚动提示。测：slash 解析快照 + 命令分派。
- [x] 8. **BDD**（tests）：用户切换模式、agent 请求被拒、批准后进入 execute 并生成 todo。测：`cargo test --all-features --test bdd`。
- [x] 9. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
