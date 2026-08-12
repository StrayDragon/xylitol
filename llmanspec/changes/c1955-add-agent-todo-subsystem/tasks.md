# Tasks: c1955-add-agent-todo-subsystem

> **门禁**：`readyToImplement=true`（Specs landed）。下方 Apply backlog 由 `llman-sdd-apply` 实施时改回 checkbox 并勾选。
> **硬禁**：本波 Specs landing 不写应用代码。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 Pre-start 核对 | ✅ | D1–D10 |
| 1 Specs landing | ✅ | agent-todo atd1–12；agent-tools r42/t2/t26 |
| 2–5 实现 | 待 apply | 见 Apply backlog |

---

## 0. Pre-start 核对 — ✅

- [x] 0.1 读 session Custom / session_env / default_tools / TUI chrome 与 `c1895`/`c1896` 边界 → 决策钉进 `design.md`
- [x] 0.2 Open Questions → D1–D10；Start readiness 表

## 1. Branch binding + Specs landing — ✅

- [x] 1.1 Branch binding：`change attach`（`sdd/c1955-…`，base_sha=83999076）
- [x] 1.2 新建 live `llmanspec/specs/agent-todo/`：`spec.toon`（atd1–12；场景 `feature: false`）
- [x] 1.3 修订 `agent-tools`：闭集纳入 `todo_list` / `todo_rewrite` / `todo_update`；Barrier 并发（r42/t2/t26）
- [x] 1.4 TUI/栏边界落在 `agent-todo`（未改 `app-tui-transcript` / `agent-session-store`）；commit Specs landing
- [x] 1.5 `llman sdd validate c1955 --strict --no-check --no-interactive`；`readyToImplement=true`

## Apply backlog（实施时勾选）

### 2. 领域 + 持久（垂直薄片）

1. protocol：`agent_todo` Custom payload 形状 + fold-latest helper（leaf 全扫）
2. 校验：非空 content、合法 status、至多一条 in_progress；单测
3. compact/cut 后若无快照残留 → 重 append 最新 list；单测

### 3. 工具面

1. `todo_list` / `todo_rewrite` / `todo_update` TypedTool + 入 `default_tools`
2. tool result = 结构化全表；未知 id / 双 in_progress 可读错误
3. BDD / 单测：工具 ↔ JSONL latest Custom 一致；不进 `project_for_llm` 前缀
4. 确认 c1900：含 Todo 的 builtins 仍首轮一次冻表

### 4. TUI checklist

1. checklist 对话条目：默认一行摘要，可展开完整列表
2. resume / switch_session / todo_* 工具结束后同源刷新
3. harness：摘要计数与 store 一致；无侧栏 / 不占 status 主清单
4. （可选）architecture 词表加「Todo 清单」一行；DESIGN playground 静图

### 5. 收口

1. Print 面：无 UI 但工具+持久可用（冒烟）
2. `just fmt` / 相关 lint / 相关 test；`llman sdd validate` 满闸
3. verify 报告；准备 finalize（勿在 apply 中途 archive）

## 实现顺序

```text
1 Specs landing ✅ → 2 领域+持久 → 3 工具面 → 4 TUI checklist → 5 收口
```
