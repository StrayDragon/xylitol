---
depends_on: []
title: Preserve plan and workflow state across compaction and session resume
needs_specs_change: true
branch: sdd/c2975-fix-preserve-state-across-compaction
base_branch: main
base_sha: 811a0d599bec7ed6dd4796e712aca0a0feb298df
---

# Preserve plan and workflow state across compaction and session resume

## Why

Manual tmux verification found two state-continuity bugs:

- Compaction cuts the leaf branch at `firstKeptEntryId`. State CustomEntries written before the cut (`plan_mode`, `plan_doc`, `plan_ref`, `workflow`, `todo_checkpoint`) disappear from `load_leaf_branch`, so the next turn sees `execute` mode, loses the active workflow, or loses plan documents. `agent_todo` already has a post-compact re-append; the other state entries do not.
- The TUI ModeStrip is only updated by `PlanModeChanged` events. Resuming a session that is already in plan mode leaves the strip empty until the user toggles `/plan` again.

## What Changes

- After compaction, re-append the latest state CustomEntries that the cut would drop:
  - `plan_mode` (latest),
  - `plan_doc` (latest per `plan_id`),
  - `plan_ref` (latest),
  - `workflow` (latest),
  - `todo_checkpoint` (latest).
- Keep the existing `agent_todo` ensure path.
- When the host switches or creates a session, emit `PlanModeChanged` from the new session's latest `plan_mode` entry so the TUI ModeStrip reflects the restored state immediately.
- Add a BDD scenario for plan-mode survival across compaction and a host unit test for the session-switch event payload.

## Capabilities

- `domain-compaction`: state entries survive compaction.
- `app-tui`: ModeStrip reflects the active session's plan mode after resume.

## Impact

- Compaction post-processing in `xylitol-official-plugin-compaction`.
- Host session switch/new-session event emission in `xylitol-serve`.
- Specs: `domain-compaction`, `app-tui`.

## Out of Scope

- Recovering state entries from sessions compacted before this fix.
- Changing the compaction cut algorithm or `firstKeptEntryId`.
- Adding a new state-snapshot entry type.

## Further Notes

- The fix follows the existing `ensure_agent_todo_after_compact` pattern: snapshot before compact, reload the cut branch, append only what is missing.
- The ModeStrip fix uses the existing `PlanModeChanged` event path, so no new wire event is needed.
