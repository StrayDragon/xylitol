# Design

## Mapping

| Before | After |
|---|---|
| `LoopStage::BeforeModel` | `LoopStage::BeforeLlmRequest` |
| `LoopStage::AfterModel` | `LoopStage::AfterLlmResponse` |
| `LoopMiddleware::before_model` | `LoopMiddleware::before_llm_request` |
| `LoopMiddleware::after_model` | `LoopMiddleware::after_llm_response` |
| `BeforeModelCtx` | `BeforeLlmRequestCtx` |
| `AfterModelCtx` | `AfterLlmResponseCtx` |
| `LoopChain.before_model` | `LoopChain.before_llm_request` |
| `LoopChain.after_model` | `LoopChain.after_llm_response` |
| `run_before_model` | `run_before_llm_request` |
| `run_after_model` | `run_after_llm_response` |
| `loop.middleware.stage=before_model` | `loop.middleware.stage=before_llm_request` |
| crate `xylitol-official-plugin-loop-example` | `xylitol-official-plugin-loop-policies` |
| config `loop_example` | `loop_policies` |
| plugin id `loop-example` | `loop-policies` |
| `LoopExampleConfig` / `LoopExamplePlugin` | `LoopPoliciesConfig` / `LoopPoliciesPlugin` |

## Config migration

`AppConfig` gains `loop_policies`. The loader rejects a `loop_example` key with:

```text
loop_example is no longer supported; use `loop_policies`; todo/workflow/plan keys moved to top-level `todo`/`workflow`/`plan`
```

No alias is kept. This is a pre-release breaking change, consistent with `c2968`'s old-key rejection.

## Docs

Add `crates/xylitol-plugin-sdk/README.md`:

- plugin authors depend only on `xylitol-plugin-sdk`;
- seven stages and `LoopControl`;
- registration / unregistration via `Ctx` + `LoopMiddlewareHandle`;
- session scope `derive()`;
- core owns turn / cancel / permission / persistence; plugins cannot replace ReAct;
- reference implementation: `plugins/xylitol-official-plugin-loop-policies`;
- testing guidance (unit + BDD).

## Tests

- Mechanical rename: compile + existing unit/BDD tests.
- BDD step names that mention `before_model` / `after_model` / `loop 示例插件` / `loop-example` are updated in both the feature file and the step definitions.
- Config loader test rejects `loop_example` with the migration hint.
- Config schema drift test and example drift test stay green.
