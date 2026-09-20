# Design — 待办栏迁入下缘固定区

## 范围

只搬家 **client 展示** 的 checklist 投影。SSOT / 工具原语 / `TodoUpdated` / 工具块
人话化不动。与 `c2805-update-todo-llm-api` 无依赖。

## 决策

### D1 下缘堆叠（被既有条款锁死）

`status` 紧贴 editor（r1237/r1217）；通知条恰好在 status 上（r1227）。待办栏只能
插在队列条与通知条之间（或队列条之上）。

采用：

```text
scrollback → 队列条 → 待办栏 → 通知条 → status → editor → footer
```

队列条仍是当轮改向；待办栏是会话常驻态；通知条不被展开翼顶走。

### D2 三区是视图，不是第二份真源

按 status 分组：**当前 doing**（全部 in_progress）、**后翼** pending、
**前翼** completed∪cancelled。区内顺序 = SSOT 表序。禁止为展示重排或另存一份列表。

### D3 默认露出与三列折叠

有条目：默认展开三区；doing / pending / completed 均可独立折上。
doing 条目用正常 on-surface 正文，不用 `[~]`。
无 `in_progress`：不假装焦点；仍画 `0 doing` 翼头。
空表：整栏消失（与今日 empty `TodoUpdated` 清除语义同构）。
零计数翼仍占翼头。

### D4 计数文案

禁止无标签 `Todo · N/M`，也禁止另画 `done`/`todo` 行。计数 MUST 写在翼头
（`▾ n doing` / `▾ n pending` / `▾ n completed`，零计数仍画 `0 …`）。折叠控件与间距以 tui-lab 固定态为准，产品
晋级后的 `designing/tui/modules/todo-bar` 为视觉 SSOT（r1234）。

### D5 先 lab 再产品

未知交互走 tui-lab SOP。本波三区两翼是新交互语言 → **T1 只交 tui-lab 固定态**，
人类目检后再晋级产品模块并接线输入坞（dock）。禁止在 lab 未定案时改产品 `UiRoot` 堆叠。

### D6 键与 Activity

待办栏 **不再** 挂 `app.tools.blocks`（Alt+E 今日连动 tools/compaction/todo）。
翼的展开命中在 lab 定（三角列，对齐 att22 精神：点正文不误切）。
投影行离开 `UiEntry` 后，信封/簇不再收纳它（改 r1353/r1365）；`todo_*` 工具块仍
走 Used，计入簇。

### D7 投影存放

live：`TodoUpdated` → 输入坞上的待办栏模型（非 `UiEntry::Todo`）。
resume：leaf `agent_todo` 快照写入同一模型。取消对话区 `UiEntry::Todo` 变体
（产品路径无调用后再删类型；过渡期保留 match 臂会编译失败则同期删）。

### D8 跨端

gpui 未开放：只保留「常驻任务清单、非对话历史」语义。本波不写 gpui MUST、
不预埋第二套动作 id。

### D9 Footprint

`reserved_lower_fixed_zone` 计入待办栏 **实际行**（默认：非空折上翼头 + 可选当前行；
展开翼按可见行；换行续行计入）。短终端槽 body 预算必须跟着缩，否则 status 会被挤出视口（r1228）。

### D10 宽度响应（三栏 / 竖叠）

断点 ≡ DESIGN `spacing.diff-side-by-side-min-cols`（100），与 Diff side-by-side 同源，
不另发明阈值。≥100：分栏（doing | pending | completed），顶对齐，高度取最高栏。三列槽位始终三等分（余数列给末栏）；折叠只收该列正文，不改列宽或列起点，避免邻列换行重排。
<100：竖叠 doing → pending → completed。默认展开三区；超出仍走 D12 栏内滚动。零计数翼仍画 `0 …` 翼头。

### D11 条目字数与换行

content trim 后 Unicode 标量 ≤ 80（r1842）：写入拒绝、**不**截断入库。绘制按当前栏
cell 宽换行（优先空格；CJK 按字），悬挂 4 cell；**禁止**省略号截断。已持久超长快照
resume 仍换行绘制，不丢表。80 是写入上限不是显示上限：栏变窄时 80 标量可占多行。

### D12 高度上限

待办栏**内容**可见行 MUST ≤ `min(6, ⌊term_rows/4⌋)`（至少 2）。6 是硬顶，避免宽屏把清单铺成墙；短终端再按 1/4 行高收。超出在**栏内滚动**（指针在栏上滚轮），翼头计数仍为全表；禁止用省略号吃正文。有表时上下 `─` 轮廓（与 editor 操作区同形）；未显示行改写边框为 `↑ N more` / `↓ N more`，禁止替换末字。轮廓计入 footprint，不计入内容上限。展开两翼仍可能触发滚动，这是有意的：editor / status / 对话区优先。

## 测试 seam（已与用户确认）

1. tui-lab/todo-bar 固定态（首刀）
2. UiRoot 输入坞 + `reserved_lower_fixed_zone`
3. bridge：`TodoUpdated` / resume → 待办栏模型，不再 append `UiEntry::Todo`
4. att36 工具块 BDD 保留；checklist 断言从 entries 改到待办栏
5. activity_fold：投影不再进信封/簇

不发明新 harness seam。

## 风险

- 展开两翼仍可能很高：用 D12 可见行上限 + 栏内滚动，不省略正文。
- 改 r1129 等已锁定 `@human` 只出 WARNING，不阻断 validate；`change diff` 会浮现。
