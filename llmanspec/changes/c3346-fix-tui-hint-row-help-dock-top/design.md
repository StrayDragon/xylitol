# Design

## 关键事实（已对照代码）

- hint 行现条件渲染：`hint_visible()` = help 开 || (editor 槽 && editor 空)，`hint_rows()` 0/1；渲染在 footer 之后（屏上最底行）。行数记账点：`editor_top_row`、`base_dock`、`last_dock_rows`、chip 行期望值、`reserved_lower_fixed_zone(hint_present)`。
- dock 渲染顺序（现状）：`queue → todo → toast → status(spinner) → tray → help → editor → footer → hint`；帧自上而下 = 滚动区 + 固定区，固定区底部锚定。绝对行位 = `term_rows − dock 总行数 + dock 内序号`：面板开合使 dock 总行数 +h，插入点上方行（含 spinner）上移 h，插入点下方行不动。
- spec 既有钉点：`固定区槽顺序` 场景断言 help 关闭时的队列条→待办栏→通知条→status→editor 顺序（help 最顶不冲突）；待办栏/通知条邻接条款（通知条紧贴 status、status 紧贴 editor）不受影响。

## 实现设计

### t1 提示行恒占位

- `hint_rows()` 恒返回 1（`hint_visible()` 保留，仅驱动文案）；`render_hint_slot` 不可见时返回单个空白行（无 ANSI）。
- `reserved_lower_fixed_zone` 去掉 `hint_present` 形参（恒 +1）；两个调用点与该函数单测同步。
- 既有文案断言更新：非空 editor 时断言「整帧行数不变 + 末行为空白」替代「不含 ? shortcuts」；空 editor / help 开两态文案断言不变。

### t2 帮助框移固定区顶

- 装配改为 `help → queue → todo → toast → status → tray → editor → footer → hint`：
  - `lines.extend(help)` 先行；`queue_strip_row` / `tray_strip_row` 锚点在 extend 后取 `lines.len()`，自动右移。
  - `base_dock` / `max_help_rows` / `editor_top_row(dock_prefix)` 均为行数口径，与顺序无关，不改公式。
  - `help_viewport_rows()` 基于 `last_dock_rows / last_help_rows` 总量，顺序无关。
- 帧断言（harness/单测）：busy 帧下开合 help，前后 status、tray、editor、footer、hint 的绝对行号逐一相等；hint 占位行恒存在。
- BDD helper 同步：`footer_line`（fixed-zone 与 host 两处）在 hint 恒占位后末行恒为 hint 行——helper 收敛为恒取倒数第二行。

### 风险

- 固定区恒 +1 行：极小终端（`MIN_ROWS`=6 下限）滚动区再少 1 行，可接受（行为恒定优先）。
- 帮助框自身高度计算（`help_viewport_rows`）依赖上一帧 dock 总量，顺序迁移不影响其口径；实施后以帧断言回归。
