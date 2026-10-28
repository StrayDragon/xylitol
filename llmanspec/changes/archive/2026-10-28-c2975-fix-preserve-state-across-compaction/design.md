# Design

## Compaction

`compact_session` already snapshots `agent_todo` before compaction and re-appends it if the cut drops it. Extend the same post-processing to the remaining state entries:

```text
ensure_state_entries_after_compact(store, session_id, entries_before):
    cut = build_context_entries(load_leaf_branch(session_id))
    for each state kind:
        if cut has no entry of that kind:
            append latest snapshot from entries_before
    for plan_doc:
        append latest doc per plan_id when the cut has no entry for that id
```

State kinds and snapshots:

| custom_type | snapshot |
|---|---|
| `plan_mode` | latest `PlanMode` |
| `plan_doc` | latest `PlanDoc` per `plan_id` |
| `plan_ref` | latest `PlanRef` |
| `workflow` | latest `Workflow` |
| `todo_checkpoint` | latest `TodoCheckpoint` |

Order: append after `ensure_agent_todo_after_compact` so all state is present before the next turn.

## Session switch

After `switch_session` / `new_session` succeeds on the writer driver, read the new session's latest `plan_mode` from the store and emit `PlanModeChanged`. Missing entry means `execute`, which clears the strip. This reuses the existing event path used by `set_plan_mode`.

## Tests

- BDD: seed a session in plan mode, compact it, assert the reloaded leaf branch still reports `plan`.
- Serve unit test: build the event payload from a session with a `plan_mode` entry and assert `PlanModeChanged { mode: Plan }`; missing entry yields `execute`.
