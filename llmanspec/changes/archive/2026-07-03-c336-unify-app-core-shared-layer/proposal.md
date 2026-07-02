---
change_id: c336-unify-app-core-shared-layer
title: 统一三面（print/rpc/tui）的共享装配（bootstrap）与共享命令分发（dispatch）到 app::core
status: proposed
priority: 336
depends_on: []
author: agent
---

# c336-unify-app-core-shared-layer

## Why

xylitol 的应用面现已三面并存（print / rpc / tui），TUI 已通过 c340 落地并 archive。但三面之间的「装配」和「命令分发」两条共享路径**至今未抽取**，导致行为分叉与复制：

### 装配分叉（bootstrap 缺失）

`la15`/`la19` 规范已要求 `app::core::composition` 集中 Agent wiring，但现状 `composition.rs` 只封装了 `build_agent` 一步（93 行）。从 CLI 参数到「构造好的 `ReActAgent`」的完整前置流程（config load → ModelRegistry build → trust resolution → resource 发现 templates/context_files/system_prompt/append → compaction settings → permission → `build_agent` → model select）仍**内联在 `src/app/cli/mod.rs::run()` 约 400 行里**（Step 1-5）。

后果：
- **server 是简化复制版**：`src/app/server/runtime.rs`（198 行）复制了 `build_agent` 调用，但显式跳过 AGENTS.md context_files 和 append_system_prompt（runtime.rs:104 注释自承「headless; resource discovery is the caller's job」）。同一个 agent 配置在 print 和 server 下行为不同——这是**无声 bug 源**。
- **TUI 暂时侥幸**：TUI 没有复制装配，因为 `tui::run(driver: &mut dyn Driver)` 接收已构造好的 driver（cli/mod.rs:469），装配路径与 print 共用。但任何让 TUI 自行装配的需求（如未来 `--remote` 直连、独立 session 加载）都会触发第三次复制。

### 命令分发分叉（dispatch 缺失）

`protocol::Command` 词表是 SSOT，但「一个 Command 变体怎么执行」的分发逻辑**只存在于 `src/app/rpc.rs::dispatch()`**（784 行，独占）。

后果：
- **TUI 的 slash 命令是 stub**：`src/app/tui/commands.rs` 的 `/model` 不工作——注释自承「Full SetModel wiring arrives with c335 shared dispatch」。当前 `/model <id>` 返回 Unknown 错误，`/model`（无参 cycle）也只是构造了 `Command::CycleModel` 后丢弃（commands.rs:35 `let _ = Command::CycleModel { id: None };`）。
- **TUI 够不着 rpc 的 dispatch**：要么复制分派逻辑（必然分叉），要么绕过 protocol::Command 另造一套（违反 write-tui skill tui4 规范明令）。

### 为什么现在做

两个分叉点都是「TUI 已落地后立即显现的痛点」：
- TUI 落地前，dispatch 只有 rpc 一个消费者，分叉不可见；TUI 落地后，`/model` 不工作直接阻塞用户。
- TUI 落地前，装配分叉只影响 print/server 两个面（server 的 resource 跳过是有意为之的 headless 决策）；TUI 落地后，任何 TUI 增强都依赖装配路径稳定。

本变更把「装配」和「分发」两条共享路径一次性下沉到 `app::core`，让 print/rpc/tui（以及未来 server）共用同一实现。**这是「三面并存」架构下保证行为一致性的核心机制**——没有共享层，三面必然漂移。

### 合并 c330 + c335 的理由

原 c330（bootstrap）与 c335（dispatch）是两个独立 draft，但：
- 两者同属「app::core 共享层」这一主题，分两次做会两次触及 cli/mod.rs 的装配路径、两次写 `app/core/mod.rs` 的模块声明、两次跑回归。
- c335 的 dispatch 下沉需要消费 bootstrap 构造好的 agent/driver，强耦合。
- 合并后单次变更覆盖「装配 + 分发」两块，一次回归验证，一次 archive。

## What Changes

### 1. 新增 `src/app/core/bootstrap.rs` — 共享装配入口

```rust
pub struct BootstrapInput { /* config_path, session_arg, model_arg, trust_override, ... */ }
pub struct BootstrappedAgent { agent: ReActAgent, session_id: String, /* discovery results */ }
pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError>
```

