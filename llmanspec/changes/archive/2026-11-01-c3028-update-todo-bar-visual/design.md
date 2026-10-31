# 设计

## 呈现形态

### 收起（默认，1 行）

```
▸ 待办 2/5 · write todo-bar copy
```

### 展开（至多 6 行）

```
▾ 待办 2/5 · write todo-bar copy
  → write todo-bar copy
  · paint lab states
  · graduate to product
  ✓ read glossary
  ✓ skip overlay slot
```

### 溢出

```
▾ 待办 1/8 · write todo-bar copy
  → write todo-bar copy
  ✓ read glossary
  · paint lab states
  · graduate to product
  … 还有 4 项 · 滚轮查看
```

## 数据结构

`TodoBarParams` 字段改为：`list`、`expanded`、`scroll`、`width`、`max_rows`。删除 `doing_open` / `past_open` / `pending_open`。

`TodoBarHit` 简化为单条摘要行命中：`row`、`col_start`、`col_end`。删除 `TodoWing` 与 `FoldTarget::Todo*`。

## 布局算法

1. 空表返回 0 行。
2. 计算摘要行：`{tri} 待办 {done}/{total}`，有 in_progress 时追加 ` · {正文}`（右截断到栏宽）。
3. 未展开返回 `[摘要行]`，命中区覆盖整行。
4. 已展开：按 doing → pending → completed 分组输出条目，每条 `  {glyph} {正文}`，续行悬挂 4 cell（2 前导 + 2 缩进）。
5. 总行数超 `max_rows` 时按 `scroll` 取窗口，末行换成 `… 还有 N 项 · 滚轮查看`。

## 取色

```
摘要 tri 与「待办 n/m」 → muted
摘要「· 正文」        → on_surface
→ doing              → on_surface + bold
· pending            → muted
✓ completed          → tool + dim
… 还有 N 项             → tool + dim
```

## 点击与滚动

- 整条摘要行可点：切换 `expanded`，清空 `todo_scroll`。
- 已展开时滚轮在栏内滚动，不改 `expanded`。
- 收起时滚轮不作用。
- 行内仍可划选复制（沿用 `todo_selection`）。

## 命中与固定区

- 待办栏仍位于队列条之后、通知条之前；空表占 0 行。
- 行数计入 Fixed-Zone Footprint：收起 1 行，展开 `min(6, ⌊term_rows/4⌋ 与 item 数决定)`。
- 移除 `↑ N more` / `↓ N more` 边框行，因此固定区在溢出时少 2 行。

## 影响面

- `crates/xylitol-app-tui/src/widgets/todo_bar.rs`：重写布局与取色。
- `crates/xylitol-app-tui/src/widgets/fold_hit.rs`：删 `FoldTarget::Todo*`。
- `crates/xylitol-app-tui/src/layout/root/mod.rs`：`todo_doing_open` 等替换为 `todo_expanded`；`toggle_todo_fold` 改为摘要行开关。
- `crates/xylitol-app-tui/src/layout/root/render.rs`：`render_todo_bar_slot` 传新参数。
- `crates/xylitol-app-tui/src/layout/root/slot_input.rs`：点击分流改为摘要行切换。
- `crates/xylitol-app-tui/src/widgets/glyphs.rs`：新增 `todo_doing` / `todo_pending` / `todo_done` 三个字形入口与 ASCII 回落。
