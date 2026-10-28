---
depends_on:
  - c2965-add-plan-mode-with-plan-doc-and-approval
title: Fix plan mode middleware reload and enable it by default
needs_specs_change: true
---

# Fix plan mode middleware reload and enable it by default

## Why

`c2965` shipped plan mode, but the middleware loads `plan_mode` only on the first `before_model` call. After a user runs `/plan` or approves a plan, the cached mode stays stale: the planning contract is never injected (or keeps being injected after approval). The config default is also `false`, so `/plan` toggles state without any prompt effect.

## What Changes

- Reload the latest `plan_mode` entry on **every** `before_model` call instead of once per middleware lifetime.
- Default `loop_example.plan_mode` to `true`; when loop-example is enabled, `/plan` works out of the box. Mode `execute` remains the default, so no contract is injected until the user/agent switches.
- Update config schema/example and the plugin unit tests.
- Add a middleware test that changes mode after the first call and observes the contract appearing/disappearing.

## Capabilities

- `agent-plan`: mode changes must take effect on the next model call.
- `agent-runtime`: middleware reads the latest mode, not a startup snapshot.

## Impact

- `PlanModeMiddleware` state handling.
- `LoopExampleConfig::default()` and `configs/example.yaml` / schema.
- Unit tests.

## Out of Scope

- Read-only enforcement (`c2969`).
- Plugin extraction (`c2968`).
- Any change to plan document or approval semantics.

## Further Notes

- Reading the mode entry every model call is a small append-only store read; correctness beats the one-time cache.
- The default is safe because `execute` injects nothing; enabling plan mode only changes behavior after an explicit switch.
