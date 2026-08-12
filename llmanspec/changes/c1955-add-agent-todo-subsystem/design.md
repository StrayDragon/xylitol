# Design: c1955 Agent Todo 子系统

> **阶段**：Designed / pre-start（**未** Branch binding；禁止本波改 live specs / 代码）。
> **产品优先线 #1**：先立 Todo SSOT；状态栏族（`c1895`/`c1896`…）仅作日后可选投影。
> **书指针**：Ch2 实验 2-8（`rewrite_todo_list` / `update_todo_status`）→ 形状灵感；书语不进 live specs。
> **同源**：TUI 与内部状态 MUST 同源（`docs/roadmaps/Web与TUI同源.md`）；无第二套「只教 TUI」故事。

## 目标

会话内一份可 resume 的 **Todo SSOT**：

1. LLM 经内置工具读写；
2. TUI 展示同源 checklist；
3. 治目标漂移 / 漏子任务；**不**靠 always-on 状态栏当真源。

## 已钉决策（Open Questions 收口）

| ID | 议题 | 决策 |
|---|---|---|
| **D1** | TUI 展开 vs 折叠；是否占主 transcript | **可折叠对话条目**（新 checklist `UiEntry`）：有条目时默认 **一行摘要**（如 `Todo · 2/5`），用户展开见完整 checklist。**禁止**常驻 Plan/双栏侧栏（`DESIGN.md`）。**禁止**塞进 status / footer / chrome toast 当主清单。 |
| **D2** | 与即时计划（plan）同模型？ | **本波仅 Todo checklist**。即时计划 **不是** 同一模型；若日后需要 → 状态栏 Agent 列独立 kind（`c1896`），不进本 SSOT。产品词汇用 **Todo / checklist**，不用 Plan。 |
| **D3** | max-attempt / 熔断 | **本波不做**。不进 Todo 字段；不进 harness 闸。后置独立 change。 |
| **D4** | 子 agent：共享 vs 命名空间 | **单会话一份列表**（无 namespace）。共享路径 = 同 `XySessionStore` / 同 session JSONL 扫最新快照；runtime 仍隔离（`src/AGENTS.md`）。**本波不实现**子 agent 接线；只钉契约意向。 |
| **D5** | 持久化形态 | **`SessionEntry::Custom`**，`custom_type = "agent_todo"`；**latest-wins 全量快照**（每次变更 append 一条完整 list）。**不**用 `CustomMessage`（避免进 LLM 前缀 / 冒充状态栏）。**不**另开旁路文件。 |
| **D6** | 工具面 | 三工具进 **`default_tools`（Print+TUI）**：`todo_list`（只读）、`todo_rewrite`（整表替换）、`todo_update`（按 id 改 status/content）。形状参考书，**不**照搬书名。并发类 **Barrier/Sequential**。 |
| **D7** | 与状态栏边界 | **Todo = 唯一业务真源**。`c1895`/`c1896` 日后 **只读投影**一帧摘要；**禁止**栏 auto 摘要写回 Todo；**禁止**本波实现栏注入 / Agent 列 publish。 |
| **D8** | 模型如何看见 Todo | **仅 tool result**（及模型主动 `todo_list`）。禁止自动把清单尾插进 system / `session_env` / `<agent_status_bar>`。 |
| **D9** | in_progress 约束 | 列表中 **至多一条** `in_progress`；`todo_rewrite` / `todo_update` 违反则拒绝并返回可读错误。 |
| **D10** | c1900 冻表 | Todo 三工具属 **核心 builtins**，须在会话 **首次 freeze 前** 进入 `tools[]`；禁止 mid-turn 热加。 |

## 领域模型

```text
TodoList {
  items: TodoItem[]   // 有序；空表合法
}

TodoItem {
  id: string          // 工具/UI 稳定引用；rewrite 时可客户端指定或服务端生成
  content: string     // 非空（trim 后）
  status: TodoStatus
}

TodoStatus = pending | in_progress | completed | cancelled
```

