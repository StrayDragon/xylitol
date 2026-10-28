# Design

## LoopStep

```rust
pub struct LoopStep<C> {
    pub ctx: C,
    pub control: LoopControl,
    pub properties: Vec<(String, String)>,
}

impl<C> LoopStep<C> {
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn with_properties(mut self, props: impl IntoIterator<Item = (String, String)>) -> Self;
}
```

`continued` / `stopped` initialize `properties` empty. Direct struct literals in tests must
add `properties: Vec::new()`.

## Span properties

`StageCall` gains `obs: &ObsSessionContext`. `call` starts the span with:

```text
loop.middleware.id / stage / order / scope
langfuse.observation.type = span
langfuse.session.id / xylitol.session.id (via langfuse_observation_properties_from)
xylitol.obs.lane = llm
```

On `Ok(step)`, core adds `step.properties` verbatim, then `control` / `changed` / `outcome`.
On fault, it adds the fault kind as today.

## Plugin property table

| Plugin / stage | Keys |
|---|---|
| plan `before_llm_request` | `loop.middleware.plan.mode`, `loop.middleware.plan.id` |
| plan `before_tool` denial | `loop.middleware.plan.denied_tool`, `loop.middleware.plan.read_only` |
| workflow `before_llm_request` | `loop.middleware.workflow.step`, `.depth`, `.replan`, `.blocked` |
| todo progress `before_llm_request` | `loop.middleware.todo.pending`, `.total` |
| todo reminder `before_llm_request` | `loop.middleware.todo.reminder_reason` |
| todo checkpoint `before_llm_request` | `loop.middleware.todo.checkpoint` |
| tool exposure `before_llm_request` | `loop.middleware.tools.before`, `.after` |
| self-heal `after_tool` | `loop.middleware.self_heal.signature`, `.attempt` |
| generator-critic `after_llm_response` | `loop.middleware.critic.verdict`, `.round` |
| loop-policies `decide_continue` | `loop.middleware.loop_policies.max_settles` |
| MCP truncate `after_tool` | `xylitol.mcp.truncated_bytes`, `xylitol.mcp.kept_bytes` |

## Tests

- Runtime unit test: a middleware returns `with_property` and the collected span carries it.
- Existing `loop.middleware` trace test asserts `langfuse.session.id` and `type=span`.
- Plugin unit tests assert the properties on the returned `LoopStep`.
- `infra-otel` session coverage list includes `loop.middleware`.
