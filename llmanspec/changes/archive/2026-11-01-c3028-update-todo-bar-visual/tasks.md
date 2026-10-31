# Tasks

- [x] 1.1 `glyphs.rs` 新增 `todo_summary` / `todo_doing` / `todo_pending` / `todo_done` 四个字形入口及 ASCII 回落，单测覆盖可见宽度为 1。
- [x] 1.2 `todo_bar.rs` 重写：`TodoBarParams` 换成 `expanded`；`TodoBarHit` 收敛为摘要行单条；删除 `TodoWing`、`plain_border`、`push_stack_wing`、`stack_col`、`zip_cols`、`doing_header` / `pending_header` / `past_header`。
- [x] 1.3 摘要行渲染：`▸ 待办 2/5 · <正文>`，无 in_progress 时省略正文段，超宽右截断。
- [x] 1.4 展开态渲染：doing → pending → completed 分组，组内 SSOT 表序，前导字符与三档取色，续行悬挂 4 cell。
- [x] 1.5 溢出：可见行至多 `max_rows`，末行 `… 还有 N 项 · 滚轮查看`，不画边框。
- [x] 1.6 单测重写：空表 0 行、默认只 1 行、点击展开后分组顺序、三档取色（深色 + 浅色）、溢出提示、CJK 换行不截断。

- [x] 2.1 `fold_hit.rs` 删 `FoldTarget::TodoDoing` / `TodoPending` / `TodoPast` 与 `hover_hits_for` 对应分支。
- [x] 2.2 `layout/root/mod.rs`：三个 `todo_*_open` 换成 `todo_expanded: bool`；`toggle_todo_fold` 改为 `toggle_todo_expanded`。
- [x] 2.3 `click_todo_bar_local` 改为摘要行任意列命中即切换；`scroll_todo_bar_at` 仅在展开时生效。
- [x] 2.4 `render.rs` 的 `render_todo_bar_slot` 与 `estimate_dock_rows` 传新参数；`todo_bar_line_count` 签名同步。

- [x] 3.1 `agent-todo` r1129 重写为摘要 + 按需展开 + 分组列表；删「须画上下 `─` 轮廓」「`↑ N more` / `↓ N more`」「三列须可独立折叠」。
- [x] 3.2 `agent-todo` 新增状态字形与溢出两条规则（前导字符 + 三档取色；末行溢出提示 + 栏内滚动）。
- [x] 3.3 `app-tui-fixed-zone` 待办栏条款同步：删轮廓行与 `N more` 边框表述，保留「空表占 0 行」与固定区顺序。
- [x] 3.4 BDD：`bindings_agent_todo` 与 `steps_app_tui_transcript` 的待办断言改为摘要行与展开后分组；新增点击展开场景。

- [x] 4.1 `just qa`、`just test-tui-e2e-pty`、`llman-sdd validate` 全绿。)
