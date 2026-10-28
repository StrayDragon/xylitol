# Design

## Scope model

```text
PlanScope = Session | Project | Global
PlanRef { scope, plan_id, path: Option<PathBuf> }
```

Precedence for an omitted scope:

1. Explicit `scope` argument.
2. Active `plan_ref` recorded in the session.
3. `Session`.

## Paths

| Scope | Location | SSOT |
|---|---|---|
| Session | `<session_dir>/plans/<plan_id>.md` | `CustomEntry(plan_doc)`; md is a view |
| Project | `<project_root>/.xylitol/plans/<plan_id>.md` | the file |
| Global | `~/.xylitol/plans/<plan_id>.md` | the file |

File format:

```markdown
---
plan_id: p1
title: Ship plan mode
status: approved
scope: project
updated_at: 1760000000000
steps: [{id: s1, content: "...", depends_on: [], status: pending}]
---

# Ship plan mode

1. ...
```

Frontmatter is the machine-readable payload; the body is the human view. Parsing must reject a missing/invalid `plan_id` and unknown step ids.

## Trust

- Project scope requires project trust before read or write.
- Untrusted project scope → `PermissionDenied` with the plan path; do not fall back to session or global.
- Global scope is user-owned and does not require project trust.

## Session reference

- Writing a project/global plan appends `CustomEntry(plan_ref)` with the scope, id and path.
- Resume reads the latest `plan_ref` to resolve the active plan; a missing file is reported as a missing plan, not silently recreated.
- Session-scope plans keep the `plan_doc` entry; no `plan_ref` needed.

## Tools

- `plan_write {scope?, plan_id, title, steps[], status?}`.
- `plan_read {scope?, plan_id?}`.
- `plan_list {scope?}`; omitted scope lists all scopes with labels.
- `plan_submit` works on any scope; approval still seeds todo from the resolved document.

## Atomic write

1. Create the parent directory.
2. Write `<plan_id>.md.tmp`.
3. `rename` over `<plan_id>.md`.
4. On failure, remove the temp file and return an error.

## Specs

- `agent-plan`: scope model, precedence, file SSOT, trust and atomic write.
- `agent-session-store`: `plan_ref` entry and resolution.
- `domain-security`: project-local plan access follows trust.

## Tests

- Unit: path resolution per scope; precedence; list merge.
- Unit: project scope denied without trust; allowed with trust.
- Unit: atomic write leaves no partial file on failure.
- Unit: resume resolves `plan_ref`; missing file is reported.
- BDD: write project plan, reopen in a new session, list it.
- `just qa` and `llman-sdd validate --all --strict`.
