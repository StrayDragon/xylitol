# 调研：Todo checklist 迁入下缘固定区

一手对照（代码 / live specs / 设计稿）。结论供本 change 的 design 用，不是 live 合约。

## 现状（对话区 latest-wins 行）

`TodoUpdated` 成功写入后，bridge 删掉全部 `UiEntry::Todo`，非空则 **append 到 transcript 末尾**（`sync_todo_checklist`）。resume / travel 走同一函数，数据来自 leaf 上 latest `agent_todo` 快照。空表 = 清除行。

画法：`paint_todo_block` 默认一行 `Todo · N/M` + `(Alt+E)`；`fold.todo_expanded` 展开字形清单。三角命中 `FoldTarget::Todo`（全局一处，无 per-id）。`app.tools.blocks`（默认 Alt+E）**同时**翻转 tools / compaction / todo 三套展开。

Activity 折叠把该行标成 `ActivityAtom::Projection`：不计入 Used N，但仍进信封/簇（att23/att34 点名 Todo）。因此清单会随对话滚动、被旧轮信封收走——这正是提案要修的可用性。

`todo_*` 工具块（时间线 Used）与投影行分离：块 body 与投影共用字形 helper（att36）。本 change **不得**动工具块。

## 下缘输入坞硬约束（不可把待办栏塞进 status 与 editor 之间）

`UiRoot::render` 下缘顺序（代码注释 + 实现）：

```text
scrollback → 队列条 → 通知条 → status → editor → footer
```

- **status 紧贴 editor 上方**（r1237 / r1217）：busy 前导空行+spinner；idle 一行呼吸空白。
- **通知条恰好在 status 上方一行**（r1227）。
- 队列条在 transcript 与 status 之间（designing `queue-steer`：`scrollback → 队列条 → 通知条 → status`）。

因此待办栏只能插在 **队列条与通知条之间**，或 **队列条之上**（仍在 scrollback 下）。不可插在通知条↔status 或 status↔editor。

已定案：

```text
scrollback → 队列条 → 待办栏 → 通知条 → status → editor → footer
```

队列条仍是当轮改向；待办栏是会话常驻态；通知条仍贴 status（瞬时诊断不被展开清单顶走）。

`reserved_lower_fixed_zone`（r1228）今日只计 queue + 通知条 + status + footer。待办栏行数 MUST 进该预算，否则短终端 Resume/Tree 槽会把 status 挤出视口。r1231 禁止未写入视觉 SSOT 的固定区成员 → 先 `designing/tui-lab/modules/todo-bar` 定案再晋级 `tui/modules/todo-bar`，并改 activity-fold 的 `todo-bar:` 对齐句（今日写「簇内 Todo 行」）。

## 信息流（本波不动）

| 信息平面 | 真值 | 本波 |
|---|---|---|
| LLM body | 工具 JSON | 不变（c2805 另案） |
| SSOT | `agent_todo` latest-wins | 不变 |
| live | `XyEvent::TodoUpdated` | 仍消费；改写入点（输入坞待办栏模型，不再 `UiEntry`） |
| resume | 快照 rebuild | 仍同源；写入待办栏 |
| 工具块 | att36 清单 body | 保留 |

与 `c2805-update-todo-llm-api` **无 depends_on**：c2805 改工具原语/条目结构；本波只搬家。条目字段若日后扩展，待办栏只读展示。

跨端：gpui 未开放。约束板只要求「常驻任务清单、非对话历史」语义同源；本波不写 gpui MUST、不预埋第二套动作 id。

## 折叠键与 SOP

迁出 transcript 后，Alt+E 再驱动输入坞待办栏会与「块级展开」语义缠在一起（tools+compaction+todo 同和弦）。建议待办栏用三角点击 + 独立展开态；**不再**挂 `app.tools.blocks`。

`designing/` SOP：未知交互进 tui-lab。三区两翼是新交互语言 → **先 tui-lab 定案再晋级**产品模块。

## 测试 seam（复用，不发明）

| 观察点 | 既有入口 |
|---|---|
| live 事件 → 投影 | `bridge/tests.rs` + `apply_xy_event(TodoUpdated)` |
| resume/travel 重建 | `rebuild_scrollback_from_travel`；BDD 今日断言 `UiEntry::Todo`，须改为断言待办栏 |
| 工具块 body | att36 BDD（`todo-block-body-renders-checklist`）**保留** |
| 输入坞堆叠 / footprint | `layout/fixed_zone_footprint.rs` 单测 + `layout_*` harness |
| 空表消失 | `bridge/tests.rs` empty `TodoUpdated` |
| Activity 不再收纳投影行 | `activity_fold` atom/scene；att23/att34 去 Todo 主语 |
| 设计帧 | `just export-design-frame` + `check_tui_designing` |

`agent-todo` 十三条规则目前 **0 条 @executable**。本波至少给改写后的 r1129/r1130 补 SceneBuilder/HostSession 验收。

## 合约改写面

- `agent-todo` r1129：主清单从对话区改为下缘待办栏；仍禁止 Plan 侧栏 / status / footer / 通知条 / ScrollNotice 冒充主清单。
- `agent-todo` r1130：resume 重建目标改为待办栏。
- `agent-todo` r1127：只读 SSOT 边界保留；待办栏不是 status。
- `app-tui-fixed-zone`：r21 新成员 + r1228 预算 + r1231 文档化槽。
- `app-tui-transcript` r1353/r1365：信封/簇不再收纳待办栏投影（工具块仍在）。

## 新问题（固定区特有）

对话区里长清单可随 scrollback 滚。进输入坞后展开会吃终端行高、挤压 transcript 与槽 body。

## 决策记录（2026-09-20）

展开高度问：用户否决「一行 `Todo · N/M` + 一次展开全表」和 overlay。意向：

- 默认展开三区只读列表（看板式看到列内条目）；两翼仍可独立折上。高度走
  D12 栏内滚动，不把清单铺成墙。（曾考虑默认只露当前任务，目检后改为展开。）
- 三区结构已确认：前 completed/cancelled、当前 in_progress、后 pending；两翼独立折。
- UI **必须先过 designing**（已晋级 `tui/modules/todo-bar`）。
- 计数文案：禁止无标签 `N/M`，禁止另画 done/todo 行。计数写在翼头：`▸ n completed` / `▸ n pending`。cancelled 不进 completed 计数，只在前翼展开。
- 无 `in_progress`：不假装焦点；只留非空折上翼头。空表仍消失。
- 宽度：≥100 列（≡ Diff side-by-side 阈值）三等分栏，更窄竖叠。
- 条目：写入 content ≤80 Unicode 标量（拒绝不截断）；绘制按栏宽换行，禁止省略号。
- 高度：可见行 ≤ min(6, ⌊term_rows/4⌋)（至少 2），超出栏内滚动。
