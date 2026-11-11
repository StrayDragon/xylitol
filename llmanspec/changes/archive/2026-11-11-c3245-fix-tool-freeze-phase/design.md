# Design — 工具表冻结相位（全部已决 2026-11-11）

## 0. 三相位的语义（依据 infra-mcp first-turn-tool-freeze 合约定稿）

| 相位 | 含义 | 进入方 | 退出方 |
|---|---|---|---|
| Unfrozen | 自由装配期（进程启动、idle /reload、resume / 切会话清冻后） | clear_tool_freeze | 装配线收口 / begin |
| Gating | 首轮门禁等待窗口：等 prepare settle 或 15s 超时，以已武装子集定稿 | begin_tool_gating / reopen（装配线内部） | freeze（定稿） |
| Frozen | 定稿：provider 可见工具表冻结，直至 idle /reload 或 resume / 切会话 | freeze_tools | clear（reload/resume）/ reopen（装配线内部） |

不变量（实现必须保证，违反 = 缺陷）：**任何装配线结束时相位必为 Frozen**；Gating 只存在于门禁等待与装配线内部；冻结后唯一合法重开路径是装配线（reopen 与 freeze 成对）或 reload/resume（clear）。

## 1. 缺陷（已核实）

1. **apply_pending_capability_overlay 结束停在 Gating**（in_process/mod.rs:315-330：reopen → set_tools，无终步 freeze）。违反不变量，副作用链已核实：is_tools_frozen=false → LoadedResourcesSnapshot.tools_table_frozen=false → TUI mcp_tools_pending 谓词误显 pending cue；下一次 generate 重新过门（多余等待）；回合内工具表失去冻结保护（只剩 context_policy 一层）。对照：install_base_tools（:304-313）以 freeze_tools 收口，是正确模式；ensure_tool_table_frozen（:336-347）是现成的收口助手。
2. **set_tools / set_tools_defer_prompt 在 Frozen 静默 no-op**（tools_ops.rs:9-39：warn 后返回），调用方无从判断生效与否。
3. **capability 相位用集合差表达**（capability_disabled vs capability_applied 两个 BTreeSet），无类型保护。

## 2. 修复（定稿）

- 装配线收口：apply_pending_capability_overlay 补终步 freeze_tools（对齐 install_base_tools 模式）；reopen 保持装配线内部配对语义（不改公开行为，仅文档化配对约束）。
- Gating 超时：**不新增**——infra-mcp 门禁 15s 墙钟超时已覆盖 Gating 的唯一合法停留场景；补 debug_assert（装配序列外不得观察到 Gating 稳态）与快照一致性回归。
- set_tools 族返回 **ToolsWriteOutcome { Applied, RejectedFrozen, RejectedMidTurn }**，消灭静默 no-op；全部调用点显式处理。
- capability 相位显式：OverlayPhase { Idle, Pending } 字段承载 pending/applied，集合差降为 debug_assert（Idle 时 applied == disabled）。

## 3. 转移矩阵（合法路径定稿）

| 当前 \ 事件 | begin | reopen（装配线） | freeze | clear（reload/resume） |
|---|---|---|---|---|
| Unfrozen | Gating | Gating | Frozen | Unfrozen |
| Gating | Gating | Gating | Frozen | Unfrozen |
| Frozen | 不变（已定稿） | Gating | Frozen | Unfrozen |

## 4. spec delta（landing 时）

- infra-mcp first-turn-tool-freeze：增补一句「能力覆盖（/capabilities）重定稿须以冻结收口，不得停留在未定稿相位」。
- agent-runtime mcp-tool-table-freeze-gate：回归确认（不新增条款）。

## 5. 测试边界（seam）

- 单测：转移矩阵全组合；overlay 应用后断言 phase == Frozen；RejectedFrozen / RejectedMidTurn 返回路径；快照 tools_table_frozen 与实际相位一致。
- BDD：infra-mcp 门禁场景回归（gate 超时收口 / reload 清冻 / capability toggle 下一回合生效且冻结）。

## 6. 风险与回滚

- 风险：Outcome 返回值改动触碰全部 set_tools 调用点（kernel composition 等）——机械适配，编译器兜底。
- 回滚：装配线终步 / Outcome / capability 相位三片独立。

## 7. 附录：场景走查（新实现者必读）

场景：会话配置 MCP 服务 search，运行中用 /capabilities 关闭 fff 工具。

正常生命周期（三条装配线）：

~~~mermaid
stateDiagram-v2
  [*] --> Unfrozen: 进程启动
  Unfrozen --> Frozen: 无 MCP：install_base_tools 直接定稿
  Unfrozen --> Gating: 首次 generate（配了 search）
  Gating --> Frozen: prepare 完成或 15s 超时，以已武装子集定稿
  Frozen --> Gating: 迟到工具下一回合边界重定稿
  Frozen --> Unfrozen: idle /reload 或 resume/切会话清冻
~~~

缺陷现场（apply_pending_capability_overlay，in_process/mod.rs:315-330）：reopen → set_tools 后序列戛然而止，无终步 freeze，相位停在 Gating。连锁三害：下次 generate 重新过门禁（多余等待）；回合内冻结保护失效（只剩 context_policy 次要防线）；快照 tools_table_frozen 谎报 false，mcp_tools_pending 谓词短路失效走长路径，存在「已连接未武装」服务器时 TUI 误显 pending cue（关联扫描 D8）。

修复即把第三条装配线补齐为 reopen → set_tools → freeze（对齐 install_base_tools 范本）。Gating 不加新超时：其唯一合法停留场景已有 15s 墙钟（MCP_FIRST_TURN_GATE_TIMEOUT）兜底，双钟表会打架；改以 debug_assert 强制「Gating 必经门禁收口」。

静默 no-op 例：冻结后调用 set_tools 仅 warn 一行并正常返回，调用方误以为生效——Outcome 返回值让编译器强迫每个调用点处理 RejectedFrozen / RejectedMidTurn。
