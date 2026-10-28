---
depends_on: []
title: Split workflow/todo vocabulary and retire the overloaded plan name
needs_specs_change: true
---

# Split workflow/todo vocabulary and retire the overloaded plan name

## Why

Current vocabulary overloads **plan** for three different things:

- `plan_state` is a todo-progress driver (flat checklist).
- `plan_execute` is a DAG execution engine (workflow-like).
- The user-facing idea of a "plan" (a plan document / plan mode) does not exist yet.

This causes confusion in config, specs, code, tests and UI. Before adding a real plan mode we must split the vocabulary so each term has one meaning.

## What Changes

- Rename the todo-progress middleware/config:
  - `plan_state` → `todo_progress`
  - `PlanStateMiddleware` → `TodoProgressMiddleware`
- Rename the DAG execution middleware/config:
  - `plan_execute` → `workflow`
  - `PlanExecuteMiddleware` → `WorkflowMiddleware`
- Rename the recovery checkpoint:
  - `plan_checkpoint` → `todo_checkpoint`
  - `PlanCheckpoint` / `PlanCheckpointStep` → `TodoCheckpoint` / `TodoCheckpointStep`
  - `CUSTOM_TYPE_PLAN_CHECKPOINT` → `CUSTOM_TYPE_TODO_CHECKPOINT`
- Rename the SDK DAG types:
  - `Plan` / `PlanStep` / `PlanStepStatus` → `Workflow` / `WorkflowStep` / `WorkflowStepStatus`
  - `CUSTOM_TYPE_PLAN` → `CUSTOM_TYPE_WORKFLOW`
- Rename the reminder:
  - `progress_reminder` → `todo_reminder`
- Rename spec rule titles and prose terms accordingly; move the DAG rule to a new `agent-workflow` capability.
- Update `configs/example.yaml`, config schema/docs and `src/AGENTS.md`.
- **One-time persistence migration**: read legacy `custom_type = "plan"` as workflow and `"plan_checkpoint"` as todo_checkpoint; write only the new names. Do **not** keep a permanent dual-parse alias.
- No behavior change in this change.

## Capabilities

- `agent-runtime`: rename middleware rule titles and terms.
- `agent-session-store`: rename custom types and document the one-time migration.
- `agent-workflow`: new capability for the DAG execution contract (moved from `agent-runtime`).
- `runtime-config`: config key rename and documentation.
- `src/AGENTS.md`: stable boundary wording.

## Impact

- Code: `plugins/xylitol-official-plugin-loop-example`, `crates/xylitol-plugin-sdk/src/session`, `crates/xylitol-plugin-sdk/src/loop_middleware`, tests, BDD bindings.
- Data: legacy sessions with `plan` / `plan_checkpoint` custom entries must still load once and be rewritten on next append.
- Config: old keys are rejected; pre-release breaking change documented in `configs/example.yaml`.
- No protocol version bump; no wire identifier version suffix.

## Out of Scope

- True plan mode, plan documents, mode switching, approval, TUI ModeStrip.
- Plugin extraction of todo/workflow/plan.
- Any behavior change to DAG validation, replan bounds or approval semantics.

## Further Notes

- The rename is deliberately atomic; no compatibility aliases for config keys or custom types beyond the one-time read migration.
- The new `Plan` name is reserved for the plan-document/plan-mode feature (`c2965`).
