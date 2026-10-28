---
depends_on:
  - c2965-add-plan-mode-with-plan-doc-and-approval
title: Add a one-line ModeStrip indicator for experimental planning modes
needs_specs_change: true
branch: sdd/c2966-add-tui-mode-strip
base_branch: main
base_sha: 5fd0228024e64154ef9acf58a82b7fb15ad6e62c
---

# Add a one-line ModeStrip indicator for experimental planning modes

## Why

Plan mode and workflow execution are experimental and must be visible without turning the fixed zone into a banner wall. Users need a persistent, low-noise cue that the agent is in a special mode, and future experimental modes need the same slot.

## What Changes

- New `ModeStrip` fixed-zone component: exactly one line, a leading glyph plus a short state label, hidden when no mode is active.
- First consumer: plan mode. `planModeChanged` updates the strip (`◆ plan` / `◆ execute · <plan_id>`).
- Placement: below the todo bar and above the toast notice, so it never competes with transient errors or conversation content.
- No interaction, no expansion, no detail pane; details stay in the plan document / scrollback.
- Fixed-Zone Footprint counts one row only while visible.
- Update the fixed-zone order and add a `mode-strip-single-line` rule.

## Capabilities

- `app-tui-fixed-zone`: component shape, placement and footprint.
- `app-tui-bridge` / `app-tui-host`: map `planModeChanged` into `UiModel`.

## Impact

- `crates/xylitol-app-tui/src/widgets/mode_strip.rs` (new).
- `UiModel` gains an optional mode-strip state.
- `layout/root` renders and reserves the row.
- Host event handling for `planModeChanged`.
- DESIGN.md / AGENTS wording for the new fixed-zone slot.

## Out of Scope

- Interactive mode switching UI (the `/plan` slash already exists in `c2965`).
- Mode details, history or per-plan switching UI.
- Any new wire method or event (reuses `planModeChanged`).

## Further Notes

- The strip is intentionally generic: `ModeStrip { glyph, label }` can later carry workflow or other experimental modes without new layout rules.
- Single-line only; long labels truncate with the existing right-truncation rule rather than wrapping.
