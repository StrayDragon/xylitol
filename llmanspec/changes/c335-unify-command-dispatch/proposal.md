---
change_id: c335-unify-command-dispatch
title: 统一 Command→执行语义的分发逻辑，供 rpc/tui 共用
status: proposed
priority: 335
depends_on:
  - c325-add-app-tui-spec
author: agent
---

# c335-unify-command-dispatch

## Why

xylitol 的 `protocol::Command` 词表（`SetModel`/`CycleModel`/`Abort`/`Quit`/`Compact`/`GetState`/… 20 个变体）是 client→core 消息的 SSOT，但「一个 `Command` 变体具体怎么执行」的分发逻辑**当前只存在于 `src/app/rpc.rs::dispatch()`**。这意味着：

- **TUI（c340）要执行 slash 命令（`/model`→`SetModel`、`/exit`→`Quit` 等），却够不着 rpc.rs 的 dispatch**——它要么复制一份分派逻辑（必然分叉），要么绕过 `protocol::Command` 另造一套命令语义（违反 write-tui skill 明令「不另造命令体系」）。
- **rpc 是当前唯一驱动 `Command` 词表的应用面**；一旦 TUI 出现，命令行为必须在两面一致（例如 `/model` 在 rpc 和 tui 下走相同的 `SetModel` 执行路径），否则用户在不同面看到不一致行为。

本变更把「`Command` → 在 agent/driver 上执行 → 产出 `Event`」的分发逻辑，从 rpc.rs 提到一个共享位置（如 `app::core::dispatch`），让 rpc 和 tui 共用。**这是「三面并存」架构下保证命令行为一致性的核心机制**。

### 调研证据

- **codex 的做法**：`app-server-protocol` 是纯协议 crate，`app-server` 是统一后端，`app-server-client` 是 facade。三面（in-process/stdio/socket）speak 同一个 protocol，分发逻辑在后端写一次。codex 明确：in-process 路径「reuse the same JSON-RPC result envelope internally」（`app-server-client/README.md`）。
- **pi 的做法**：rpc-mode 的 `RpcCommand` 分发与 interactive-mode 的 slash 命令最终都落到 `AgentSession` 的同一组方法（`session.prompt/compact/setModel/...`）——命令语义在 session 层统一，面只负责解析与调用。
- **xylitol 现状**：`Command` 词表已 SSOT 化（好），但 dispatch 只在 rpc.rs（差）。这是「协议统一但分发未统一」的半成品状态。codex/pi 都把 dispatch 收敛到了共享层。

### 为什么是独立变更而非 c340 内联完成

c340 的 slash 命令解析（`/model` → `Command::SetModel`）需要消费 dispatch。若在 c340 内联做完整 dispatch 抽取，TUI 变更会同时承担「重构 rpc.rs dispatch」+「写 TUI」两件事。拆为独立变更：先把 dispatch 提到共享层并让 rpc 切换（本变更），再让 tui 直接消费（c340 只需调用共享 dispatch）。c340 落地时若本变更未 full 化，TUI 会**临时**复用 rpc.rs 的 dispatch 函数（若可见）或最小本地分派，等本变更 full 化后切换。

## What Changes

1. 新增 `src/app/core/dispatch.rs`（或扩展 `composition.rs` / `driver.rs`），暴露：
   ```rust
   pub async fn dispatch(driver: &mut dyn Driver, cmd: Command) -> Result<DispatchOutcome, DispatchError>
   ```
   将 rpc.rs 中 `Command` 各变体的执行语义（调 driver/agent 的 select_model/compact/abort/get_state/...）下沉到此。
2. `rpc.rs::dispatch()` 改调共享 `dispatch(...)`（行为不变）。
3. tui（c340）的 `commands.rs` 将调用共享 `dispatch(...)`，不再自带执行语义。

注意：`Subscribe`/`ApproveTool`/`AnswerQuestion` 三个变体被 rpc 显式拒绝（属 WS 协议，非 stdio），它们**不进**共享 dispatch（保留在 server/ws.rs 的 ReverseRpcGateway），避免把传输特有逻辑泄漏进共享层。

## Capabilities

- `app-protocol`（修改）：声明 `Command` 的执行语义在共享 dispatch 层，rpc/tui 复用。
- `cli-entry`（修改）：rpc 与 tui 的命令分派路径统一。

## Impact

- **受影响代码**：`src/app/rpc.rs`（dispatch 下沉，-约 100 行，+共享 dispatch 调用）、新增 `src/app/core/dispatch.rs`。
- **受影响规范**：`app-protocol`、`cli-entry`。
- **风险**：中。触及 rpc（已验证）的命令行为。必须有回归测试证明 rpc 各命令行为不变。

## 反降级护栏（防止本变更被降级为「只新增 dispatch 但 rpc 不切换」）

- [ ] rpc MUST 实际调用新共享 `dispatch(...)`（非仅新增模块）——否则又是未被驱动的骨架。
- [ ] rpc 既有命令行为（SetModel/CycleModel/Abort/Compact/GetState/...）MUST 回归通过。
- [ ] 共享 dispatch MUST NOT 包含 WS 专属逻辑（Subscribe/ApproveTool/AnswerQuestion 留在 server/ws.rs）。
- [ ] `arch_guard` MUST 不报 `agent ↔ infra` 违规（dispatch 属 app::core，经 Driver trait 操作，不 reach into agent/infra 内部）。
- [ ] 本变更完成前 MUST NOT 新建 tui 代码（避免夹带）。
