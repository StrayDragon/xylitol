---
title: 驱动同步面 async 化：解除 actor 接线的所有权墙
depends_on: []
needs_specs_change: false
branch: sdd/c3433-refactor-driver-async-surface
base_branch: main
base_sha: 039dcbd12129d71ec210dd3817b0f034b09ec8ff
---

## Why

c3432（驱动物理化，路线 A）要把 `XyInProcessDriver` 退化为 handle、由 actor 任务独占 `AgentRuntime`。实施期实测撞上一堵结构性所有权墙：

- 一旦 `ActorState`（含 `AgentRuntime`）移交给 actor 任务，驱动自身的同步方法再也拿不到 runtime。
- 同步方法不能走异步信箱：`blocking_recv` 在 tokio 任务内调用直接 panic（实测 `Cannot block the current thread from within a runtime`）。
- 把信箱调用改成 inline 回退，则 actor 与驱动同时持有 runtime 写权，破坏单写者。

根因在 `XyDriver` 的 trait 面本身：**同步的可变方法**（`&mut self` 且非 async）无法与「runtime 独占于 actor」共存。c3432 的 design.md 第 6 点（重入禁令）与这批同步方法直接冲突。

本 change 不承载新用户语义（`needs_specs_change: false`）：`XyDriver` 是内部端口，不在 `llmanspec/specs/` 的产品合约面内。价值是把 trait 面调整到能承载 actor 模型，让 c3432 的路线 A 得以真正落地。

## What Changes

把 `XyDriver` 上「同步 + 可变」的方法改为 async，使其能经 actor 信箱串行执行：

| 方法 | 现状 | 目标 |
|---|---|---|
| `set_bash_run_sink` | `fn(&mut self)` | **保持 `fn(&mut self)`**（设计点 2：只设驱动侧 sink 槽位，不触碰 runtime） |
| `set_capability` | `fn(&mut self)` 返回 `Result` | `async fn` |
| `clear_capabilities` | `fn(&mut self)` 返回 `Result` | `async fn` |
| `take_mcp_gate_notice` | `fn(&mut self)` | `async fn` |

只读方法（`&self` 且无副作用：`current_model`、`session_id`、`thinking_level`、`is_tools_frozen`、`queue_items`、`capability_snapshot` 等）保持同步——它们从快照读，不需要 actor 往返。`abort` 保持同步（经共享 coordinator 直达，不进信箱）。

实现侧同步改造各 driver 实现与调用点：

- `XyInProcessDriver`（runtime）
- `OccupancyDriver`（kernel 包装）
- `XyRemoteDriver`（app-tui 端侧协议客户端）
- `ScriptedDriver` / `StubDriver` / `OAuthDriver`（测试替身）

## 实测证据基线（2026-11-21，sdd/c3432 @ b1c028f）

- `XyDriver` 上 `&mut self` 且非 async 的方法共 4 个：`set_bash_run_sink`、`set_capability`、`clear_capabilities`、`take_mcp_gate_notice`（`crates/xylitol-runtime/src/driver/proto.rs`）。
- `set_bash_run_sink` 调用点 8 处，跨 app-tui / serve / kernel 测试 / occupancy 测试。
- `set_capability` 调用点 5 处：kernel 生产路径 `command_exec.rs:149`、测试替身 `harness/scripted.rs:1330`、kernel 测试 `tests/mod.rs:194`、BDD `steps_capability.rs:43` 与 `:77`。
- `take_mcp_gate_notice` 调用点 5 处，其中 app-tui `effects/mod.rs:271` 是生产路径。
- c3432 分支实测失败：`cargo nextest run -p xylitol-runtime -p xylitol-kernel` 562 测试 7 失败，关键输出 `panicked at mod.rs:1046: driver runtime already moved into its actor` 与 `handle.rs:438 Cannot block the current thread from within a runtime`。

## Impact

- 现有 1240 BDD 场景与 occupancy 48 例为回归地板。
- app-tui 端侧 `XyRemoteDriver` 走协议 unary，async 化无额外成本。
- c3432 的 actor 接线在本 change 归档后继续；本 change 本身不改驱动所有权结构。
- 实现提交 `b5ea70f` 顺带删除 4 个 app-tui / tui 文件里的 8 行可移除注释（`input_policy.rs:80-81`、`host/mod.rs:450`、`layout/root/mod.rs:1199-1200`、`pointer.rs:247-249`），属注释闸策略的主动清理，与本 change 主改动无逻辑关联。

## Further Notes

- 仓库存在两份 `XyDriver` trait 声明：`crates/xylitol-runtime/src/driver/proto.rs`（runtime 端口，42 个共享方法 + 独有 `run_turn`）与 `crates/xylitol-app-tui/src/driver/proto.rs`（端侧 client 镜像，独有 13 个客户端方法）。本次三个方法在两边逐一手工同步。无编译期护栏——app-tui 不得依赖 runtime，共享子集也不宜抽进 plugin-sdk。建议后续 change 新增 `scripts/check_driver_trait_parity.py` 比对共享方法的 `(name, is_async, receiver)` 三元组，按根 `AGENTS.md` 接线进 `just qa`。
