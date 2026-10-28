# Design

## Rename map

| Current | New | Notes |
|---|---|---|
| `plan_state` | `todo_progress` | config key + middleware id |
| `PlanStateMiddleware` | `TodoProgressMiddleware` | order 200 unchanged |
| `PLAN_CONTRACT` | `TODO_PROGRESS_CONTRACT` | system fragment text unchanged |
| `plan_execute` | `workflow` | config key + middleware id |
| `PlanExecuteMiddleware` | `WorkflowMiddleware` | order 190 unchanged |
| `Plan` | `Workflow` | SDK type |
| `PlanStep` | `WorkflowStep` | SDK type |
| `PlanStepStatus` | `WorkflowStepStatus` | SDK type |
| `CUSTOM_TYPE_PLAN = "plan"` | `CUSTOM_TYPE_WORKFLOW = "workflow"` | persisted key |
| `latest_plan` | `latest_workflow` | helper |
| `validate_plan` | `validate_workflow` | helper |
| `validate_replan` | `validate_workflow_replan` | helper |
| `runnable_step` | `runnable_workflow_step` | helper |
| `downstream_ids` | `workflow_downstream_ids` | helper |
| `plan_checkpoint` | `todo_checkpoint` | config key + middleware id |
| `PlanCheckpoint` | `TodoCheckpoint` | SDK type |
| `PlanCheckpointStep` | `TodoCheckpointStep` | SDK type |
| `CUSTOM_TYPE_PLAN_CHECKPOINT = "plan_checkpoint"` | `CUSTOM_TYPE_TODO_CHECKPOINT = "todo_checkpoint"` | persisted key |
| `latest_plan_checkpoint` | `latest_todo_checkpoint` | helper |
| `progress_reminder` | `todo_reminder` | config key + middleware id |
| `ProgressReminderMiddleware` | `TodoReminderMiddleware` | order 150 unchanged |

The `<plan>{json}</plan>` control block is **not** renamed in this change; it remains the workflow write entry until `c2972` replaces it with a tool. The term "workflow" is the execution engine; "plan" is reserved for the new plan-document feature.

## Persistence migration

`SessionManager::load_entries` (or the SDK entry deserializer) must accept legacy custom types:

```text
"plan"            -> Workflow
"plan_checkpoint" -> TodoCheckpoint
```

- Read compatibility is a **one-time migration path**, not a permanent alias.
- New appends always write `"workflow"` / `"todo_checkpoint"`.
- The migration must not rewrite history eagerly; it only maps on read and writes the new key on the next append. This keeps old sessions readable without a bulk rewrite.
- Add unit tests for: old `plan` loads as workflow; old `plan_checkpoint` loads as todo_checkpoint; new appends use new keys.

## Specs

- `agent-runtime`: rename `plan-state` / `plan-execute` / `plan-checkpoint` / `progress-reminder` rule prose and scenario names; keep req IDs where the rule is semantically unchanged.
- `agent-session-store`: rename `custom_type` values and add the migration note.
- New `agent-workflow`: move the DAG rule (`r2946`) and its scenarios from `agent-runtime`.
- `runtime-config`: update the `loop_example` key documentation; no new keys.
- `src/AGENTS.md`: replace "规划执行" wording with workflow/todo terms; reserve "plan" for the plan-document feature.

## Config

`configs/example.yaml` and generated schema:

```yaml
loop_example:
  todo_progress: false
  todo_reminder: false
  todo_reminder_stale_after: 3
  todo_checkpoint: false
  workflow:
    enabled: false
    max_replans: 3
```

Old keys are rejected with a clear error. No alias keys.

## Tests

- Unit: rename helpers, migration read/write.
- BDD: update scenario names and step bindings; keep behavior assertions identical.
- Config: schema/example validation.
- `just qa` and `llman-sdd validate --all --strict`.

## Risks

- Large mechanical diff; mitigate with `rg` sweeps and compiler-driven fixes.
- Legacy sessions must remain readable; migration tests are mandatory.
- Config break is intentional and documented; no fallback.
