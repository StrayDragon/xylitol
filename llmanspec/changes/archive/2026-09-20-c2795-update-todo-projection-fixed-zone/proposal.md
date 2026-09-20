---
depends_on: []
branch: sdd/c2795-update-todo-projection-fixed-zone
base_branch: main
base_sha: c3eeb42e510a6568658871ca794f4a5cf1547cba
---

## Why

Todo checklist 投影行（`Todo · N/M` 折叠行）目前是**对话区内**的 latest-wins 行：
每次 `TodoUpdated` 事件把旧行删除、在 transcript 尾部重新插入。位置因此随对话
推进漂移——插入后若助手继续输出正文/工具，清单行会被推离视口焦点；它表达的是
「当前任务状态」，本质是**常驻状态**而非对话历史，放对话区内语义与可用性都不佳。

用户实测反馈（2026-09-13，refine-todo-display-planes 归档后目检）：清单行更适合
放**输入框上方固定区**——像队列条/状态条一样常驻、稳定、不必在对话流里找。

## What Changes

- Checklist 从对话区投影行改为下缘 **待办栏**：`队列条 → 待办栏 → 通知条 →
  status → editor`。有条目驻留，空表整栏消失。
- 三区视图（不改 SSOT 顺序）：doing（全部 in_progress，可多条）、pending、completed/cancelled。
  默认展开；三列可独立折上。doing 用正常正文，不用 `[~]`。栏内可划选复制。
  列宽 ≥100 分栏（槽位始终三等分；折叠只收正文、不改列宽），更窄竖叠；条目 ≤80 标量，按栏宽换行、不截断。
- 无 `in_progress` 时不假装焦点，非空翼默认展开条目；零计数翼仍画
  `▾ 0 doing` / `▾ 0 pending` / `▾ 0 completed`，不用 `done`/`todo` 行。
- 对话区 `todo_*` 工具块保留；对话区投影行取消。数据源仍是 `TodoUpdated` 与
  `agent_todo` 快照。
- **先 tui-lab 定案再晋级**产品 `todo-bar` 模块；Alt+E 不再驱动待办栏。

## Capabilities（预估，正式化时核对）

- `agent-todo`：r1129 主清单落点改为下缘固定区卡片；r1130 resume 写入同一卡片；
  r1127 只读 SSOT 边界保留（本卡片 ≠ status）。
- `app-tui-fixed-zone`：r21 待办栏为下缘成员（队列条与通知条之间）+ r1228
  footprint 计入实际行 + r1231 文档化槽。
- `app-tui-transcript`：r1353/r1365 信封与簇不再收纳待办栏投影（`todo_*`
  工具块仍在时间线）。
- 跨端同源：语义「常驻任务清单」可写约束；gpui 未开放，本波不写现行 MUST。

## Impact

- 行为合约变更 → 完整 SDD。与 `c2805-update-todo-llm-api` 无依赖（彼改工具原语，
  本波只搬家）。
- 下缘顺序受既有条款锁定：status 紧贴 editor（r1237/r1217）、通知条紧贴 status
  （r1227）→ 待办栏只能在队列条与通知条之间（或队列条之上）：
  `队列条 → 待办栏 → 通知条 → status → editor`。
- Alt+E（`app.tools.blocks`）今日连动 tools/compaction/todo；迁出后不再挂该和弦。
- 设计：`designing/tui-lab/modules/todo-bar` 定案 → 晋级 `tui/modules/todo-bar`；
  改 activity-fold 对齐句 + `just export-design-frame`。

## Decisions

1. 三列独立折（doing | pending | completed）；默认展开；doing 不用 `[~]`；栏内可划选复制。
2. 无焦点：不假装当前任务；计数写在翼头 `n doing` / `n pending` / `n completed`（含 `0 …`）；不用 done/todo 行；空表才整栏消失。
3. 接缝五条全要（lab / 输入坞+footprint / bridge / att36+断言搬家 / activity_fold）。
4. **非本波**：条目绑定 message id、嵌套、默认绑最新、完成时自动记录 → `c2805-update-todo-llm-api`。
5. ≥100 列分栏 doing|pending|completed（槽位始终三等分；折叠只收正文、不改列宽）、更窄竖叠；content ≤80 标量写入拒绝；绘制换行、禁止省略截断；内容可见行 ≤ min(6, ⌊term_rows/4⌋)，超出栏内滚动；有表时上下 `─` 轮廓，溢出用 `↑ N more` / `↓ N more`（同 editor）。

## Further Notes

调研：`research/fixed-zone-todo-card.md`（现状投影、输入坞硬约束、接缝、合约）。
