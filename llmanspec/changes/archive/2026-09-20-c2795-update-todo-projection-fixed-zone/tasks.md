# Tasks — 待办栏迁入下缘固定区

垂直切片。接缝见 design.md，全部复用既有入口。T1 人类目检前 **禁止** 改产品输入坞。

## T1 tui-lab：三区待办栏固定态

- [x] `designing/tui-lab/modules/todo-bar/`：`draft.yaml`（`surface: tui-lab`）+
      `intent.md` + states（至少：空表消失、仅当前任务、前翼展开、后翼展开、
      两翼展开、无焦点混合、全完成、全 pending；以及 wrap-stack / wrap-wide /
      `*-wide` 三栏对照）
- [x] 计数写在翼头 `n completed` / `n pending`，MUST NOT 写 done/todo 行、无标签 `N/M`
- [x] ≥100 分栏（槽位始终三等分；折叠不改列宽）/ <100 竖叠；content ≤80 按栏宽换行，MUST NOT 省略截断；可见行
      ≤ min(6, term_rows/4)，超出栏内滚动；预览可拖列宽试断点
- [x] `just gen-designing-index`；预览 `/tui-lab/todo-bar/<state>`（晋级后 `/tui/todo-bar/<state>`）
- [x] 验证：人类目检三区/默认露出/无焦点翼头；`check_tui_designing` 不因 lab 红
- 目检通过前 T2 不得开始

## T2 晋级产品模块 + 输入坞槽

- [x] `git mv` lab → `designing/tui/modules/todo-bar/`，`surface: tui`；改
      activity-fold / queue-steer / status 的 `todo-bar:` 对齐句
- [x] 词表：`TUI信息呈现与固定区词汇.md` 增 **待办栏**；分类落点（常驻态，非通知条）
- [x] `UiRoot` 下缘插入待办栏：`队列条 → 待办栏 → 通知条 → status → editor`
- [x] `reserved_lower_fixed_zone` 计入待办栏实际行；补 footprint 单测
- [x] 空表 0 行；有表至少露出非空翼头或当前任务一行
- [x] `just export-design-frame` + `just gen-designing-index`
- [blocked-by: T1]

## T3 投影改写入输入坞，取消对话区 Todo 行

- [x] `TodoUpdated` 与 resume/travel 重建写入待办栏模型，不再
      `entries.push(UiEntry::Todo)`
- [x] 删除产品路径 `UiEntry::Todo` 及 `paint_todo_block` / `FoldTarget::Todo` /
      `fold.todo_expanded` 对对话区的依赖；`app.tools.blocks` 不再翻转待办栏
- [x] 字形 helper 仍与 att36 工具块共用
- [x] 测试：`bridge/tests.rs` empty 清除、live 刷新改断言待办栏；
      `scrollback/tests.rs` 不再依赖 transcript 内 Todo 行
- [blocked-by: T2]

## T4 Activity 折叠不再收纳投影

- [x] 信封/簇路径不再把 checklist 当 `Projection` 条目（无该 `UiEntry`）
- [x] 回归：thinking+todo_* 仍为 Used（工具块），checklist 不计入 Used N
- [x] `activity_fold` 既有 scene/atom 单测改夹具
- [blocked-by: T3]

## T5 BDD / harness 验收（已确认 seam）

- [x] att36 `todo-block-body-renders-checklist` **保持绿**（工具块）
- [x] 原断言 `UiEntry::Todo` 的 transcript BDD 改为待办栏：默认展开三区、
      无焦点仍展开非空翼、空表消失、resume 与 live 同源
- [x] 验证：`cargo test --lib --all-features tests::bdd::` 相关 +
      `just test-tui` 布局切片
- [blocked-by: T3, T4]

## T6 门禁闭合

- [x] `just fmt` / `just lint` / 相关 test / `just test-tui`；需要时 `just qa`

## T7 轮廓与 more 边框（本波补需求）

- [x] 有表上下 `─` 轮廓（空表仍 0 行）；内容上限仍 ≤ min(6, term_rows/4)
- [x] 溢出同 editor：`↑ N more` / `↓ N more` 写在边框，不替换正文末字
- [x] 指针在栏上滚轮滚动内容；footprint 计入轮廓
- [x] 固定态 YAML / 预览 / 单测同步

## T8 折叠稳定栏宽

- [x] 宽屏三列槽位始终三等分；折叠只收正文，不改列宽/列起点
- [x] 预览 layout / 宽屏展开固定态 / 单测同步

## T9 默认展开两翼

- [x] 产品默认 `past`/`pending` 展开；两翼仍可独立折上
- [x] r1129 / 固定态 / BDD：默认展开；doing|pending|completed；doing 无 `[~]`；栏内划选复制
