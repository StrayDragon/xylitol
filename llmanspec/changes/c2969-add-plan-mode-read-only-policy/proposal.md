---
depends_on:
  - c2965-add-plan-mode-with-plan-doc-and-approval
title: Enforce a read-only tool policy while plan mode is active
needs_specs_change: true
---

# Enforce a read-only tool policy while plan mode is active

## Why

`c2965` makes plan mode advisory: the prompt asks the model to plan first, but the model can still call `write`, `edit` or `bash` and change the workspace before approval. Classic plan modes are read-only. Users need the guarantee that planning cannot mutate files.

## What Changes

- Plan mode gains a **tightening-only** `before_tool` gate:
  - Allowed by default: `read`, `grep`, `find`, `ls`, `ask`, `plan_write`, `plan_read`, `plan_list`, `plan_submit`, `plan_mode`, `todo_rewrite`, `todo_update`.
  - Denied by default: `write`, `edit`, `bash` and any tool not on the allowlist.
  - Denial happens before execution, returns a stable reason (`plan mode is read-only`), and never executes the tool.
- Config: `plan.read_only` default `true`; optional `plan.allowed_tools` extends the allowlist for research tools (for example a read-only MCP tool).
- Core permission remains the final authority: the gate can only deny, never allow something core would deny.
- Leaving plan mode (approval or user toggle) restores normal tool availability immediately.
- No tool-schema filtering in this change; hidden tools are `c2968`-era exposure work if needed.

## Capabilities

- `agent-plan`: read-only guarantee, denial reason, allowlist.
- `agent-tools`: plan-mode gate ordering and non-execution semantics.
- `agent-runtime`: middleware ordering with other `before_tool` gates.

## Impact

- `PlanModeMiddleware` gains `before_tool`.
- `PlanPluginConfig` / `LoopExampleConfig` gains `read_only` and `allowed_tools` until `c2968` moves it.
- Tests: denied write/edit/bash; allowed read/ask/plan/todo; execute mode restores; other middleware still runs after a denial.
- Docs: plan-mode read-only contract in specs and AGENTS.

## Out of Scope

- Read-only bash classification (MVP denies `bash` entirely).
- Filesystem sandboxing or OS-level enforcement.
- Hiding denied tools from the model schema.
- Changing core permission semantics.

## Further Notes

- Denying `bash` entirely is deliberately conservative: a read-only bash classifier is easy to bypass and needs its own design.
- The allowlist is additive only; it cannot remove tools that core already denies.
