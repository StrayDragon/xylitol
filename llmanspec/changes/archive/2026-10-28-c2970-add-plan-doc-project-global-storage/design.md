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

## Task breakdown

- 1. **specs**（spec）：`agent-plan` 增 scope/优先级/文件 SSOT/原子写；`agent-session-store` 增 `plan_ref`；`domain-security` 增项目计划 trust。测：`llman-sdd validate --all --strict`。
- 2. **SDK**（sdk）：新增 `PlanScope` / `PlanRef` / 路径解析与 frontmatter 编解码；`plan_ref` CustomEntry 读写。测：路径、解析、往返。
- 3. **文件后端**（tools/plugin）：project/global 计划读写走原子写；session 仍用 CustomEntry + md 视图。测：原子写失败无残片。
- 4. **trust 闸**（plugin）：project scope 未信任时 `PermissionDenied`，不降级；global 不受项目 trust 限制。测：trust 允许/拒绝。
- 5. **工具参数**（tools）：`plan_write` / `plan_read` / `plan_list` 增 `scope`；`plan_list` 跨 scope 合并。测：scope 优先级与列表标签。
- 6. **会话恢复**（sdk/runtime）：`plan_ref` 解析活动计划；文件缺失明确报错。测：新会话重开项目计划。
- 7. **BDD**（tests）：项目计划写入、跨会话读取、未信任拒绝。测：`cargo test --all-features --test bdd`。
- 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
