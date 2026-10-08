# Tasks

## T1 — capabilities 自持会话身份事实

- `AgentCapabilities`：删 `obs_slot_writes` 字段与 `set_obs_slot_writes`/`obs_slot_writes`；增 `session_name` 自有事实 + `set_session_name_fact`。
- `session_ops.rs`：`set_session` 不再写全局槽；`obs_session_snapshot()` 用自有 fact（`session_name: self.session_name.clone()`）。
- 校验：`cargo test -p xylitol --lib agent::capabilities` 绿。

## T2 — react turn 归因 owner 化 + 删 passthrough

- `react/mod.rs`：删 `set_obs_slot_writes`/`obs_slot_writes` passthrough；`LiveReactArgs`/`ReActConfig` 线程传 `obs_session_name`（build 处取自 capabilities fact）；turn 起点（约 line 899）改用该值，不再读槽。
- 校验：`cargo test -p xylitol --lib agent::runtime` 绿。

## T3 — driver/host/bootstrap 写槽收窄（writer 事件）

- `in_process/session.rs`：`switch_session` 删 gate 块（不再写槽）；`set_session_name` 与 active 分支 `set_session_name_for` 改为 fact + 槽（writer 事件）。
- `bootstrap.rs` 会话恢复：fact + 槽。
- `host.rs new_reader_driver`：删 `set_obs_slot_writes(false)`。
- 改写 `reader_driver_switch_session_does_not_stomp_obs_slot` 测试为「switch 不写槽」结构性断言（删 gate 行，注释更新为 Phase B 语义）。
- 校验：`cargo test -p xylitol --lib app::core::driver` 绿。

## T4 — ai-bridge 收集 per-owner 过滤

- `provider/trace.rs`：`SpanCollectScope::records_for(session_id)`（按 `xylitol.session.id` 过滤）+ 单测。
- `provider/obs_session.rs`：文档标注「默认身份」语义收窄（API 不变）。
- 校验：`cargo test -p xylitol-ai-bridge --lib` 绿。

## T5 — spec + BDD

- `infra-otel` 新规则（r1918+）：materialized 会话观测归因 MUST 来自运行时所持会话事实（与进程槽无关），进程槽仅作默认身份；收集断言按 owner 过滤。带嵌套场景。
- `tests/bdd`：otel fixture `records()` → `records_for(SESSION_UUID)`；otel7 改名走产品路径（`AgentRuntime::set_session_name`）；新场景绑定。
- 校验：`cargo test --all-features --test bdd`（并行）稳定全绿；`llman-sdd validate --strict` 绿。

## T6 — 门禁与收口

- clippy 0 新警告；`just qa`（含 live-provider）绿；实机 served 冒烟（attach/`/model`/run 归因）。
- 收口：`llman-sdd change finalize c2843-refactor-observation-identity-isolation --into <当前分支>`（无需 --no-check）。
