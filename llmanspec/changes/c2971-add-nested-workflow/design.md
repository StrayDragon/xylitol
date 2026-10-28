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
