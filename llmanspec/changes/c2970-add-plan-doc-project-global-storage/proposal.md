---
depends_on:
  - c2965-add-plan-mode-with-plan-doc-and-approval
title: Add project and global scopes for durable plan documents
needs_specs_change: true
---

# Add project and global scopes for durable plan documents

## Why

`c2965` stores plan documents in the session temp directory, so a useful plan disappears when the session ends. Users need plans that survive sessions, either for one repository or across all projects.

## What Changes

- Introduce `PlanScope`: `session` (default), `project`, `global`.
- Storage:
  - `session`: existing CustomEntry SSOT + `<session_dir>/plans/<plan_id>.md` view.
  - `project`: `<project>/.xylitol/plans/<plan_id>.md`; the file (frontmatter + markdown) is the durable SSOT.
  - `global`: `~/.xylitol/plans/<plan_id>.md`; same file format.
- Session entries record a `plan_ref {scope, plan_id, path}` so resume can reopen a project/global plan.
- `plan_write` / `plan_read` / `plan_list` accept `scope`; omitted scope resolves from the active `plan_ref`, then falls back to `session`.
- Project scope is trust-gated: reading or writing project-local plans requires project trust. Untrusted project scope returns a clear error and never silently falls back to session.
- Atomic writes: write a temp file and rename; never leave a partial plan.
- `plan_list` can merge scopes and labels each entry with its scope.
- No automatic mirroring between scopes; moving a plan is an explicit operation.

## Capabilities

- `agent-plan`: scope selection, precedence, durable file SSOT.
- `agent-session-store`: `plan_ref` entry and session/project path resolution.
- `domain-security`: project-local plan access follows the trust gate.

## Impact

- SDK: `PlanScope`, `PlanRef`, path resolution helpers.
- Plan gateway: file backend for project/global; session backend unchanged.
- Tools: `scope` args and list merge.
- Config/docs: default scope and trust behavior.
- Tests: path resolution, trust gate, atomic write, cross-session reopen.

## Out of Scope

- Moving plans between scopes automatically.
- Plan sharing between users or repositories.
- Remote/cloud plan storage.
- Changing session temp plan behavior.

## Further Notes

- Session scope keeps the CustomEntry SSOT from `c2965`; project/global scopes deliberately make the file the SSOT because it must outlive the session.
- Project plans are project-local resources, so the existing trust gate applies rather than inventing a new permission.
