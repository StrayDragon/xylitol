# Tasks

- [x] 1. **specs**（spec）：`agent-runtime` 把 `progress-reminder` → `todo-reminder`、`plan-checkpoint-middleware` → `todo-checkpoint-middleware`，示例插件场景与术语同步；`agent-session-store` 把 `plan-checkpoint-entry` → `todo-checkpoint-entry`、`plan-artifact` → `workflow-artifact` 并改 `custom_type` 文案；新增 `agent-workflow` capability 承接原 `plan-and-execute` 规则（`r2946`）。测：`llman-sdd validate --all --strict`。
- [x] 2. **SDK 类型**（sdk）：`session/plan.rs` → `session/workflow.rs`；`Plan`/`PlanStep`/`PlanStepStatus` → `Workflow`/`WorkflowStep`/`WorkflowStepStatus`；`CUSTOM_TYPE_PLAN` → `CUSTOM_TYPE_WORKFLOW`；helper 同步改名。`checkpoint.rs` 的 `PlanCheckpoint*` → `TodoCheckpoint*`，`CUSTOM_TYPE_PLAN_CHECKPOINT` → `CUSTOM_TYPE_TODO_CHECKPOINT`。测：DAG 校验、runnable、下游闭包、重规划约束单测。
- [x] 3. **一次性读取迁移**（sdk）：`latest_workflow` 读取 `custom_type = "workflow"` 与旧 `"plan"`；`latest_todo_checkpoint` 读取 `"todo_checkpoint"` 与旧 `"plan_checkpoint"`；新写入只用新名，不做永久别名。测：旧条目可读、新写入用新键。
- [x] 4. **插件**（plugin）：`loop-example` 配置键与中间件 id/类型改名：`plan_state` → `todo_progress`、`plan_execute` → `workflow`、`plan_checkpoint` → `todo_checkpoint`、`progress_reminder` → `todo_reminder`；契约常量同步。测：注册/卸载单测。
- [x] 5. **配置产物**（config）：`just gen-config-schema`；`configs/example.yaml` 与 schema 用新键；旧键拒绝并给出清晰错误。测：config schema 漂移单测。
- [x] 6. **BDD**（tests）：`tests/bdd/bindings_loop_middleware.rs` / `steps_loop_middleware.rs` 场景名与步骤文本同步；不改变行为断言。测：`cargo test --all-features --test bdd`。
- [x] 7. **文档**（docs）：`src/AGENTS.md` 用 workflow/todo 术语，保留 `plan` 给计划文档能力。测：`just qa-doc-check`。
- [x] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
