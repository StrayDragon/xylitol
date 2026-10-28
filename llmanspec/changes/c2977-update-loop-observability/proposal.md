---
depends_on: []
title: Enrich loop middleware spans with session and semantic properties
needs_specs_change: true
---

# Enrich loop middleware spans with session and semantic properties

## Why

- `loop.middleware` spans only carry `loop.middleware.*`; they miss `langfuse.session.id`,
  `langfuse.observation.type` and `xylitol.obs.lane`, so Langfuse cannot group or filter
  them consistently with `agent.turn` / `llm.request` / `tool.execute`.
- New feature plugins (plan, workflow, todo, loop-policies, MCP truncation) are visible
  only as a generic middleware id and a coarse `changed` fingerprint. Operators cannot
  tell plan mode / plan id / workflow step / todo counts / critic verdict / truncation
  bytes from the trace.
- The user asked for the standard observability scope: session attributes plus
  `LoopStep` semantic properties. Trace metadata/tags and custom metrics are out of scope.

## What Changes

- Add `LoopStep.properties: Vec<(String, String)>` with `with_property` / `with_properties`
  helpers. Default empty; existing constructors keep working.
- Pass `ObsSessionContext` into the middleware runner and add
  `langfuse_observation_properties_from("span", obs)` to every `loop.middleware` span.
- Core writes `LoopStep.properties` verbatim to the span. Convention:
  `loop.middleware.<feature>.<field>`.
- Official plugins attach semantic properties:
  - plan: mode, plan id, denied tool / read-only;
  - workflow: step, depth, replan count, blocked;
  - todo: pending/total, reminder reason, checkpoint state;
  - loop-policies: tool exposure before/after, self-heal signature/attempt, critic verdict/round;
  - MCP truncation: truncated/kept bytes.
- Specs: `agent-runtime` r2924 adds the optional semantic properties; `infra-otel`
  session coverage list includes `loop.middleware`.

## Capabilities

- `agent-runtime`: `loop.middleware` span session attributes and semantic properties.
- `infra-otel`: low-frequency span coverage includes `loop.middleware`.

## Impact

- SDK: `LoopStep` shape and helpers.
- Runtime: `run_stage` / `call` signature and span properties.
- Plugins: plan / workflow / todo / loop-policies / MCP.
- Tests: span property unit test; plugin unit tests.
- Specs: `agent-runtime`, `infra-otel`.
- Docs: `crates/xylitol-plugin-sdk/README.md` property convention.

## Out of Scope

- Langfuse trace metadata/tags or custom metrics/scores.
- New spans for session slot eviction, tape, pagination or transport.
- Changing existing span names or the `loop.middleware` control contract.

## Further Notes

- Properties are optional and cheap: no allocation when empty; spans are only created when
  `provider_trace_active()`.
- Keys are written verbatim so plugins can use `xylitol.*` when a property is not a
  middleware decision (e.g. `xylitol.mcp.truncated_bytes`).
