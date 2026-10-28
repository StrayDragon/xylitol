# Design

## Problem

`set_plan_mode` reads the leaf branch before deciding the next mode. A session bound before its first prompt has no header or entries, so the store returns `NotFound`. The command surfaces that as a remote error and `/plan` fails on a fresh TUI session.

## Decision

Treat `XySessionStoreError::NotFound` from `load_leaf_branch` as an empty branch. `latest_plan_mode` already defaults to `execute`, so `toggle` enters `plan` and the mode entry is appended to the store's pending buffer. Other store errors still propagate.

## Alternatives

- **Create the session header in `set_plan_mode`.** Rejected: it would make `/plan` materialize sessions that the user never used, changing session lists and persistence timing.
- **Return a clearer error.** Rejected: the spec requires the toggle to work, and a fresh session is a normal state, not an error.

## Test seam

The kernel bootstrap harness builds a real in-process driver with a persisted `SessionManager`. The regression test binds a session id without creating it, toggles twice, and asserts the outcome plus the pending branch contents.
