# Tasks

全部在 change 分支（当前 PR 分支）上实施；收口（finalize --into 当前分支）不列为任务。

## T1 — v3 上行透明化（command_backed 全 RAW）

- 在 `src/app/core/host_client/wire_v3_client.rs::{build_request, request_family_shapes}` 将全部 command_backed 方法改走 `Request::Raw`（`host.describe` 与 fbs schema 完整的 `Subscribe` 保留 typed）；v3 上行成为透明信封，注入的 `session_id`/`cwd`/`model_id`/`thinking_level` 全程保真。
- 更新 `tests/bdd/steps_wire_v3.rs` 与 mapping 单测断言：命令上行形状断言由 typed 改为 RAW 原文。
- 校验：`cargo test -p xylitol --lib wire_v3_client host_client` 全绿。

## T2 — 清理上行 typed Command 死代码

- 删除 Rust 侧不再触达的 `command_to_v3` / `v3_to_command` / `v3_command_ptr` / server `to_jsonrpc_request` 的 Command 臂；`xy_wire_v3.fbs` 与 `generated.rs` 的 Command 类型保留（不重新生成）。
- 同步清理引用它们的单测/BDD 断言，确保无 dead import / no unused。
- 校验：`cargo clippy --all-features --all-targets` 0 警告；`cargo test -p xylitol --lib` 绿。

## T3 — get_state 反映写者模型

- `dispatch_readonly_unary` 的 `get_state`：会话写者已装配时返回写者 `current_model`/thinking（无写者才回退 reader/Null）。
- 单测/BDD：`get_state` 对持写者会话返回写者模型（可在 server-core 场景中覆盖）。
- 校验：路由修复后 `/model` 的 `writer_conflict` 症状消失（BDD/实机复测）。

## T4 — Spec 落地（server-core / cli-entry）

- `server-core`：新增规则——任何 session-scoped 命令（含 v3）MUST 携带的会话身份不得因线协议形状转换而剥离，命令 MUST 路由到所指会话（含 set_model/steer/get_state 等）；新增规则——`get_state` 在写者已装配时 MUST 返回写者当前模型。
- 每条规则带嵌套 `场景:` 并绑定步骤（`/model` 切换不再 writer_conflict；get_state 模型收敛）。
- `cli-entry`/`user-experience` 如需展示语义修订一并落地。
- 校验：`llman-sdd validate c2842-update-v3-session-scoped-command-identity --strict`。

## T5 — 门禁与收口预备

- `cargo test --all-features --test bdd` 绿；clippy 0 警告。
- 收口：`llman-sdd change finalize <id> --into <当前 PR 分支>`（沿用 c2841 正确姿势）。
