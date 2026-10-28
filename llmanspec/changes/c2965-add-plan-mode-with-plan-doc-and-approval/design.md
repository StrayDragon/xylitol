# Design

## State model

```text
PlanModeKind = Plan | Execute
PlanMode { mode, plan_id: Option<String>, updated_at }
PlanDocStatus = Draft | AwaitingApproval | Approved | Abandoned
PlanDocStep { id, content, depends_on: Vec<String>, verification: Option<String>, status: TodoStatus }
PlanDoc { plan_id, title, status, steps, created_at, updated_at }
```

- `plan_id` is stable and user/model supplied; default when omitted is the active plan.
- `PlanDocStatus::Approved` means the user approved this revision. A later `plan_write` on an approved document resets it to `Draft`.
- Plan steps reuse `TodoStatus` so approval can hand off directly to the todo list.

## Persistence

- `CUSTOM_TYPE_PLAN_DOC = "plan_doc"` CustomEntry, append-only. Latest entry per `plan_id` wins.
- `CUSTOM_TYPE_PLAN_MODE = "plan_mode"` CustomEntry, append-only. Latest entry wins.
- Materialized view: `<session_dir>/plans/<plan_id>.md`, rewritten on each write. It is a **view**, never the SSOT. If the session directory or store is unavailable, the tool still succeeds and returns a warning.
- `plan.md` is not a shared filename: each plan owns `plans/<plan_id>.md` so multiple plans cannot collide. The active plan MAY additionally be mirrored to `plans/plan.md` for convenience; the mirror is best-effort.
- Session temp scope only in this change; project/global scope is `c2970`.

## Tools

All plan tools are sequential and publish state events.

| Tool | Args | Behavior |
|---|---|---|
| `plan_write` | `{plan_id, title, steps[], status?}` | Validate ids/dependencies; write CustomEntry + md view; publish `PlanDocUpdated`. |
| `plan_read` | `{plan_id?}` | Return JSON + markdown of the selected/latest plan. |
| `plan_list` | `{}` | Return summaries `{plan_id,title,status,updated_at}`. |
| `plan_submit` | `{plan_id}` | Set `AwaitingApproval`, ask user, then approve or return to draft. |
| `plan_mode` | `{mode, plan_id?, reason?}` | Agent-requested mode switch; entering always asks, exiting requires an approved plan or ask approval. |

`plan_submit` approval sequence:

1. Load the document; reject empty steps.
2. Persist `AwaitingApproval`.
3. Ask one question (`id = plan_approval`, options `approve` / `revise`, `allow_other = true`).
4. `approve` → persist `Approved`, write `PlanMode { Execute, plan_id }`, seed todo via `AgentTodoGateway::rewrite` (one todo item per plan step, status mapped), publish `PlanDocUpdated` + `PlanModeChanged`, return a short summary.
5. `revise` / skipped / free text → persist `Draft`, keep `PlanMode::Plan`, return the user's answer so the model can revise.

If no ask gateway is installed (print/headless), `plan_submit` returns a structured error telling the caller approval is unavailable; it must not silently approve.

## Mode switching

- User: `set_plan_mode` RPC writes the mode entry immediately and publishes `PlanModeChanged`.
- Agent: `plan_mode` tool. `mode = plan` always asks before switching. `mode = execute` is allowed without a second ask only when the referenced plan is `Approved`; otherwise it asks.
- Denial or skipped ask leaves the previous mode untouched.
- Mode is per session; a new session starts in `Execute` (normal agent behavior).

## Prompt injection

`PlanModeMiddleware` (order 185) reads the latest `PlanMode` at `before_model`:

- `Plan` → append an ephemeral user message `PLAN_MODE_CONTRACT` asking the model to research/read first, write a plan with `plan_write`, then call `plan_submit`; do not edit files before approval.
- `Execute` → inject nothing.
- Store/session unavailable → no injection, no error.

MVP does **not** filter tool schemas or deny tools. That is `c2969`.

## Protocol and client

