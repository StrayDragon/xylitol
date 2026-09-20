# todo-bar（文案草稿）

下缘待办栏。三区是视图，不改 SSOT 顺序。空表不渲染。

## 宽度（已定案）

断点 ≡ DESIGN `spacing.diff-side-by-side-min-cols`（100）。<100 竖叠 doing→pending→completed；≥100 分栏（doing|pending|completed，顶对齐，高度取最高栏）。三列槽位始终三等分；折叠只收该列正文，不改列宽或列起点。默认展开三区。零计数翼仍画 `0 doing` / `0 pending` / `0 completed` 翼头，空表才整栏消失。

## 计数（已定案）

写在翼头：`▾ n doing` / `▾ n pending` / `▾ n completed`（折上 `▸`；有 cancelled 则 `n completed · k cancelled`；n 可为 0）。doing 条目不用 `[~]`，用正常正文。空表才不画栏。不用 done/todo 行。

## 字数、换行、高度（已定案）

content trim 后 Unicode 标量 ≤ 80；写入拒绝、不截断入库；旧快照仍可换行绘制。按栏宽换行（空格优先；CJK 按字），悬挂 4 cell；禁止 `…` 吃正文。

内容可见行 ≤ min(6, ⌊term_rows/4⌋)，至少 2。超出在栏内滚动（指针在栏上滚轮）；翼头计数仍为全表。有表时上下 `─` 轮廓（与 editor 同形）；未显示行写在边框 `↑ N more` / `↓ N more`，禁止替换末字。空表无轮廓。Footprint 按可见行（内容视口 + 轮廓）计。

## 默认（有 in_progress）

竖叠与宽屏均展开 doing / pending / completed（超出栏内滚动）。无焦点：不画 doing 列条目，非空翼仍展开。栏内可划选复制。
