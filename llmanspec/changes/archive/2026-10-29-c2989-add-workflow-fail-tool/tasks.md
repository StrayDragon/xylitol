# Tasks

- [x] 1. **specs**（agent-workflow）：`r2966` 增加 `workflow_fail` 工具契约（step_id 可选、reason 必填、completed 拒绝、成功 append + `WorkflowUpdated`），并补一条可执行场景。测：`llman-sdd validate c2989-add-workflow-fail-tool`。
- [x] 2. **workflow plugin**（tools）：实现 `WorkflowFailTool`（args/schema/execute/注册），复用 `validate_workflow_replan` 与 `WorkflowGateway::persist`。测：`cargo test -p xylitol-official-plugin-workflow workflow_fail`。
- [x] 3. **workflow plugin**（middleware）：`after_tool` 对 `workflow_fail` 复用 `workflow_write` 的 reload / pending error 路径。测：`cargo test -p xylitol-official-plugin-workflow`。
- [x] 4. **tests**（repo）：补工具单测（默认 in_progress、显式 step_id、无 workflow、completed、无 in_progress）与 BDD 场景步骤/绑定。测：`cargo test --all-features --test bdd`。
- [x] 5. **门禁**（repo）：`cargo test -p xylitol-official-plugin-workflow --all-features`、`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
