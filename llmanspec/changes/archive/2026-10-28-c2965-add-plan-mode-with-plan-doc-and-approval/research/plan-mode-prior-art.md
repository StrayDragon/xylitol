# Plan mode prior art

Scope: how established coding agents separate planning from execution. This note informed `c2965` (plan mode MVP) and the deferred `c2969` (read-only enforcement).

## Claude Code

- Permission modes separate normal editing from planning/auto modes; the docs describe mode-specific approval behavior rather than a single global switch.
- Source: <https://code.claude.com/docs/en/permission-modes>

## Cursor Plan Mode

- Plan mode gives the model dedicated tools to create/update a plan and an interactive editor to revise it inline.
- The agent researches the codebase, asks clarifying questions, then writes a Markdown plan with file paths and code references.
- The user reviews/edits the plan, then builds directly from it; the plan can optionally be saved into the repository.
- Source: <https://cursor.com/blog/plan-mode>

## Aider architect mode

- Two-stage flow: an architect model proposes the change, then an editor model turns the proposal into concrete file edits.
- Mode is explicit (`/architect`, `/chat-mode architect`) and the prompt shows the active mode.
- Source: <https://aider.chat/docs/usage/modes.html>

## OpenAI Codex plan mode

- Plan mode is a collaboration mode with strict rules: it ends only on an explicit mode change, and user imperative language does not exit it.
- Non-mutating exploration is allowed (read/search, static analysis, dry runs, builds/tests that only touch caches); mutating actions are forbidden.
- It is explicitly distinct from the `update_plan` checklist/todo tool; `update_plan` is not plan mode.
- Source: <https://github.com/openai/codex/blob/main/codex-rs/collaboration-mode-templates/templates/plan.md>

## Takeaways adopted here

1. **Mode is explicit state**, not an inference from user wording. `c2965` stores `plan_mode` per session.
2. **The plan is an artifact** the user can inspect and revise. `c2965` uses a `PlanDoc` CustomEntry plus a Markdown view.
3. **Approval ends planning.** `c2965` requires explicit user approval before switching to execute.
4. **Planning is non-mutating.** Deferred to `c2969` so the MVP can validate the workflow first; the design keeps the gate tightening-only.
5. **Todo is not plan mode.** Codex makes this distinction explicitly; our hierarchy is workflow → N plans → N todos, and todo remains an independent progress ledger.
