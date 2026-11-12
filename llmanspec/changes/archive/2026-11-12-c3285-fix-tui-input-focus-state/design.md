# Design — TUI 引擎指针 / 选区 / 焦点状态（2026-11-13 复核后定稿）

## 1. 目标与边界

- 指针捕获闩丢失 Up 可回收；选区状态机单一实现；overlay 焦点单一真值；ChoicePrompt Review 态显式；gutter 拖拽可复位。
- 边界：不改 hit-testing 管线与 AO 模式语义；不改选区坐标空间（引擎 u16 屏幕格 / editor usize 行内偏移）。

## 2. 决策摘要（2026-11-13 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| 指针相位 | 不新建单枚举 PointerPhase：点击策略 PointerClickPhase（app 每帧 set_phase，外部）与捕获闩 capture: Option<Instant>（内部手势）正交两轴；capture 超 750ms 遇新 Down 回收；set_phase 变更清 pending_click；Down 重置 dragged/down_cell，Up 清二者 | 定稿（修正：单枚举会要求 app 驱动捕获态并改点击吞噬语义） |
| 选区合一 | 抽取 SelectionCore（anchor/focus/dragging/granularity/click_count/last_click 单一状态机；点击计数以 usize 行列为准，引擎 u16 转换）；引擎 SelectionController 与 editor EditorSelection 均持有 core（editor 为薄包装：usize 坐标 + LayoutLine 词/行展开）；dock clamp 与 auto-scroll 仍为引擎配置 | 定稿（Open Q1 取「editor 薄包装」：坐标空间不同，强行并入引擎需坐标抽象且无行为收益） |
| 焦点合一 | FocusState { root: Option<usize>, overlay: Option<u64>, restore: OverlayFocusRestore } 单字段；dispatch 的对账改为 FocusState 单一方法（保留入口，消除散写） | 定稿（复核：hidden 不可聚焦已由 focus_overlay/hide/set_hidden 过滤，缺显式不变量测试） |
| ChoicePrompt | ChoiceTab { Question(usize), Review } 枚举；states 索引只取 Question；消灭 tab==len 越界编码 | 定稿 |
| overlay 可见性 | 显式不变量：focus_overlay 对 hidden 直接返回（已有）+ 单测；last_screen_rect 不引入帧号（复核：rect 仅在渲染时写入，未渲染为 None；屏上矩形即命中真值） | 定稿（修正：帧号校验无必要） |
| gutter 拖拽 | 新 Down 先清 gutter_drag_grab；拖动中指针移出 gutter 列即清 grab | 定稿 |

## 3. 指针相位机

点击策略：Idle → DownSwallow（Down 命中）→ pending_click；Idle → UpIfEmptySelection（Up 空选区）→ pending_click。捕获闩：Down 命中 overlay/region → capture=Some(now)；Up → capture=None；新 Down 且 now-since ≥ 750ms → 先回收再按未捕获处理（防丢 Up 永久吞点击）。drag 只置 dragged；Up 清 down_cell+dragged。

## 4. 选区结构

SelectionCore 方法：clear / is_dragging / has_selection / bounds / update_click_count。引擎 controller 持 core + auto_scroll/hit_priority/copy_on_release；editor 持 core + 行内展开（expand_editor_range）。两处点击计数语义（400ms 同格）只在 core 实现一份（usize 行列；引擎在 update_click_count 转换）。

## 5. 测试边界（seam）

- 单测：丢 Up 注入（新 Down 回收）；set_phase 清 pending_click；down_cell/dragged 生命周期；SelectionCore 点击计数与 bounds；editor/引擎选区行为对照；FocusState 转移矩阵（含 hidden 不可聚焦）；ChoiceTab Review 下标安全；gutter 新 Down 复位。
- BDD：app-tui-mouse-actions / app-tui-ask 回归。

## 6. 风险与回滚

- 选区是行为面最大片（本次只做状态机合一，坐标/展开不动）；四片（指针/选区/焦点/choice）独立回滚。

## 7. 实施前复核（2026-11-13）

| 结论 | 复核 | 证据 |
|---|---|---|
| 指针 8 字段，capture_latch 仅 Up 清、无重入/超时 | 成立 | pointer.rs:291-303；439-538（Down 在 latch 时直接 return） |
| set_phase 不清 pending_click | 成立 | pointer.rs:337-339 |
| 选区双实现分叉 | 成立 | selection.rs:98-112（引擎：auto_scroll/hit_priority）；editor/selection.rs:8-16（editor：无 auto_scroll，last_click_cell 为 usize） |
| overlay 焦点三字段 + dispatch 对账 | 成立 | overlay.rs:182-202、294-381；tui/mod.rs:980-1022（dispatch 内再聚焦/恢复） |
| hidden 可聚焦 | **部分不成立**：focus_overlay / hide_overlay / set_overlay_hidden / restore 过滤均已排除 hidden；缺显式不变量测试 | overlay.rs:448-454、169-171、219-234、402-428 |
| last_screen_rect 旧矩形 | **不成立为缺陷**：rect 仅渲染时写入（overlay.rs:628），未渲染为 None；屏上矩形即命中真值 | overlay.rs:443-446；tui/mod.rs:527-535 |
| ChoicePrompt tab==len 编码 Review | 成立 | choice_prompt.rs:218、292-298、350、367、378-381、447-451 |
| gutter 拖拽仅 Up 清 | 成立 | application_owned_runtime.rs:299-341 |
