# 设计：Loop 中间件契约载荷（结构化结果 / 工具路由 / 压缩决策）

## 1. 三个缺口同源

c2944 的七阶段 SPI 已能「有序参与」，但三处 ctx 载荷不足以表达真实决策：

| 缺口 | 现状 | 后果 |
|---|---|---|
| `after_tool` 只有 `result: String` | 工具可产出 `AgentPart::Image`、`details`（write/edit 的 JSON）被 `parts_preview_text` 拍平 | 结构化结果不可逆丢失；插件无法读结构化字段 |
| `before_tool` 只能改 `args` | `env.tools.get(name)` 在中间件之前绑定，并发窗也按原名规划 | 「改路由」不可达；路由后并发安全无定义 |
| `compact` 无载荷 | `instructions=None`、`CompactionStart.reason` 恒 `manual` | 插件不能指定压缩焦点，观测无法区分来源 |

三者都是**插件 ctx 形状**，一次升级比两轮破坏性适配更省（已并入原 c2947）。

## 2. `after_tool` 结构化载荷

```rust
pub struct AfterToolCtx {
    pub turn_index: u32,
    pub tool_call_id: String,
    pub tool_name: String,
    pub parts: Vec<AgentPart>,
    pub details: Option<Value>,
    pub is_error: bool,
}
```

- `parts` 为工具原始产出（经 AgentHooks / 脚本 hook 的既有改写后）。
- `details` 在工具文本可解析为 JSON **对象**时填入该对象，否则 `None`。插件因此能直接读 `success` / `display_diff` 等字段，不必自己 parse。
- 不保留旧 `result: String` 别名（AGENTS：改名不留兼容别名）。

**执行顺序（保持 hooks 旁路不变）**：

```text
工具执行 → AgentHooks.after_tool_call → 脚本 hook tool_result_post
  → middleware.after_tool（结构化 ctx）
  → core quiet（write/edit 摘要）
  → 落 history / ToolExecutionEnd
```

**quiet 与结构化结果的合成规则**：

- 若最终 `parts` 恰为单个 `Text` 且工具是 `write`/`edit`：走既有 `quiet_write_edit_for_history`（保留摘要文本与 details）。
- 否则：history 直接用 `parts`，details 用 `ctx.details`。
- quiet 返回 `None` details 时回退 `ctx.details`（插件给的 details 不被丢弃）。

## 3. `before_tool` 改路由

### 3.1 为什么需要「批次前预演」

并发窗（`Parallel` / `Barrier`）在 `run_one` 之前按**原工具名**的 `execution_mode()` 规划。若中间件把 `read` 路由到 `write`，仍在并行窗执行 → 破坏串行安全假设。

因此把门禁从「执行前一刻」提到「批次规划前」：

```text
for each tool_call:
  hooks(tool_call_pre) → AgentHooks.before_tool → middleware.before_tool
    → core permission（用最终 name+args 重算 target）
  ⇒ PreparedCall { id, final_name, final_args, tool, denied, fault, stop }
按 PreparedCall.final_name + tool.execution_mode() 重新 classify → plan_windows
逐窗执行（ToolExecutionStart 在此刻发，带最终 name/args）
```

### 3.2 不变量

- **顺序不变**：脚本 hooks → AgentHooks → middleware → permission。只是整体前移到批次前。
- **权限只能更严**：路由后 core 用 `(final_name, final_args)` 重算 target 再裁一次；中间件放行不能撤销上游拒绝。
- **事件口径**：`ToolExecutionStart.name` 与 `args` 均取最终形状；`Start` 仍在执行时刻发（不提前到预演），保持「每调用 Start→Update→End」既有节奏。
- **并发分类**：按最终工具重新 `classify`（MCP 前缀仍强制 Sequential，不因路由绕过）。
- **拒绝/故障**：预演阶段记录的 deny / fault 在轮到该调用时照旧回写 tool-error，`stop_run` 语义不变。

### 3.3 边界

- 不新增工具调用（只改已有调用的 name/args）。
- 不允许跨工具表族路由（MCP → 内置）绕过前缀纪律：路由目标仍须在 `ToolSet` 内；找不到即 tool-error。
- `before_tool` 预演每调用最多一次（沿用 2s 超时 + 取消竞速）。

## 4. `compact` 决策载荷

```rust
pub struct CompactRequest {
    pub instructions: Option<String>,
    pub reason: Option<String>,
}

pub struct DecideContinueCtx {
    ...
    pub compact: Option<CompactRequest>,   // 输出位：中间件填，core 读
}
```

- 中间件返回 `LoopControl::Compact` 时，core 用 `ctx.compact` 执行一次强制压缩。
- `reason` 进入 `CompactionStart.reason` / `CompactionEnd.reason`，稳定格式 `loop_middleware:<id>`（缺省由 core 用中间件 id 补全）。`compaction_reason_kind` 增加该前缀映射，观测不再把它归为 manual。
- `XyCompaction::compact` 增 `reason: &str` 参数（原硬编码 `"manual"` 改为传入值）；既有调用点传 `"manual"`。
- 压缩后结算：orchestrator 已在成功后发 `ContextTokenSettlement`（`AfterCompaction`），core 只负责把它转进 run 事件流（既有 `CompactionStreamTee` 已覆盖），本 change 加回归断言而非新增机制。
- 硬上限沿用 c2944（`compact` 与 `continue` 各自 32 次无进展闸）。
- 压缩失败仍以错误结束本 run，消息带中间件 id。

## 5. 兼容与迁移

- `AfterToolCtx` / `DecideContinueCtx` 形状变更 = 破坏性插件 API 变更；仓库 pre-1.0，一次性改调用点，不留别名。
- `XyCompaction::compact` 新增参数：Noop 实现与 orchestrator 同步；测试调用点补 `"manual"`。
- 官方示例插件 `loop-example` 不使用 `after_tool`，本 change 不改其行为；结构化载荷的第三方用法由 runtime 单测 + BDD 覆盖（MCP 摘要迁移留给 c2950）。

## 6. 验收

- `agent-runtime`：更新 `r2922`（允许改路由 + 重裁 + 重分类 + 事件口径），新增 `after_tool` 结构化载荷与 `compact` 请求两条规则及可执行场景。
- `just qa` 绿；`validate --strict` 绿。
