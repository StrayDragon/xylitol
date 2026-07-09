---
change_id: c440-refactor-tui-examples
title: "refactor xylitol-tui examples into a single fake coding-agent surface"
status: draft
priority: 440
depends_on: []
author: agent
---

# c440-refactor-tui-examples

## Why

`packages/xylitol-tui` had multiple overlapping examples (`demo`, `showcase`, `show_all`, `agent_demo`) serving different purposes. In practice this created two problems:

1. acceptance and E2E drifted toward the smallest surface (`demo`) while real manual testing happened on a different surface (`show_all`);
2. example-specific width/layout bugs were easy to misdiagnose as terminal-protocol issues.

The package needs one primary example that matches the real product story: a fake coding-agent workflow showing transcript, tools, plan/sidebar, input editor, and a small command/settings surface. Acceptance should target that single surface.

## What Changes

1. Remove the old multi-demo example set and keep one example: `agent_demo`.
2. Rewrite `agent_demo` into a fake coding-agent primary flow, not a kitchen-sink showcase.
3. Keep harness improvements (`render_result`, generic `spawn_example`) and retarget tests/E2E to `agent_demo`.
4. Record the current terminal support matrix in `packages/xylitol-tui/AGENTS.md` and `_HANDOFF.md`.

## Capabilities

- `tui-testing`

## Impact

- `packages/xylitol-tui/examples/`
- `packages/xylitol-tui/tests/`
- `tests/tui_e2e/`
- `packages/xylitol-tui/AGENTS.md`
- `_HANDOFF.md`
- `llmanspec/specs/tui-testing/spec.toon`
