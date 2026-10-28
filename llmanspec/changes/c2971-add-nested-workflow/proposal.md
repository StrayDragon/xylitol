---
depends_on:
  - c2965-add-plan-mode-with-plan-doc-and-approval
  - c2968-refactor-extract-plan-workflow-todo-plugins
title: Support nested workflows inside workflow steps
needs_specs_change: true
---

# Support nested workflows inside workflow steps

## Why

The current workflow is a single flat DAG. Real tasks often contain a sub-task that needs its own dependency graph (for example "implement the feature" contains "write tests" → "run tests" → "fix failures"). Flattening everything into one graph makes replanning coarse and the plan hard to read.

## What Changes

- `WorkflowStep` gains an optional `sub_workflow: Option<Workflow>`.
- Execution expands a runnable parent step into its child workflow:
  - Parent becomes `in_progress` when the child starts.
  - Child completion marks the parent `completed`.
  - Child failure marks the parent `failed` and triggers replan from the child upward.
- Validation is recursive: unique ids per level, dependencies exist, no cycles at any level, depth and total-step bounds.
- Bounds: `max_depth` default 2, `max_total_steps` default 100.
- Resume cursor becomes a step path (`["s2", "s2.1"]`) so a nested run can resume at the right level.
- Approval/blocked steps propagate upward: a blocked child blocks the parent chain.
- Persistence stays in one `CustomEntry(workflow)`; no separate entries for child workflows.
- Replan constraints apply recursively: completed steps at any level stay locked.

## Capabilities

- `agent-workflow`: nested model, execution expansion, bounds, replan/resume.
- `agent-session-store`: workflow entry carries nested steps and path cursor.

## Impact

- SDK: `WorkflowStep.sub_workflow`, recursive validation, path helpers.
- Workflow middleware: expansion state machine, failure propagation, resume.
- Config: `workflow.max_depth`, `workflow.max_total_steps`.
- Tests: validation, execution, replan, resume, bounds.
- Docs: workflow hierarchy `workflow → N plans → N todos` clarified.

## Out of Scope

- Cross-workflow references by id (child is embedded, not shared).
- Parallel execution of sibling sub-workflows.
- Plan-document nesting (plans stay flat lists in `c2965`).
- Recursive UI rendering of nested workflows.

## Further Notes

- Embedding the child workflow keeps persistence atomic and avoids orphan references.
- Depth 2 covers the common "task → sub-task" case; deeper nesting needs an explicit config change.
