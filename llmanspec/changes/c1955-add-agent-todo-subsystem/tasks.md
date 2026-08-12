# Tasks: c1955-add-agent-todo-subsystem

> Designed / pre-start。T0 在默认分支可做文档核对；**T1+ 须 Branch binding 之后**（禁止在默认分支改 live specs）。

## 0. Pre-start 核对

- [x] 0.1 读 session Custom / session_env / default_tools / TUI chrome 与 `c1895`/`c1896` 边界 → 决策钉进 `design.md`
- [x] 0.2 Open Questions → D1–D10；Start readiness 表

## 1. Branch binding + Specs landing

- [ ] 1.1 `llman sdd change start c1955-add-agent-todo-subsystem`（干净树 + 默认分支）
- [ ] 1.2 新建 live `llmanspec/specs/agent-todo/`：`spec.toon` + `*.feature`（模型、Custom latest-wins、三工具语义、至多一条 in_progress、栏只读边界、Custom 不进 LLM 前缀）
- [ ] 1.3 修订 `agent-tools`：内置闭集纳入 `todo_list` / `todo_rewrite` / `todo_update`；Barrier 并发
- [ ] 1.4 按需补 `agent-session-store` / `app-tui-transcript`（或 `app-tui-todo`）场景；commit Specs landing
- [ ] 1.5 `llman sdd validate c1955-add-agent-todo-subsystem --strict --no-interactive`；`readyToImplement=true`

## 2. 领域 + 持久（垂直薄片）

- [ ] 2.1 protocol：`agent_todo` Custom payload 形状 + fold-latest helper（leaf 全扫）[blocked-by: 1.5]
- [ ] 2.2 校验：非空 content、合法 status、至多一条 in_progress；单测
- [ ] 2.3 compact/cut 后若无快照残留 → 重 append 最新 list；单测

## 3. 工具面

- [ ] 3.1 `todo_list` / `todo_rewrite` / `todo_update` TypedTool + 入 `default_tools` [blocked-by: 2.1]
- [ ] 3.2 tool result = 结构化全表；未知 id / 双 in_progress 可读错误
- [ ] 3.3 BDD / 单测：工具 ↔ JSONL latest Custom 一致；不进 `project_for_llm` 前缀
- [ ] 3.4 确认 c1900：含 Todo 的 builtins 仍首轮一次冻表

## 4. TUI checklist

- [ ] 4.1 checklist `UiEntry`：默认一行摘要，可展开完整列表 [blocked-by: 2.1]
- [ ] 4.2 resume / switch_session / todo_* 工具结束后同源刷新
- [ ] 4.3 harness：摘要计数与 store 一致；无侧栏 / 不占 status 主清单
- [ ] 4.4（可选）architecture 词表加「Todo 清单」一行；DESIGN playground 静图

## 5. 收口

- [ ] 5.1 Print 面：无 UI 但工具+持久可用（冒烟）
- [ ] 5.2 `just fmt` / 相关 lint / 相关 test；`llman sdd validate` 满闸
- [ ] 5.3 verify 报告；准备 finalize（勿在 apply 中途 archive）
