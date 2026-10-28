---
depends_on:
  - c2964-refactor-split-workflow-todo-vocabulary
  - c2968-refactor-extract-plan-workflow-todo-plugins
title: Replace the plan control block with a workflow_write tool
needs_specs_change: true
branch: sdd/c2972-add-workflow-write-tool
base_branch: main
base_sha: 59487865f5acc3902c1f280ec2b249efb9142ba7
---

# Replace the plan control block with a workflow_write tool

## Why

The workflow middleware currently asks the model to emit a raw `<plan>{json}</plan>` block inside assistant text. That block is fragile: it is prose-coupled, hard to validate at the call boundary, and pollutes provider POST content. A typed tool gives schema validation, a structured result, and an observable event.

## What Changes

- Add `workflow_write` tool:
  - Args: the full workflow (`goal`, `steps[]`, optional `resume_cursor`).
  - Validates DAG and replan constraints against the latest persisted workflow.
  - Persists `CustomEntry(workflow)` and publishes `WorkflowUpdated`.
  - Returns a compact JSON summary (`goal`, step count, current runnable step).
- Add `workflow_read` tool returning the latest workflow as JSON.
- Middleware stops parsing `<plan>` from assistant text. It observes `workflow_write` in `after_tool`:
  - Success → adopt the tool's workflow state.
  - Error → inject the validation error as `<workflow_error>` before the next model call.
- Remove `extract_plan_json` and the `<plan>` contract text; the model is told to call `workflow_write`.
- Provider POST no longer carries raw `<plan>` blocks.
- Tool is registered only by the workflow plugin; disabled workflow means no tool and no middleware.

## Capabilities

- `agent-workflow`: write/read tools, validation at the tool boundary, error feedback.
- `agent-session-store`: workflow entry written by the tool; same format.
- `infra-provider` / `agent-llm-projection`: no raw control block in POST.

## Impact

- Workflow plugin: new tools + middleware simplification.
- SDK: shared validation used by both tool and middleware.
- Config: no new keys; tool follows `workflow.enabled`.
- Tests: tool validation, middleware adoption, error feedback, no raw block in POST.
- Docs: workflow write entry is a tool, not an assistant block.

## Out of Scope

- Changing workflow semantics or replan rules.
- Nested workflow (`c2971`) beyond using the same validation.
- Migrating old assistant `<plan>` text (only persisted CustomEntries matter).

## Further Notes

- Old sessions already store workflows as CustomEntries; no text migration is needed.
- Removing the raw block also removes a class of prompt-injection surface from assistant content.
