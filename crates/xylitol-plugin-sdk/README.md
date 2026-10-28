# xylitol plugin SDK

Official plugins and third-party plugins depend on this crate only. It carries the
plugin vocabulary (events, tools, sessions, models), the ports, the error types, and
the `rt` re-export used to register a plugin with the rutis lifecycle.

## Loop middleware

`LoopMiddleware` lets a plugin participate in ReAct decisions without replacing the
loop. Core keeps turn boundaries, cancellation, permission final say, and message
persistence; a plugin only returns `{ ctx, control }`.

### Stages

| Stage | Granularity | Purpose |
|---|---|---|
| `before_turn` | turn | system prompt / fragments |
| `before_llm_request` | each LLM request | messages, tool schemas, generation options |
| `after_llm_response` | each LLM response | assistant parts / tool calls |
| `before_tool` | each tool call | args / tool routing |
| `after_tool` | each tool result | structured parts / details / error flag |
| `decide_continue` | settle | continue / stop / compact / handoff |
| `after_turn` | turn | observe only; illegal control is ignored |

`before_llm_request` runs after skills expansion and before `project_outbound`.
`after_llm_response` runs after the stream ends and before persistence.

### Control

`LoopControl` is a closed set: `Continue`, `Stop`, `Compact`, `Handoff`.
`Stop` ends the current run. `Compact` asks core to compact once and continue.
`Handoff` is reserved; core currently returns `LoopMiddlewareHandoffUnsupported`.

`LoopStep::continued(ctx)` and `LoopStep::stopped(ctx)` are the usual constructors.

### Registration

```rust
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::{LoopMiddleware, LoopMiddlewareRegistry};

impl Plugin for MyPlugin {
    type Deps = ();

    fn name(&self) -> &str { "my-plugin" }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(registry) = ctx.get::<LoopMiddlewareRegistry>() else {
            return Ok(());
        };
        let handle = registry
            .register(std::sync::Arc::new(MyMiddleware))
            .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?;
        ctx.effect(|| async move {
            handle.unregister();
            Ok(())
        })?;
        Ok(())
    }
}
```

- `id()` must be unique; `order()` orders stages (lower first, registration order breaks ties).
- `stages()` declares exactly the stages the middleware implements.
- The chain is snapshotted at run start; a run is not affected by later registrations.
- An empty chain is zero-overhead and produces no span or event.
- Host session slots can derive a scoped registry (`LoopMiddlewareRegistry::derive()`);
  a derived registry freezes the parent snapshot and isolates later registrations.

### Boundaries

- Do not replace ReAct, call the model yourself, or bypass core permission.
- `before_tool` can tighten, change args, or route to another assembled tool; core
  re-checks permission with the final `(name, args)` and reclassifies concurrency.
- `after_tool` receives structured `parts`, `details`, and `is_error`; keep the
  structure intact unless you intentionally rewrite it.
- `decide_continue` is called only on settle. A non-progressing `continue` or
  `compact` is bounded by core.
- Plugin state must be persisted through `RuntimeSessionStore`, not through the
  session-store plugin's service instance.

## Reference implementation

[`plugins/xylitol-official-plugin-loop-policies`](../../plugins/xylitol-official-plugin-loop-policies)
is the plugin-author reference. It covers five stages with real policies:

- `before_tool`: deny configured bash patterns (tightening only);
- `before_llm_request`: narrow the tools the model can see;
- `after_tool`: continue on deterministic failures (self-heal);
- `after_llm_response`: bounded critic pass on final answers;
- `decide_continue`: bounded settle count.

Read its `middleware.rs` and `plugin.rs` together with this guide. It is config-gated
(`loop_policies`), registers nothing by default, and unregisters cleanly on dispose.

## Testing

- Unit-test the middleware directly against the ctx types.
- Test registration/unregistration through a `Ctx` root and a `LoopMiddlewareRegistry`.
- For end-to-end behavior, use the fake provider and the BDD harness under `tests/bdd/`.