封装 cli/mod.rs::run() 中现有的：config load → registry build → trust resolution → resource 发现（templates/context_files/system_prompt/append）→ compaction settings → permission → `build_agent` → model select → `register_prompt_commands`。

### 2. `cli/mod.rs::run()` 改调 `bootstrap(...)` — print/tui 共用

Step 1-5 的内联装配下沉为 `bootstrap(...)` 调用。mode 分发（rpc/print/tui）保持不变，仍接收构造好的 driver。**行为字节级不变**。

### 3. `server/runtime.rs` 改调 `bootstrap(...)` — 消除 server 简化复制

server 的 `build_agent` 调用替换为 `bootstrap(...)`，**同时补回 resource 发现**（context_files/append_system_prompt），消除 server 与 print 的行为分叉。

### 4. 新增 `src/app/core/dispatch.rs` — 共享命令分发

```rust
pub async fn dispatch(driver: &mut dyn Driver, cmd: Command) -> Result<DispatchOutcome, DispatchError>
```

将 rpc.rs::dispatch() 中各 Command 变体的执行语义（select_model/compact/abort/get_state/...）下沉到此。

### 5. `rpc.rs::dispatch()` 改调共享 `dispatch(...)` — 行为不变

rpc.rs 的 dispatch 逻辑替换为对共享 dispatch 的调用。**rpc 各命令行为回归通过**。

### 6. `tui/commands.rs` 改调共享 `dispatch(...)` — 激活 `/model`

TUI 的 slash 命令解析（`/model` → `Command::SetModel`/`CycleModel`）改为调用共享 dispatch，移除当前 stub。

## Capabilities

- `layer-architecture`（修改）：`la15`/`la19` 已要求共享 composition/seam，本变更补齐其未实现的装配前置步骤——`app::core::bootstrap` 是新的共享装配点，`app::core::dispatch` 是新的共享分发点。
- `cli-entry`（修改）：rpc/tui 的命令分派路径统一；装配逻辑下沉到 core，cli 仅做参数解析 + mode 分发 + 调 bootstrap。
- `app-protocol`（修改）：声明 `Command` 的执行语义在共享 dispatch 层，rpc/tui 复用。

## Impact

- **受影响代码**：
  - `src/app/cli/mod.rs`（约 -300 行内联装配，+bootstrap 调用；dispatch 分发路径调整）
  - `src/app/server/runtime.rs`（简化，补回 resource 发现）
  - `src/app/rpc.rs`（dispatch 下沉，-约 150 行，+共享 dispatch 调用）
  - `src/app/tui/commands.rs`（stub 替换为共享 dispatch 调用）
  - 新增 `src/app/core/bootstrap.rs`、`src/app/core/dispatch.rs`
- **受影响规范**：`layer-architecture`、`cli-entry`、`app-protocol`。
- **风险**：中。重构触及 print（已验证主路径）、rpc（已验证）、server（已验证）三条路径。必须有回归测试证明：print 字节级不变、rpc 各命令行为不变、server 行为不变（含 resource 发现补回后的新行为需显式验证）。

## 反降级护栏（防止本变更被降级为「只新增模块但不切换」）

- [ ] print 与 tui MUST 实际调用新 `bootstrap(...)`（非仅新增模块）。
- [ ] server MUST 实际调用新 `bootstrap(...)`，且补回 context_files/append_system_prompt 发现。
- [ ] rpc MUST 实际调用新共享 `dispatch(...)`。
- [ ] tui 的 `/model` MUST 经共享 dispatch 实际切换模型（非 stub）。
- [ ] print 既有输出（prompt 响应、错误信息）MUST 回归通过（字节级）。
- [ ] rpc 既有命令行为（SetModel/CycleModel/Abort/Compact/GetState/...）MUST 回归通过。
- [ ] 共享 dispatch MUST NOT 包含 WS 专属逻辑（Subscribe/ApproveTool/AnswerQuestion 留在 server/ws.rs）。
- [ ] `arch_guard` MUST 不报违规（bootstrap/dispatch 属 app::core seam，经 Driver trait 操作，不 reach into agent/infra 内部）。
