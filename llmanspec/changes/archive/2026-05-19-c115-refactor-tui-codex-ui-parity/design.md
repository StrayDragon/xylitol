# c115-refactor-tui-codex-ui-parity Design

## Goal

- Replace xylitol's legacy boxed layout (Chat/Tools/Input/StatusBar) with a Codex-style layout and interaction model.
- Preserve xylitol backend semantics as SSOT:
  - AgentLoop + AgentEvent stream
  - ToolRegistry execution
  - ApprovalHub + SecurityEngine gating
  - ReviewEngine diff hunks + diff_review UI (must remain untouched)

## Non-Goals

- Re-implement Codex app-server protocol, multi-agent infrastructure, or plugin system.
- Modify `src/interface/diff_review/**`.

## Target UI Structure (Codex-inspired)

We converge on a 2-surface layout:

1) **Transcript Surface** (flex):
   - Renders committed transcript items and the in-flight streaming tail.
   - Uses Codex-style prefix/wrap (e.g. user `› `) and avoids `Borders::ALL` panels.
   - Tool calls and thinking blocks remain visible inline, still collapsible.

2) **Bottom Pane** (fixed height, auto-grow):
   - Composer (textarea) + Footer (hints/statusline)
   - Local popup/view stack that temporarily replaces composer (command popup, history search, selectors)

Overlays that must remain (review-related) are rendered above both surfaces:
- Approval overlay (modal)
- Diff preview overlay (modal)

## Input Routing

Priority order per key event:

1. Top-most overlay (approval/diff/help/transcript) gets first refusal.
2. App-level keymap bindings (quit/interrupt/clear/copy/transcript/raw toggle).
3. Bottom pane:
   - Active view/popup (slash command list, history search, selectors)
   - Else composer textarea

This matches Codex' philosophy: bottom pane decides local focus; app decides process-level actions.

## Visual Style Constraints (from codex styles.md)

- Prefer default foreground + `dim` for secondary text.
- Use limited colors: `cyan` (hints/selection), `magenta` (codex identity), `green` (success), `red` (errors).
- Avoid heavy panel borders; no persistent `Borders::ALL` for primary surfaces.

## Backend Wiring

- Transcript listens to AgentEvent:
  - TextDelta: append streaming assistant tail
  - ToolCallStart/End: create/update tool card items
  - StepComplete: compute diffs (ReviewEngine) for diff preview overlay
  - Error/Repeat: render as inline error cells

- Bottom pane submit/queue semantics:
  - Enter submits immediately.
  - Tab queues when running; otherwise submits (except `!` shell drafts).
  - Esc cancels draft; Esc Esc backtracks previous user message.

- Approval:
  - ToolCallStart triggers ApprovalOverlay if tool requires approval.
  - ApprovalOverlay resolution unpauses tool execution via ApprovalHub.

## Migration Approach

- Implement new Codex-style renderer/components in-place under `src/interface/tui/`.
- Keep existing event loop and tool wiring, but replace `App::render` layout and components.
- Remove ToolPanel/StatusBar from the main render path; keep as fallback until parity is verified.

## Licensing / Attribution

If we directly copy code from `../codex` (Apache-2.0) into xylitol (MIT):
- Add a `NOTICE` (or `THIRD_PARTY_NOTICES.md`) that includes the upstream Codex NOTICE and Apache-2.0 license text.
- Add prominent modification notices in copied files.
