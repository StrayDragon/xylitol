# Design

## Gate

`PlanModeMiddleware` order 185 gains `before_tool`:

```text
if latest_plan_mode == Plan && config.read_only:
    if tool_name not in allowlist:
        ctx.args = { "blocked_by": "plan-mode", "reason": "plan mode is read-only" }
        return LoopStep::stopped(ctx)
return LoopStep::continued(ctx)
```

- `stopped` prevents execution; the rewritten args carry the stable reason so the tool result is self-explanatory.
- The gate runs before other `before_tool` middleware by order; later middleware may still observe the call, but the tool never executes.
- If mode cannot be read (store/session unavailable), the gate **fails closed** in plan mode only when a cached mode says `Plan`; otherwise it does nothing. The middleware caches the last known mode from `before_model`.

## Allowlist

Default:

```text
read grep find ls ask
plan_write plan_read plan_list plan_submit plan_mode
todo_rewrite todo_update
```

- `plan.*` tools must stay available or the agent cannot produce/approve a plan.
- Todo tools stay available so an approved plan can be handed off.
- `ask` stays available for clarifying questions and approval.
- Config `plan.allowed_tools` appends names; duplicates are ignored.
- Unknown tools are denied by default. This is intentional: plan mode must not accidentally allow a newly added mutating tool.

## Mode transitions

- `Plan` → gate active.
- `Execute` → gate inactive; normal tool availability restored.
- Approval switches to `Execute` in the same tool call, so the next turn is unrestricted.
- User `/plan off` also switches immediately.

## Interaction with core permission

- The gate only returns `stopped`; it never rewrites a denied call into an allowed one.
- Core permission checks still run for allowed tools.
- If another `before_tool` middleware returns `stopped` first, the plan gate does not run; the final decision remains the first stop.

## Specs

- `agent-plan`: new `plan-mode-read-only` rule with allowlist and denial semantics.
- `agent-tools`: plan-mode gate must not execute denied tools and must not bypass core permission.
- `agent-runtime`: ordering rule for multiple `before_tool` gates.

## Tests

- Unit: write/edit/bash denied in plan mode; read/ask/plan/todo allowed; execute restores.
- Unit: unknown tool denied; allowlist extension works.
- Unit: denial reason is stable and no side effect is recorded.
- BDD: plan mode rejects an edit attempt; approval then allows it.
- `just qa` and `llman-sdd validate --all --strict`.
