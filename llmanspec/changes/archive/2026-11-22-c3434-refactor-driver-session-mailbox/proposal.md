---
title: 会话操作命令化与驱动 trait 一致性门禁
depends_on: []
needs_specs_change: false
branch: sdd/c3434-refactor-driver-session-mailbox
base_branch: main
base_sha: a27221dad1c5f7ad533568f6192e0d55d3c37563
---

## Why

c3432 把 `AgentRuntime` 独占移交给 actor 任务，但信箱目前只承载 4 个命令（`run`/`run_turn`/`select_model`/`cycle_model`/`set_thinking_level`/`compact`）。会话绑定、切换、fork、命名四类操作仍走内联 `state_mut()` 路径，因为它们的实现依赖三个 driver 侧字段：

- `reload_cwd`：决定新会话 cwd
- `session_profile`：切换时比对存储 profile、写回新会话
- `prepare_hub`：切换/fork 时释放旧会话的 prepare 槽

这三个字段都是 `Clone` 数据或 `Arc`，**并非独占资源**，本可以共享。c3432 实施期只做到「确认 actor 看不到它们」，没有进一步分析「它们本可以搬进 `SharedSubsystems`」。本 change 补上这一步，把会话四操作真正命令化。

同时补一条长期护栏：仓库有两份 `XyDriver` trait 声明（runtime 端口 31 个方法、app-tui 端侧镜像 42 个方法，共享 30 个），靠手工同步。c3433 那次三个方法（`set_capability`/`clear_capabilities`/`take_mcp_gate_notice`）就是逐一手改两边的，没有编译期护栏。

## What Changes

### 一、会话操作命令化

把三个 driver 侧字段搬进 `SharedSubsystems`，新增四个 `DriverCommand` 变体，让 `session.rs` 的四个方法改走信箱：

| 操作 | 现状 | 目标 |
|---|---|---|
| `switch_session` | 内联，9 处 `state_mut()` 之一 | `DriverCommand::SwitchSession` |
| `new_session` | 内联 | `DriverCommand::NewSession` |
| `fork_session` | 内联 | `DriverCommand::ForkSession` |
| `set_session_name` / `set_session_name_for` | 内联 | `DriverCommand::SetSessionName` / `SetSessionNameFor` |

### 二、script hook 重入审计

`switch_session` 里有 `session_before_switch` 的 cancel hook 与 `session_shutdown_resume` 的 observe hook。命令化后这两个 hook 要在 actor 任务内执行，须确认它们不会回调 driver 造成重入（c3432 设计点 6 的已知风险点）。

### 三、驱动 trait 一致性门禁

新增 `scripts/check_driver_trait_parity.py`，比对两份 `XyDriver` 声明的共享方法 `(name, is_async, receiver, params, ret)` 五元组，不一致即 exit 1。两侧独有方法走白名单。按根 `AGENTS.md` 接线进 `just qa`。

## 实测证据基线（2026-11-22，main @ d6e618a）

- `state_mut()` 调用点：`mod.rs` 31 处、`session.rs` 9 处、`prepare.rs` 6 处，合计 46 处。
- `actor::DriverCommand::` 构造点：`mod.rs` 4 处。
- 两份 `XyDriver` 共享方法 30 个，async 性与签名失配 **0 处**。
- app-tui 侧仍有 4 个同步可变方法：`drain_idle_events`、`take_resync_rebuild`、`set_bash_run_sink`、`bind_debug_scene`；前两个按 c3433 设计点 1 的判定标准宜 async 化，但它们是端侧独有方法，不进共享集。
- 现有门禁脚本先例：`check_crate_graph.py`、`check_layer_invariants.py`、`check_no_version_names.py`、`check_tui_tokens.py`。

## Impact

- 现有 4798 workspace 测试与 1240 BDD 场景为回归地板。
- `just qa` 与 `just qa-e2e` 每阶段实跑。
- 预期净减约 100 行 inline 代码，`state_mut()` 从 46 处降到个位数。
