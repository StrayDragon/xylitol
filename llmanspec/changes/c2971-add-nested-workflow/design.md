# Design

## Model

```text
WorkflowStep {
  id, description, depends_on, inputs, outputs, risk,
  verification, rollback, requires_human, status, output_ref,
  sub_workflow: Option<Box<Workflow>>
}
```

- A step with `sub_workflow` is a **container**: its own `description` summarizes the child goal; the child carries the executable steps.
- A container step must not also carry `verification`/`output_ref` semantics that conflict with the child; those fields are ignored while the child is active and restored on completion.
- Ids are unique within a level. Cross-level references are not allowed.

## Validation

- Recursive: each level checks unique ids, known dependencies, no self-dependency, no cycle.
- `max_depth` counts nesting levels from the root (root = 1).
- `max_total_steps` counts every step at every level.
- Replan validation recurses; completed steps at any level are locked.

## Execution

State machine:

1. Find a runnable step. If it has `sub_workflow`, mark the container `in_progress` and push the child as the active workflow.
2. Run the child with the same `decide_continue` rules.
3. Child `all_completed` → mark container `completed`, pop to parent, continue.
4. Child has a failed step → replan the child first; if the child cannot recover within `max_replans`, mark the container `failed` and replan the parent.
5. Blocked child → bubble the block to the parent; approval applies to the child step.
6. All levels complete → inject the completion summary once.

## Resume

- `resume_cursor` stores a path: `["s2", "s2.1"]`.
- Loading resolves the path; a missing/invalid path falls back to the first unfinished step at the root and records a warning.
- Completed containers stay completed; their child state is not re-executed.

## Bounds

- `max_depth` default 2; exceeding it is a validation error, not a runtime surprise.
- `max_total_steps` default 100; validation rejects oversized workflows.
- Replan count stays per active workflow; a parent replan does not reset child counters.

## Persistence

- One `CustomEntry(workflow)` per update, including nested children.
- `updated_at` changes on every accepted write.
- No separate child entries, so fork/travel sees one atomic workflow artifact.

## Specs

- `agent-workflow`: nested model, validation bounds, execution expansion, replan/resume.
- `agent-session-store`: nested workflow entry and path cursor.

## Tests

- Unit: recursive validation (depth, total steps, cycle, duplicate id).
- Unit: container completion, child failure → parent replan, blocked propagation.
- Unit: resume path resolution and fallback.
- BDD: nested workflow completes and marks the parent done.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：`agent-workflow` 增嵌套模型、递归校验、执行展开、replan/resume 与边界；`agent-session-store` 增嵌套条目与路径 cursor。测：`llman-sdd validate --all --strict`。
- 2. **SDK 模型**（sdk）：`WorkflowStep.sub_workflow`；递归 `validate_workflow`；路径 helper 与 `max_depth` / `max_total_steps`。测：深度/总数/环/重复 id。
- 3. **中间件展开**（plugin）：容器步骤进入子 workflow；子完成标记父完成；子失败向上 replan；blocked 上浮。测：完成/失败/阻塞传播。
- 4. **恢复**（plugin/sdk）：`resume_cursor` 路径解析与回退；已完成容器不重跑。测：路径恢复与缺失回退。
- 5. **配置**（config）：`workflow.max_depth` / `max_total_steps`；schema/example 同步。测：schema 漂移。
- 6. **BDD**（tests）：嵌套 workflow 完成后父步骤为 completed。测：`cargo test --all-features --test bdd`。
- 7. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
