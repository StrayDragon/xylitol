---
depends_on: []
title: Fix plan mode toggle before the first prompt
needs_specs_change: false
---

# Fix plan mode toggle before the first prompt

## Why

A fresh TUI session receives a session id before anything is persisted. Running `/plan` at that point makes `set_plan_mode` load the leaf branch from a session that does not exist yet, so the store returns `NotFound` and the host surfaces `remote: not_found: not found: <uuid>`. The spec already requires `/plan` and `set_plan_mode` to write mode state immediately; the implementation simply treats "no entries yet" as a failure.

## What Changes

- Treat a missing session in `set_plan_mode` as an empty branch instead of an error. `latest_plan_mode` then falls back to `execute`, so the first toggle enters plan mode and appends the mode entry to the pending store.
- Keep real store failures (I/O, legacy layout, lock) as errors.
- Add a driver-level regression test that toggles plan mode on a bound-but-unpersisted session and toggles back.

## Capabilities

- `agent-plan`: `/plan` works before the first prompt; mode changes still take effect immediately.

## Impact

- `set_plan_mode` command execution in the kernel driver.
- Regression test next to the bootstrap driver harness.

## Out of Scope

- Auto-creating a session header on `/plan`; the first prompt still owns persistence.
- Changes to plan document, approval or read-only semantics.

## Further Notes

- The pending store already creates a header when the first entry is appended, so no explicit `create` call is needed.
