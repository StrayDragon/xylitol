# Future — c265-refactor-agent-composition

> 候选待办池，非静态备注。归类为 now（已在本 change）/ later（保留并补触发信号）/ drop（拒绝并说明）。

## now
全部已进 `tasks.md`。本文件只记 later / drop。

## later

### L1 工具自声明能力分类（取代 react.rs 硬编码派发）
- **现状**：`react.rs:99-115` 用硬编码 `match tool_name { "read" => check_read, "write"|"edit" => check_write, "bash" => check_network }` 把工具名映射到 permission check 方法。新增工具需回这里改 match。本轮标 `// NOTE:` 锁定，未解。
- **触发信号**：xylitol 自身要作为 harness，工具数量增长到硬编码 match 难维护；或 MCP/扩展工具也需要 permission 派发。
- **落地路径**：引入 `XyTool::capabilities() -> &[Capability]`（Read/Write/Edit/Exec/Network/Other），`react.rs` 按 capability 派发 check。颗粒度届时按真实工具集定。capability spec 命名 `tool-capability`。
- **第一动作**：`llman-sdd-propose` 一个 `c??-add-tool-capability`，受影响 `tool-system` + `agent-runtime`。

### L2 真隔离：tool-routing 模式（pi Gondolin/OpenShell 范式）
- **现状**：`XyPermission` 是咨询性的，非安全边界（本轮诚实改名 + 文档已说明）。
- **触发信号**：需要跑不可信仓库 / 不可信生成代码 / 无人值守自动化，要求 OS 级强制隔离。
- **落地路径**：按 pi 范式——替换内置工具实现，把 read/write/edit/bash/grep/find/ls 的操作**委托**进容器/VM/micro-VM，而非扩展 `XyPermission`。新增 `sandbox-runtime` 或 `tool-routing` capability。
- **第一动作**：`llman-sdd-propose`，受影响 `tool-system` + 新 `sandbox-runtime`。

### L3 HookDispatcher 与 AgentHooks 的关系审视
- **现状**：两套"hook"机制并存且不重叠——`HookDispatcher`（`infra/hooks/`，脚本式，外部 stdin/stdout，接在 `tui/diff_review`）与 `AgentHooks`（`agent/runtime/hooks.rs`，进程内闭包，本轮接上用于 ReAct 工具调用切面）。本轮刻意不合并。
- **触发信号**：用户反馈"为什么要两套 hook"，或 AgentHooks 需要脚本能力 / HookDispatcher 需要进程内回调。
- **落地路径**：审视是否统一为单一拦截器抽象（或明确分工文档化）。倾向先文档化分工，再视需求合并。
- **第一动作**：探索模式讨论，再决定是否 propose。

## drop

### D1 引入 `AgentMode` / `Capability` preset（本轮否决）
- **理由**：预设 enum 压缩不了二维以上的组合表（design §1），且 `Capability` 颗粒度会被钉死。xylitol 作为 agent 编排核心，不应认识"探索/只读"这类表皮语义。单元操作（`ToolSet::retain`）+ 调用方闭包判断已足够。
- **若日后证明 preset 有价值**：以 L1 的 capability 分类为前提，再讨论是否提供便利构造器（仍非 enum 第一公民）。

### D2 把 `XySandboxEngine` 改造成"外部 sandbox 适配器"
- **理由**：陷阱（design §2）。只要还是 `check_*(target) -> Verdict` 形状，在我们这层就仍是咨询性的（可被忽略）。真隔离走 L2 的 tool-routing，不走扩展 permission trait。

### D3 引入 `SecurityToolWrapper`（security-policy r1 描述但未实现）
- **现状**：`security-policy` spec r1 ("SecurityToolWrapper MUST wrap XyTool ... enforce approval policy") grep 确认**代码零实现**。它是第三种"工具前拦截"设想。
- **理由**：与 `XyPermission`（轮询式）和 `AgentHooks.before`（链式）功能重叠。若未来需要"逐工具审批策略"，应复用 `AgentHooks.before` 或 `XyPermission`，而非再造包裹器。
- **动作**：spec r1 是 stale requirement，候选清理（单独 change 修 spec，或留待下次触及 security-policy 时一并处理）。
