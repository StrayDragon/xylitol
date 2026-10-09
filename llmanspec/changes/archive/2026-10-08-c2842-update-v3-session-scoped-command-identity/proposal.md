---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# v3 会话命令身份透明化：command_backed 上行全改 RAW 透传 + get_state 反映写者模型

## Why

c2841 已修复 `prompt` 的会话身份保真，但**同类问题覆盖全部 command_backed 方法**：remote driver 的 `unary()` 经 `with_session` 给每个命令注入 `session_id`/`cwd`（JSON 轨 params 透传因此一直正确），而 v3 上行把 command_backed 方法压缩进 typed `Command` 变体——fbs schema 只载各命令的业务字段、**不含这些注入字段**，于是 `set_model`/`get_state`/`steer`/`bash`/`abort` 等全部落回 `fallback_session`。实测症状：

- `/model` 切换报 `writer_conflict: another client is the writer for this session`（set_model 打到 fallback 会话、在错误的槽上抢写者租约）；
- `get_state` 侧写错误会话（fallback 无写者 → `model:null`），客户端状态同步拿不到写者模型；
- 任意 session-scoped 命令的副作用都发生在错误会话上（修掉 prompt 后 c2842 成为同类最后一块）。

这违反 `src/protocol/wire/v3/mod.rs` 自述的「v3 是纯编码层、语义不变」——编码层不得改变 JSON 轨 dispatch 的会话解析语义。

## What Changes

1. **v3 上行透明化**（核心）：`wire_v3_client::build_request` 将**全部 command_backed 方法**改走 `Request::Raw` 原文过线（与 `prompt` 同构；`host.describe` 与 `Subscribe` 等 fbs schema 已完整的类型可维持 typed）。v3 ⇒ JSON 的会话解析语义完全一致，注入字段全程保真。
2. **清理死代码**：RAW 透传后，上行 typed `Command` 路径（`command_to_v3` / `v3_to_command` / `v3_command_ptr` / server `to_jsonrpc_request` Command 臂）不再被触及——删除 Rust 侧映射与相关单测断言，改为断言 RAW 形状；`xy_wire_v3.fbs` 与 `generated.rs` 的 Command 类型保留（fbs 工具链纪律，不重新生成）。
3. **get_state 反映写者模型**（part 2）：`dispatch_readonly_unary` 的 `get_state` 在会话写者已装配时返回写者的 `current_model` / thinking（无写者才回退 reader/Null）。路由修复后客户端 `refresh_fixed_zone_caches` 经 get_state 即可与写者状态收敛（c2841 的 ModelSelect 下行保留为即时通道，本项覆盖其余表面 / 一致性）。
4. **spec 落地**：server-core 新增「session-scoped 命令会话身份」规则（v3 不得剥离注入的 session_id/cwd，run 与命令 MUST 路由到所指会话）与「get_state 反映写者模型」规则；BDD 可执行场景（`/model` writer_conflict 症状、get_state 模型收敛）。

## Capabilities

- `server-core` / `protocol`（wire v3）：会话身份透明化、get_state 写者模型。
- `cli-entry` / `user-experience`：`/model` 在 served 路径上的切换语义修复。
- `test-*`：`steps_wire_v3` 断言迁移、新 BDD 场景。

## Impact

- 修复 served 路径 `/model`/steer/follow_up/bash/abort/get_state 等全部 session-scoped 命令的正确会话路由（消除 writer_conflict、错误槽副作用）。
- 上游不做改变（v3 是纯透传，旧 JSON 语义不变）。
- 删除死代码面：`command_to_v3`/`v3_to_command`（约 70 变体臂 + 单测）。fbs/generated 不动。

## 调研记录

- llman-sdd `attach` 语义与 `base_branch` 记录（分叉源）见 `research/llman-sdd-attach-base_branch.md`；
- OTel BDD 并行干扰机制与前瞻修复见 `research/bdd-otel-parallel-interference.md`（归入本 change 调研，与本次修复正交但同一调研轮产出）。
