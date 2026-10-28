---
depends_on:
  - c2964-refactor-split-workflow-todo-vocabulary
  - c2965-add-plan-mode-with-plan-doc-and-approval
title: Extract plan, workflow and todo capabilities into official plugins
needs_specs_change: true
---

# Extract plan, workflow and todo capabilities into official plugins

## Why

Plan, workflow and todo are optional capabilities, but today they live inside the loop example and the built-in tools plugin. Users who do not need DAG execution or plan mode still carry the config, middleware and tool surface. Extracting them keeps the core loop small and makes each capability independently enableable.

## What Changes

- New official plugins:
  - `xylitol-official-plugin-todo`: `todo_progress`, `todo_reminder`, `todo_checkpoint`, `todo_rewrite` / `todo_update`, session todo gateway.
  - `xylitol-official-plugin-workflow`: `workflow` middleware and workflow tools.
  - `xylitol-official-plugin-plan`: plan-mode middleware, plan tools, plan document gateway.
- `xylitol-official-plugin-loop-example` keeps only the example middleware plus self-heal, generator-critic and tool-exposure.
- `xylitol-official-plugin-tools` keeps core tools (`read` / `write` / `edit` / `bash` / `grep` / `find` / `ls`) and `ask`.
- Config moves from `loop_example.*` to top-level `todo:` / `workflow:` / `plan:` sections. Defaults: `todo` enabled, `workflow` and `plan` disabled.
- Kernel boot registers plugins from config. Cross-plugin collaboration uses SDK services/events (`AgentTodoGateway`, `PlanDocGateway`, `AskUserGateway`), never crate-to-crate `use`.
- No plugin marketplace, registry or dylib loading.

## Capabilities

- `workspace-assembly`: new crates and dependency direction.
- `runtime-config`: `todo` / `workflow` / `plan` config sections.
- `agent-todo`, `agent-workflow`, `agent-plan`: ownership moves, behavior unchanged.
- `src/AGENTS.md`: crate table and plugin boundary wording.

## Impact

- New plugin crates and kernel boot wiring.
- Tool registration moves from `tools-builtin` to the owning plugin.
- Config schema and example rewrite; pre-release breaking change, no alias keys.
- BDD registration scenarios move to the owning plugin.
- `loop-example` shrinks to an actual example.

## Out of Scope

- New plan/workflow behavior (owned by `c2965` / `c2972` / `c2971`).
- Third-party plugin installation or discovery.
- Per-plugin remote configuration.

## Further Notes

- The todo capability stays default-on because it is part of the baseline product UX; plan and workflow stay opt-in.
- Extraction is mechanical: behavior and specs stay the same except config location and ownership.
