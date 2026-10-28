# Design

## Middleware

Remove the `loaded: AtomicBool` short-circuit. Every `before_model`:

1. Resolve the runtime session store and session id.
2. Load the leaf branch and call `latest_plan_mode`.
3. Update the cached mode.
4. Inject `PLAN_MODE_CONTRACT` only when the fresh mode is `Plan`.

If the store/session is unavailable, keep the previous mode and inject nothing new. This preserves the silent-degradation rule.

## Config

`LoopExampleConfig::default()` sets `plan_mode: true`.

- `execute` remains the default mode, so enabling the middleware does not inject a contract until a switch.
- `configs/example.yaml` documents `plan_mode: true` as the default.
- Schema regenerated.

## Tests

- Middleware unit: first call with `execute` injects nothing; write a `plan_mode` entry, second call injects the contract; write `execute`, third call injects nothing.
- Plugin unit: default config registers `plan-mode` alongside `loop-example`.
- Config schema/example drift test.
- `just qa` and `llman-sdd validate --all --strict`.

## Specs

- `agent-plan`: clarify that a mode switch takes effect on the next model call (already implied by r2948/r2951).
- `agent-runtime`: r2955 already says "read latest"; no wording change needed beyond the implementation fix.