| 规则 | 说明 |
|---|---|
| 权威顺序 | `items[]` 数组序 = 展示序 |
| 完成态 | `completed` / `cancelled` 仍保留在表内直至 rewrite 删掉（便于 resume 回顾） |
| 禁止字段（本波） | max_attempt、tool_call_id、assignee、namespace、plan_kind |

## 持久化

```text
tool mutate
    │
    ▼
validate → in-memory fold
    │
    ▼
append SessionEntry::Custom {
  custom_type: "agent_todo",
  data: { "items": [ {id, content, status}, ... ] }
}
    │
    ▼
TUI / resume：扫 leaf 分支上 latest custom_type=="agent_todo"
```

| 点 | 钉死 |
|---|---|
| 为何 Custom | `as_agent_message` 对 `Custom` 返回 `None` → **不进** LLM history；与 D8 一致 |
| 读路径 | **全 leaf 扫描** latest（勿只扫 `build_context_entries` 裁切后的 context 窗——compact 后仍须能拿到 Todo） |
| Fork | 跟随分支：在 **当前 leaf** 上 latest-wins |
| Compact | Custom 不进 LLM；cut 策略不得把「唯一 Todo 快照」裁到不可恢复——实现时：若 cut 会丢掉全部 `agent_todo`，须在 cut 后 **重 append 最新快照**（与 `c1906` session_env 保证同族思路；细节 apply 时单测钉） |
| Export | HTML/JSONL 可见 `[custom:agent_todo]` 或等价；不假装用户消息 |
| 与 c1930 | Todo **不参与** provider `input` 前缀；投影契约无变更义务（除非误用 CustomMessage） |

## 工具面

| 工具 | 参数（意向） | 结果 | 副作用 |
|---|---|---|---|
| `todo_list` | （无 / 可选 filter 本波不做） | 当前全表 JSON | 无 |
| `todo_rewrite` | `{ items: [{id?, content, status}] }` | 写入后全表 | 校验后 append Custom |
| `todo_update` | `{ id, status?, content? }`（至少改一项） | 写入后全表 | 同上；未知 id → 错 |

- 装配：`infra::tools::default_tools`（及 TUI `default_tools_with_ask` 路径的 builtins 基座）。
- Spec：修订 `agent-tools`「七种内置」→ 含 Todo 三工具（具体计数 Specs landing 钉）。
- Tool result：短、结构化 JSON；**完整 list** 回传以便模型无需再 list（`todo_list` 仍保留给 resume/遗忘场景）。
- 事件：沿用既有 `tool_execution_*`；**不**为本波新开 `XyEvent::Todo*`（TUI 可在 todo_* `ToolExecutionEnd` 后或 store 扫描刷新）。
- Ethics：默认进全量表已由产品优先线 #1 确认（proposal escalation 收口）。

## TUI

| 项 | 钉死 |
|---|---|
| 落点 | 新 **对话条目**（checklist / Todo 块）；信息类 **A**；词表对齐 `docs/architecture/TUI信息面与chrome词汇.md`（可增「Todo 清单」一行，apply 时再改 architecture） |
| 默认 | **折叠一行摘要**；展开 = 勾选列表（只读本波；用户手改后置） |
| 刷新 | resume / switch_session：自 store latest Custom 重建；turn 内：todo_* 工具结束后刷新同源模型 |
| 禁区 | 双栏 Plan；占满 status；chrome toast 当清单；ScrollNotice 刷墙 |
| Print 面 | 无 checklist UI；工具与持久仍可用（同源 SSOT） |
| 跨面 | 动作语义 =「查看 / 由 agent 更新 Todo」；Web 未开闸不阻塞；勿发明 TUI 专属第二套状态机 |

## 与状态栏延后边界

