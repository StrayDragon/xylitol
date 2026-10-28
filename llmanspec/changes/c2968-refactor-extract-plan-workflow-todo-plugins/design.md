# Design

## Crates

| Crate | Owns | Depends on |
|---|---|---|
| `xylitol-official-plugin-todo` | todo middleware + tools + `SessionAgentTodoGateway` | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-workflow` | workflow middleware + workflow tools | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-plan` | plan-mode middleware + plan tools + `SessionPlanDocGateway` | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-loop-example` | example middleware, self-heal, generator-critic, tool-exposure | `xylitol-plugin-sdk` |
| `xylitol-official-plugin-tools` | core tools + `ask` | `xylitol-plugin-sdk` |

Rules:

- Plugins only `use xylitol_plugin_sdk`. No plugin-to-plugin crate dependency.
- Plan needs todo handoff: it resolves `AgentTodoGateway` from `Ctx` at apply time; if absent, `plan_submit` reports that todo handoff is unavailable.
- Plan needs approval: it resolves `AskUserGateway` from `Ctx`; if absent, `plan_submit` reports approval unavailable.
- Kernel supplies those services before plugin `apply`.

## Config

```yaml
todo:
  enabled: true
  progress: false
  reminder: false
  reminder_stale_after: 3
  checkpoint: false
workflow:
  enabled: false
  max_replans: 3
plan:
  enabled: false
  read_only: true
```

- Old `loop_example.plan_state` / `plan_execute` / `plan_checkpoint` / `progress_reminder` keys are rejected with a migration hint.
- `loop_example` keeps `enabled`, `max_settles`, `deny_bash_patterns`, `self_heal`, `generator_critic`, `tool_exposure`.

## Boot

1. Kernel boots core plugins and built-in tools.
2. Kernel registers services: `RuntimeSessionStore`, `AgentTodoGateway`, `AskUserGateway` (when available).
3. `todo` plugin applies (default-on), registering middleware and tools.
4. `workflow` / `plan` plugins apply only when enabled.
5. Each plugin returns an unregister effect so unload leaves no middleware or tools behind.

Tool names stay stable (`todo_rewrite`, `todo_update`, `workflow_*`, `plan_*`); only ownership changes.

## Specs

- `runtime-config`: replace `loop_example` sub-keys with `todo` / `workflow` / `plan` sections.
- `workspace-assembly`: new crate members and dependency direction.
- `agent-todo` / `agent-workflow` / `agent-plan`: ownership wording only; behavior unchanged.
- `src/AGENTS.md`: crate table, plugin boundary and default-on/off rules.

## Tests

- Plugin registration/unregistration per crate; disabled plugin registers nothing.
- Config schema/example drift test; old keys rejected.
- Tool ownership test: core tools only in `tools-builtin`; todo/workflow/plan tools only in their plugin.
- Service absence: plan plugin without ask/todo degrades with a structured error.
- BDD: default todo enabled, plan/workflow disabled.
- `just qa` and `llman-sdd validate --all --strict`.
