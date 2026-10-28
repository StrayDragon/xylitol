---
depends_on: []
title: Rename loop vocabulary to LLM request/response and loop policies
needs_specs_change: true
branch: sdd/c2976-refactor-loop-vocabulary
base_branch: main
base_sha: 23f39a9beab20a4b3a073b84e19ce36d22f40e54
---

# Rename loop vocabulary to LLM request/response and loop policies

## Why

- `before_model` / `after_model` bracket **each LLM generation request/response**, not a turn and not model selection. The current names are ambiguous in specs, AGENTS and plugin author docs.
- After `c2968`, `xylitol-official-plugin-loop-example` is no longer an example: it is a full loop policy pack (`deny_bash_patterns`, `max_settles`, `self_heal`, `generator_critic`, `tool_exposure`). Keeping `example` in the crate, config key and plugin id misleads plugin authors and users.
- The plugin author guide should point at this pack as the reference implementation, so the name should describe what it does.

## What Changes

- Rename loop stages and ctx types:
  - `before_model` → `before_llm_request`
  - `after_model` → `after_llm_response`
  - `BeforeModelCtx` → `BeforeLlmRequestCtx`
  - `AfterModelCtx` → `AfterLlmResponseCtx`
  - `LoopStage::{BeforeModel,AfterModel}` → `LoopStage::{BeforeLlmRequest,AfterLlmResponse}`
- Rename the official plugin:
  - crate `xylitol-official-plugin-loop-example` → `xylitol-official-plugin-loop-policies`
  - config section `loop_example` → `loop_policies`
  - plugin id `loop-example` → `loop-policies`
  - types `LoopExampleConfig` / `LoopExamplePlugin` → `LoopPoliciesConfig` / `LoopPoliciesPlugin`
- Reject the legacy `loop_example` config section with a migration hint to `loop_policies` (and to the top-level `todo` / `workflow` / `plan` sections for the keys moved by `c2968`).
- Add `crates/xylitol-plugin-sdk/README.md` as the plugin author guide; name `plugins/xylitol-official-plugin-loop-policies` as the reference implementation.
- Update specs, AGENTS, config schema/example, tests and BDD step names.

## Capabilities

- `agent-runtime`: loop stage vocabulary and plugin reference rule wording.
- `runtime-config`: `loop_policies` config section and legacy `loop_example` rejection.
- `workspace-assembly`: crate/plugin naming and dependency table.

## Impact

- SDK: `loop_middleware.rs` stage/ctx/chain names.
- Runtime: `middleware.rs` runners/fingerprints and ReAct call sites.
- Kernel: builtins / boot / bootstrap / in-process driver.
- Plugins: `loop-policies` rename; plan/todo/workflow middleware stage names.
- Config: schema/example/generator/loader.
- Tests: BDD step names and bindings.
- Docs: `crates/xylitol-plugin-sdk/README.md`, `AGENTS.md`, `src/AGENTS.md`.

## Out of Scope

- Any behavior change to loop control, plan mode, workflow, todo or compaction.
- Observability enrichment (`c2977`): `LoopStep` semantic properties and `loop.middleware` session attributes.
- Compatibility aliases for the old stage names or the old config key. Pre-release breaking rename; old config key is rejected with a hint.

## Further Notes

- The rename is mechanical and should keep `just qa` green with unchanged behavior.
- `loop_policies` is the plugin config section; the plugin id stays `loop-policies`.
