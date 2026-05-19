# c92-add-tui-approval-diff Tasks

- [ ] **1 — Wire ApprovalOverlay**: modify `app.rs` to detect `ToolCallStart` events that require approval; call `ApprovalOverlay::prompt(name, args)`; route user decision back via approval channel; block agent until decision received
  - Verify: trigger a tool that requires approval → overlay appears → confirm → tool executes
- [ ] **2 — Wire DiffPreviewOverlay**: after `StepComplete`, collect file diffs from ReviewEngine or similar crate; call `DiffPreviewOverlay::set_diff(hunks)`; ensure Ctrl+R toggles visibility
  - Verify: `cargo check --features ui-tui` + manual Ctrl+R after agent edits a file
- [ ] **3 — Refactor diff_preview.rs**: replace custom `DiffKind`/`DiffLine` with imports from `diff_review::types`; use `DiffHunk` directly instead of parsing raw diff text; delete duplicate type definitions
  - Verify: `cargo build --features ui-tui,ui-review` compiles; no duplicate DiffLineKind in codebase
- [ ] **4 — Tool call cards**: in `chat.rs`, add tracking for active tool calls (id, name, status, args); render as styled block cards with status emoji/icon; Enter toggles expand/collapse; args shown when expanded
  - Verify: agent runs a tool → card appears with spinner → completes → checkmark + duration shown
- [ ] **5 — Thinking blocks**: in `markdown.rs` or `chat.rs`, detect `<thinking>...</thinking>` or model-specific reasoning markers; render as collapsible dim blocks; default collapsed
  - Verify: agent emits thinking content → renders as collapsed block → Enter expands
- [ ] **6 — Approval overlay with inline diff**: when a write/edit tool needs approval, generate diff preview in the approval overlay using the shared diff_review types; show file path + unified diff +/- lines within the overlay
  - Verify: `cargo build --features ui-tui,ui-review` + manual: edit tool with approval enabled → diff shown in approval popup
