# Design

## Evidence

- `plugins/xylitol-official-plugin-plan/src/middleware.rs:111-136`：`before_tool` 命中只读闸时改写 `ctx.args` 为 `{blocked_by: plan-mode, ...}`，返回 `LoopStep::stopped(ctx)`。
- `crates/xylitol-runtime/src/agent/runtime/tool_exec.rs:124-137`：`out.control != LoopControl::Continue` → `stop_run = true`，`denied_reason` 记为 `loop middleware <id> stopped at before_tool`。
- `crates/xylitol-runtime/src/agent/runtime/react/mod.rs:1827-1845`：`outcome.stop_run` → 发 `TurnEnd`、跑 `after_turn`、`break 'outer`。
- `llmanspec/specs/agent-plan/agent-plan.feature:24-37`：`r2960` 只规定“拒绝在工具执行前、稳定原因、不得执行”，未规定 run 是否继续。
- `tests/bdd/steps_loop_middleware.rs`：现有断言只覆盖中间件返回值，不覆盖 runtime run 行为。

## Semantics today

- 工具不会执行（`denied_reason` 分支直接返回 error tool_result）；
- 但 run 被终止：模型看不到“你可以改用 plan_write”的下一步机会；
- 用户需要重新发一条消息才能继续。

## Options

### Option A：拒绝即结束 run（保留现状，补 spec + E2E）

- 在 `r2960` 写明“拒绝后本 run 结束”，补一个 BDD：plan 模式下模型调用 `write` → 工具被拒绝且 run 结束。
- 优点：零代码；语义严格；实现简单。
- 缺点：模型无法自我纠正；用户体验偏硬；plan 契约（“用 plan_write 产出计划”）与 run 终止相互矛盾。

### Option B：拒绝但继续 run（推荐）

- core 提供“拒绝但不终止”的通道。两种实现形态：
  - **B1**：`BeforeToolCtx` 增加 `deny_reason: Option<String>`；中间件返回 `Continue` 并设置原因；core 在链结束后读取，走既有 `denied_reason` 分支（不执行工具、回写 error tool_result），但 `stop_run` 保持 false。
  - **B2**：`LoopControl` 增加 `Deny` 变体；`LoopStep::denied(ctx, reason)`；runtime 对 `Deny` 只置 `denied_reason`，不置 `stop_run`。
- 模型在同一回合看到 `Tool 'write' blocked: plan mode is read-only`，可改调 `plan_write` / `plan_submit`。
- 优点：符合 plan 契约；减少用户重发；拒绝仍是“更严”而不是绕过权限。
- 缺点：SDK 词表/上下文变更（需要 `SDK_API_VERSION` bump）；需要防止模型反复撞同一闸（可由 core 的 32-turn 上限兜底，或在中间件内计数后升级为 Stop）。

### Option C：混合（N 次拒绝后停止）

- 前 N 次拒绝继续，超过阈值后 `Stop`。
- 优点：兼顾自我纠正与防死循环。
- 缺点：多一个配置面与计数状态；需要定义“同一 run 内计数”还是“跨 turn 计数”。

## Recommendation

推荐 **B1**（`BeforeToolCtx.deny_reason`）：

- 不改 `LoopControl` 封闭集合，语义更聚焦（“本工具被拒绝”不是“run 控制”）；
- 复用 core 既有 `denied_reason` 路径，改动面小；
- 拒绝仍发生在权限终裁之前/之后由 core 决定，不会绕过 `permission_check`；
- 若担心死循环，先用 core 的 turn 上限兜底，后续再评估 Option C。

## Spec wording (if B)

`r2960` 追加：

> 拒绝 不得结束本 run；模型 须在同一回合收到工具错误结果，并可改调 allowlist 内工具（如 `plan_write` / `plan_submit`）。拒绝 不得绕过 core 权限终裁。

## Test seam

- BDD：mock 模型先调 `write` 再调 `plan_write`，断言第一次被拒绝、run 未结束、第二次成功。
- runtime 单测：`before_tool` 设置 `deny_reason` 后 `PreparedCall.stop_run == false` 且工具未执行。
- 若选 A：BDD 断言 `TurnEnd` 后 run 结束、无后续工具调用。

## Risks

- `deny_reason` 若被后续中间件清空，会变成“拒绝失效”；需要定义链语义（后置中间件可覆盖，或只允许首次设置）。
- 拒绝后继续会让模型在同一回合反复尝试；需监控 turn 上限与 token 消耗。
- SDK 变更需要同步插件作者文档与 `SDK_API_VERSION`。

## Open questions

1. 产品语义选 A 还是 B？（决定是否改 runtime/SDK）
2. 若选 B，拒绝通道用 `BeforeToolCtx.deny_reason` 还是 `LoopControl::Deny`？
3. 是否需要“同一 run 拒绝 N 次后升级为 Stop”？N 取多少？

## Decision (implemented)

- 采用 Option B1：`BeforeToolCtx.deny_reason`，拒绝不置 `stop_run`。
- `prepare_call` 读取链终态 `deny_reason` 后走既有 denied 分支：工具不执行、回写错误结果，但 run 继续。
- plan 只读闸设置 `deny_reason = "plan mode is read-only"` 并返回 `Continue`；保留观测属性与 args 标注。
- `agent-plan` 的 `r2960` 与「拒绝写入后本回合继续」场景已落地；`SDK_API_VERSION` → `0.6.0`。