```text
                    ┌─────────────────────┐
   tools / TUI ────►│  Todo SSOT (c1955)  │◄── session Custom snapshots
                    └──────────┬──────────┘
                               │ 只读投影（后置）
                               ▼
                    ┌─────────────────────┐
                    │ c1895 Runtime 栏     │  观测 KV；可含「完成数」一类摘要
                    │ c1896 Agent 列       │  typed 通道；禁止反写 Todo
                    └─────────────────────┘
```

| 允许（后置） | 禁止（永远 / 本波） |
|---|---|
| 栏从 SSOT 渲染一帧 `done/total` | 栏内容当 Todo 真源 |
| Agent 列 kind 镜像摘要 | `todo_*` 只写栏不写 SSOT |
| — | 本波实现栏 / refresh 工具 / session_env 混装 Todo |

## 分层落点（组织方向；非文件钉死）

| 职责 | 层 |
|---|---|
| Todo 值类型 / Custom payload 契约 | `protocol`（session 侧） |
| fold latest、校验、工具编排协作 | `agent`（薄；可经 capabilities 持有当前 fold） |
| `todo_*` TypedTool 实现 | `infra::tools` |
| 组合根注入 default_tools | `app/core` |
| checklist `UiEntry` + bridge | `app/tui` |
| 包通用组件（若抽列表渲染） | 仅当复用需要；默认可先产品面内联 |

硬约束：`agent` ↛ `infra`；面只经 `XyDriver`；子 agent 不共享 runtime 队列。

## Capabilities（Specs landing 意向；start 后写）

| Capability | 动作 |
|---|---|
| **`agent-todo`**（新） | 领域：模型、持久 latest-wins、工具语义、至多一条 in_progress、与栏边界 |
| `agent-tools` | 修订内置工具闭集 + 并发类 |
| `agent-session-store` | Custom `agent_todo` 可存可扫（若现有 s2 已覆盖 custom 则只加场景） |
| `app-tui-transcript`（或新 `app-tui-todo`） | checklist 条目折叠/重建；禁侧栏 |

## 验证缝（seam；start 后落 `.feature`）

| 缝 | 断言意向 |
|---|---|
| 工具 → store | rewrite/update 后 JSONL latest Custom 与 tool result 一致 |
| store → resume | 新进程 / switch_session 后 `todo_list` 与落盘一致 |
| 工具 → TUI | todo_* 成功后 checklist 摘要与表一致（harness） |
| 负例 | 双 `in_progress` 拒绝；未知 id 拒绝；Custom 不进 `project_for_llm` 前缀 |
| 回归 | c1900：含 Todo 的 builtins 仍首轮一次冻表 |

## Out of scope（本 change）

- Agent 状态栏 always-on / Lane Runtime / Agent 列（→ `c1895`/`c1896`）
- max-attempt、熔断、自动从轨迹生成 Todo
- 用户手改 checklist、slash `/todo`
- 子 agent 实现与命名空间
- tool_search / MCP / YAML 产品旋钮
- 即时计划（plan）产品

## Ethics 映射

| 条款 | 设计落实 |
|---|---|
| 禁止静默丢用户 Todo | latest 快照 + compact 重挂保证；无静默 truncate 表 |
| 禁止栏冒充 SSOT | D7 / 边界图 |
| 禁止 LLM 编造当权威 | 权威 = store；模型只能经工具写 |
| 默认进全量表 | D6 / D10；优先线 #1 已确认 |

## Start readiness

| 项 | 状态 |
|---|---|
| proposal Why/What/Out-of-scope | ✅ |
| Open Questions 已钉（D1–D10） | ✅ |
| design.md 领域 / 持久 / 工具 / TUI / 栏边界 | ✅ |
| tasks.md 垂直切片 | ✅ |
| Branch binding（`change start`） | ⬜ **未做**（本波硬禁） |
| Specs landing | ⬜ start 之后 |
| `readyToImplement` | ⬜ 需 Full ∧ specsLanded |

**结论**：规划壳已达 **Designed / pre-start**。下一步人类确认后执行 `llman sdd change start c1955-add-agent-todo-subsystem`（干净树 + 默认分支），再 Specs landing。
