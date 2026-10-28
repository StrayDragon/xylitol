---
depends_on:
  - c2964-refactor-split-workflow-todo-vocabulary
title: Add a reviewable plan mode built on plan documents and explicit approval
needs_specs_change: true
---

# Add a reviewable plan mode built on plan documents and explicit approval

## Why

Today the agent can start editing immediately. `todo_progress` drives a flat checklist and `workflow` drives a DAG, but neither gives the user a **reviewable plan artifact** before work begins. Classic coding agents separate planning from execution so the user can inspect and approve the intended change first.

`c2964` frees the word **plan** for this feature. This change introduces the missing planning phase.

## What Changes

- **Per-session plan mode** with explicit states `plan` / `execute`, stored as a `plan_mode` CustomEntry so it survives resume.
- **User toggle**: `/plan` slash command and a `set_plan_mode` unary RPC; toggle is immediate because the user already decided.
- **Agent request**: a `plan_mode` tool may request entering plan mode, but it **must** obtain user approval through the `ask` gateway; denial leaves the mode unchanged.
- **Plan documents**:
  - New `PlanDoc` domain model with stable `plan_id`, title, ordered steps (`id`, `content`, optional `depends_on` / `verification`), status and timestamps.
  - Multiple plans per session; all plan tools take `plan_id` (or default to the active plan).
  - `CustomEntry(custom_type = "plan_doc")` is the SSOT; a human-readable `plans/<plan_id>.md` view is materialized under the session directory (default session temp scope).
  - Tools: `plan_write`, `plan_read`, `plan_list`, `plan_submit`.
- **End-of-plan approval**: `plan_submit` marks the document `awaiting_approval` and asks the user through the ask gateway. Approval sets `approved`, switches mode to `execute`, and seeds the todo list from the plan steps. Denial keeps the document editable and stays in plan mode.
- **Prompt injection only (MVP)**: in plan mode the runtime injects a planning contract that asks the model to research, read and produce a plan document before editing. Built-in tools are **not** restricted in this change; read-only enforcement is deferred to `c2969`.
- **Mode event**: `planModeChanged` wire event so clients can render mode state (`c2966` consumes it).

## Capabilities

- `agent-plan` (new): plan mode state machine, plan documents, approval and todo handoff.
- `agent-session-store`: `plan_doc` / `plan_mode` custom entries and `plans/<plan_id>.md` view.
- `protocol-app` / `server-core`: `set_plan_mode` unary and `planModeChanged` event.
- `app-tui-commands`: `/plan` slash command.
- `agent-runtime`: plan-mode prompt injection middleware.

## Impact

- SDK: `session/plan_doc.rs`, `session/plan_mode.rs`, `ports/plan.rs`.
- Tools plugin: plan tools + `SessionPlanDocGateway`.
- Loop example plugin: `PlanModeMiddleware`.
- Kernel: tool wiring with plan gateway and optional ask gateway.
- Runtime driver: `set_plan_mode` writes the mode entry and publishes the event.
- Serve: route for `set_plan_mode`.
- Wire: event mapping.
- TUI: slash parsing and RPC call (rendering arrives in `c2966`).

## Out of Scope

- Read-only tool enforcement (`c2969`).
- Project/global plan storage (`c2970`).
- Nested workflow (`c2971`).
- Replacing the `<plan>` workflow block with a tool (`c2972`).
- Extracting plan/todo/workflow into standalone plugins (`c2968`).

## Further Notes

- MVP deliberately keeps plan mode advisory. Prompt injection alone is enough to validate the workflow; hard enforcement needs its own approval semantics.
- Plan document and mode state are append-only CustomEntries, matching existing session storage conventions.
- Prior art: Claude Code / Cursor use read-only planning modes; Aider uses a two-stage architect→edit flow; Codex uses a read-only sandbox plus a plan template. This change takes only the reviewable-plan and explicit-approval parts.