- Unary `set_plan_mode` (`{mode: "plan"|"execute"|"toggle", plan_id?}`) → `{mode, plan_id}`.
- Downlink event `planModeChanged` (`{mode, plan_id?}`).
- TUI `/plan` parses to a pending slash and calls `set_plan_mode` with `toggle`; `/plan on` / `/plan off` are explicit.
- Rendering the mode indicator is `c2966`.

## Failure modes

- Unknown `plan_id` → `InvalidArgs`, no write.
- Duplicate step id, self-dependency, unknown dependency or cycle → `InvalidArgs` with the offending id.
- Approved document edited → status returns to `Draft`; approval must be repeated.
- Ask cancelled/aborted → tool returns `Aborted`, mode unchanged.
- Store write failure → tool error; in-memory state must not claim success.
- Todo seed failure after approval → mode still switches (approval is a user decision), but the tool reports the seed failure and leaves todo untouched.

## Specs

- New `agent-plan` capability: mode state, plan document, approval/handoff, prompt injection.
- `agent-session-store`: `plan_doc` / `plan_mode` entries and md view.
- `protocol-app`: `set_plan_mode` + `planModeChanged`.
- `app-tui-commands`: `/plan`.
- `agent-runtime`: plan-mode middleware registration.

## Tests

- SDK unit: plan doc round-trip, latest-per-id, mode latest, md rendering, validation.
- Tool unit: write/read/list, submit approve/revise/skip, todo seed, no-ask error.
- Middleware unit: contract injected only in plan mode.
- Protocol/wire: method registry + event round-trip.
- BDD: user toggles mode; agent request denied; approval seeds todo.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：新增 `agent-plan` capability（模式状态、计划文档、审批与 todo 交接、提示注入）；`agent-session-store` 增 `plan_doc` / `plan_mode` 条目与 md 视图；`protocol-app` 增 `set_plan_mode` 与 `planModeChanged`；`app-tui-commands` 增 `/plan`；`agent-runtime` 增计划模式中间件注册场景。测：`llman-sdd validate --all --strict`。
- 2. **SDK 域模型**（sdk）：新增 `session/plan_doc.rs`（`PlanDoc` / `PlanDocStep` / `PlanDocStatus` / `latest_plan_doc` / `plan_docs` / markdown 渲染）与 `session/plan_mode.rs`（`PlanMode` / `PlanModeKind` / `latest_plan_mode`）；新增 `PlanDocGateway` 端口。测：往返、按 id 取最新、校验、md 渲染。
- 3. **计划工具**（tools）：新增 `plan_write` / `plan_read` / `plan_list` / `plan_submit` / `plan_mode`；`SessionPlanDocGateway` 经 `XySessionStore` 落 CustomEntry 并写 `plans/<plan_id>.md`；`plan_submit` 经 ask 审批并调用 `AgentTodoGateway::rewrite` 交接 todo。测：写读列、批准/驳回/跳过、无 ask 网关、todo 交接失败。
- 4. **模式中间件**（plugin）：新增 `PlanModeMiddleware`（order 185，`before_model` 注入 `PLAN_MODE_CONTRACT`）；`LoopExampleConfig` 增 `plan_mode` 开关；注册/卸载单测。
- 5. **协议与事件**（protocol/wire/runtime/serve）：新增 `set_plan_mode` unary（toggle/plan/execute）与 `planModeChanged` 事件；runtime driver 写模式条目并发布事件；wire 双向映射；serve 路由。测：registry、事件往返、路由参数校验。
- 6. **接线**（kernel）：`set_plan_mode` 绑定 driver；计划工具接入 ask/todo 网关；无 ask 场景降级。测：kernel 装配单测。
- 7. **TUI 命令**（app-tui）：`/plan`、`/plan on`、`/plan off` 解析并调用 `set_plan_mode`；忙态策略与错误滚动提示。测：slash 解析快照 + 命令分派。
- 8. **BDD**（tests）：用户切换模式、agent 请求被拒、批准后进入 execute 并生成 todo。测：`cargo test --all-features --test bdd`。
- 9. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
