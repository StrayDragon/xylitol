---
change_id: c330-extract-shared-app-bootstrap
title: 抽取共享应用装配逻辑（config→registry→trust→resource→build_agent）为 app::core::bootstrap
status: proposed
priority: 330
depends_on:
  - c325-add-app-tui-spec
author: agent
---

# c330-extract-shared-app-bootstrap

## Why

xylitol 的应用面装配逻辑（加载 config → 构建 ModelRegistry → 解析 trust → 发现 resource → 构造 system prompt/compaction/permission → 调 `composition::build_agent`）当前**内联在 `src/app/cli/mod.rs::run()` 约 200 行**里。`server/runtime.rs` 又**复制了一份简化版**（少了 resource/context_files 发现）。这意味着：

- **三份装配代码并存**：print 走 cli/mod.rs 全量版；server 走 runtime.rs 精简版；未来的 tui（c340）若不抽取，将**第三次复制**，且必然再次分叉。
- **行为不一致**：server 当前跳过了 AGENTS.md context_files 和 append_system_prompt（runtime.rs:102-104 注释自承「headless; resource discovery is the caller's job」），print 却有。这种分叉是无声的 bug 源——同一个 agent 配置，print 和 server 下行为不同。
- **变更困难**：信任解析、resource 发现、compaction 读取等任何一处逻辑调整，都要同步改两处以上，极易遗漏。

本变更把「从 CLI 参数到构造好 `ReActAgent`」的公共装配流程抽成 `app::core::bootstrap`，让 print / rpc / tui（以及 server）共用同一条装配路径。**这是「三面并存」架构下保证行为一致性的核心机制**——没有共享装配，三面必然漂移。

### 调研证据

- **codex 的做法**：`codex-app-server-client` crate（`app-server-client/README.md`）存在的全部目的就是「集中化 in-process app-server 运行时生命周期，让 CLI 客户端不必各自重实现」。codex 把装配集中在一个 facade 后，TUI 和 exec 共用。
- **pi 的做法**：`createAgentSession()`（`core/sdk.ts`）是所有面（print/rpc/interactive）共用的会话工厂——装配只写一次，三面消费同一个 `AgentSessionRuntime`。
- **xylitol 现状**：装配逻辑分散在 cli/mod.rs（print 主路径）和 server/runtime.rs（简化复制），**没有共享 facade**。这是与两个对标项目的主要差距之一。

### 为什么是独立变更而非 c340 内联完成

c340（TUI 主体）在落地时**需要**装配好的 agent。若在 c340 内联做完整抽取，会让 TUI 变更同时承担「重构 print/server 装配」+「写 TUI」两件事，违反 write-surface「改动聚焦，不夹带无关重构」。因此拆为独立变更：先让 print/rpc/server 切到共享 bootstrap（本变更），再让 tui 直接消费（c340 只需调用）。c340 落地时若本变更尚未 full 化，TUI 会**临时**直接调用 cli/mod.rs 的既有函数（最小复制），等本变更 full 化后零成本切换。

## What Changes

1. 新增 `src/app/core/bootstrap.rs`（或扩展 `composition.rs`），暴露一个装配入口，例如：
   ```rust
   pub struct BootstrapInput { cli_config_path, session_arg, model_arg, trust_override, /* … */ }
   pub struct BootstrappedAgent { agent: ReActAgent, session_id: String, /* shared discovery results */ }
   pub fn bootstrap(input: BootstrapInput) -> Result<BootstrappedAgent, BootstrapError>
   ```
   封装现有 cli/mod.rs 中的：config load → registry build → trust resolution → resource discovery（templates/context_files/system_prompt/append）→ compaction settings → permission → `build_agent` → model select → `register_prompt_commands`。
2. `cli/mod.rs::run()` 的 print 与 rpc 分支改调 `bootstrap(...)`（行为不变，仅路径下沉）。
3. `server/runtime.rs` 改调 `bootstrap(...)`（**同时补回** resource/context_files 发现，消除 server 与 print 的行为分叉）。
4. tui（c340）将直接调用 `bootstrap(...)`。

## Capabilities

- `cli-entry`（修改）：装配逻辑下沉到 core，cli 仅做参数解析 + mode 分发 + 调 bootstrap。
- `server-runtime`（修改）：复用共享 bootstrap，补齐 resource 发现。
- `layer-architecture`（修改）：`app::core::bootstrap` 是新的共享装配点，与 `composition` 并列（composition 注入 infra ports，bootstrap 编排装配流程）。

## Impact

- **受影响代码**：`src/app/cli/mod.rs`（约 -150 行内联装配，+几行 bootstrap 调用）、`src/app/server/runtime.rs`（简化）、新增 `src/app/core/bootstrap.rs`。
- **受影响规范**：`cli-entry`、`server-runtime`、`layer-architecture`。
- **风险**：中。重构触及 print（已验证）和 server 两条主路径。必须有回归测试证明 print 行为字节级不变。

## 反降级护栏（防止本变更被降级为「只新增 bootstrap 但 print/server 不切换」）

- [ ] print 与 rpc MUST 实际调用新 `bootstrap(...)`（非仅新增模块）——否则等于又造了一个未被驱动的骨架（write-surface 红线）。
- [ ] server MUST 改调 `bootstrap(...)` 且**补回** resource/context_files 发现——否则行为分叉被固化而非消除。
- [ ] print 模式既有行为 MUST 回归通过（含 trust、resource 发现、model 选择、prompt command 注册）。
- [ ] `arch_guard` MUST 不报新的 `agent ↔ infra` 违规（bootstrap 与 composition 同属允许 import 双方的组合根）。
- [ ] 本变更完成前 MUST NOT 新建 tui 代码（tui 是 c340 的职责，避免夹带）。
