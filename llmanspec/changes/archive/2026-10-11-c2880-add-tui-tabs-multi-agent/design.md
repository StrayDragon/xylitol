# Design — 包层 Tabs

## 范围

本票只加 `packages/xylitol-tui` 的 `Tabs`。产品 host、`src/app/tui`、Down 重绑、子 agent runtime 都不改。

## 决策

| id | 内容 |
|---|---|
| D1 | 只做刀 A。刀 B（产品 Down → picker / 子面）park。 |
| D2 | 组件名 `Tabs`。状态是 `titles: Vec<String>` + `selected: usize`。不持 session id。 |
| D3 | 只画标题条。`render` 只输出这一行。MUST NOT 持有或绘制对话条目、槽内容或其它页。产品端按 `selected` 下标自己换内容。 |
| D4 | 实现 `Component`。需要收键时实现 `Focusable`。host / `TUI` 决定谁有焦点。 |
| D5 | `KeybindingsManager` 登记 `tui.tabs.prev`、`tui.tabs.next`。默认键列表为空。产品端或测试写入绑定后，`handle_input` 才改 `selected`。 |
| D6 | 不登记 `tui.tabs.select.1`–`9`，也不把裸 `1`–`9` 写进组件。以后要数字跳转，由产品端另登 id。 |
| D7 | 左键按下按列改 `selected`。列 = 屏上列 − `screen_origin.col`，行必须落在 `screen_origin.row`（条高 1）。点标题 → 该标题下标；点标记 → 该侧第一个隐藏标题下标。未命中、行不在条上、尚未 `render` → 不改下标。不登记 `HitRegion`。 |
| D14 | 滚轮：仅当指针行落在条上时，向上 = 下一项，向下 = 上一项，循环。否则不改下标，外层可滚对话条目。 |
| D8 | 标题条一行。溢出 = **窗口 + 两侧计数**（Q5）。先放 selected（完整宽），再左右轮流加邻居，加不下就停。左侧藏着画 `← +N`，右侧画 `+N →`。隐藏标题不出现在条上，也不进 `title_col_ranges`。标记列不计入标题区间。单条 selected 比 `width` 还长时，只截这一条以落入 `width`。 |
| D12 | `title_col_ranges()` 每段带原下标：`(标题下标, 起始列, 结束列)`，只含画出来的标题。从左到右按画出来的顺序。 |
| D13 | `overflow_col_ranges()`：左、右至多各一段。有隐藏时给出 `(该侧第一个隐藏标题的原下标, 起始列, 结束列, 隐藏数量)`。左侧下标 = 窗口起点 − 1；右侧下标 = 窗口起点 + 可见数。无隐藏的一侧不出现。标记列不进入 `title_col_ranges`。 |
| D9 | `Vec<String>` ANSI。主题用闭包。不用 `StyledLine`，不依赖 ratatui `widgets::Tabs`。 |
| D10 | `Container` 仍只垂直排。host 若要把条和内容叠在一起，自己往 `Container` 里放 `Tabs` 再放内容。 |
| D11 | 选中变化可回调 `on_change: Option<Box<dyn FnMut(usize)>>`。crate / BDD 用它断言，不是产品事件。 |
| D15 | 库做指针命中。`Component` 增加默认空操作的 `set_screen_origin` / `offer_pointer(event) -> bool`。`Tabs` 实现：减 origin、对照两类区间、改 `selected`、改了则 `input_wants_rerender` 为 true。Moved 返回 false。 |
| D16 | `TUI::dispatch_event` 在 listener 之后、焦点路由之前，对根 child（及可见 overlay）调用 `offer_pointer`。第一个 true 消费该指针，不再送给焦点 editor。键盘路由不变。 |
| D17 | `Container::handle_input` 仍不转发键盘（r1614）。`Container::offer_pointer` 按上次 `render` 各 child 占用的行，把屏上事件转给命中的 child（child origin.row = 容器 origin.row + 行偏移）。 |

## 公开形状

```text
Tabs::new(titles: Vec<String>)
  .with_theme(TabsTheme)
  .with_on_change(f)
  .set_selected(i) / selected() / set_titles(...)
  .set_screen_origin(col, row)                 // 条左上角屏上格；默认 (0, 0)
  .offer_pointer(event) -> bool                // 与 handle_input 同一套命中；true = 已改 selected
  .title_col_ranges() -> Vec<(usize, usize, usize)>
      // (标题下标, 起始列, 结束列)；条内列 [start, end)
  .overflow_col_ranges() -> Vec<(usize, usize, usize, usize)>
      // (该侧第一个隐藏标题下标, 起始列, 结束列, 隐藏数量)
```

无 `with_pages`。

`set_titles` 之后若 `selected >= titles.len()`，夹到 `len.saturating_sub(1)`（空表则 0）。

空 `titles`：`render` 返回空 `Vec`，两类区间表都空。

## 布局

一行。分隔空格不计入任一 `title_col_ranges`。

溢出（D8）：

1. 量每个 title 的自然显示列。
2. 预算 = `width`。若左侧将有隐藏，先扣 `← +N` 宽；右侧同理扣 `+N →`（N 随窗口变，最多迭代几次到稳定，对齐 opencode `solve` 3 次）。
3. 窗口必须含 `selected`。左右轮流塞完整 title，塞不下的整条隐藏。
4. 输出：`[← +L][可见 titles 以空格分隔][+R →]`。
5. `title_col_ranges` 每项是 `(原 titles 下标, 起始列, 结束列)`。分隔不进区间。隐藏标题没有项。
6. 有左侧隐藏则 `overflow_col_ranges` 含左标记，下标为窗口内第一项的前一个标题；右侧同理，下标为窗口内最后一项的后一个标题。

## 输入

`handle_input` 与 `offer_pointer` 看 `InputEvent::Key`、左键按下、滚轮。指针用屏上坐标减 origin。

- 当前生效绑定匹配 `tui.tabs.prev`：`selected` 循环到前一项
- 匹配 `tui.tabs.next`：循环到后一项
- 左键按下：用事件列对照上次 `render` 的两类区间，命中则改 `selected`
- 滚轮向上：循环到下一项；向下：循环到上一项
- 未绑定、空表、未命中、其它键、尚未 `render`：`NotHandled`

切成功：`invalidate`，调用 `on_change`，返回 `Handled`。

产品端以后对照关系（不进本票 API）：

| `selected` | 端换什么（候选，未交付） |
|---|---|
| 下标 i | 会话书签 i 的对话条目 |
| 以后 | 子 agent 端（另建隔离 runtime） |
| 以后另开 change | 标题左右分列、整条隐藏 |

包测试用 `set_user_bindings` 临时绑测试键，不断言 Left、Right、1–9。

## 指针：库做命中，底层仍开放（Q12）

调用方默认路径：

1. 布局后写下 `set_screen_origin`（条在第几屏行列）。根 child 从 (0, 0) 起画时可以不写。
2. 走 `TUI::dispatch_event`。引擎在 listener 之后、焦点之前调用 `offer_pointer`。点在条上则标题条吃掉，editor 收不到这次 Down / 这次条上滚轮。
3. `Container` 里把 `Tabs` 放在第一块时：容器按上次渲染的行偏移转给 child，调用方不必对 Tabs 再写一遍 origin。

底层仍可用：`title_col_ranges` / `overflow_col_ranges`、`set_selected`、条内列的 `handle_input`、自己 `mark_hit`。标题条不分配 `HitId`。

不在本票做：按任意浮动矩形做完整命中树；改键盘焦点路由。

## 主题

`TabsTheme` 最少：`title_style(selected: bool)`。分隔符样式可选。不写死产品色。

## 测试缝

复用包 `Component::handle_input` / `render`，以及 `tests/bdd` 的 `bindings_package_tui_*`。不新开 harness。不测产品 host。

## 规格映射

| req | 行为 |
|---|---|
| r1938 | 已绑定的上一标题 / 下一标题键改 `selected`；未绑定则方向键不改下标 |
| r1939 | 只输出标题条，不持有 page |
| r1940 | `render` 后可见标题的列区间，每段带原下标 |
| r1941 | 溢出窗口：当前标题完整可见；隐藏的不画；两侧计数 |
| r1942 | 溢出标记的列区间 + 该侧第一个隐藏标题下标 |
| r1943 | 左键按下按条内列改 `selected`；不登记命中区 |
| r1944 | 指针在条上时滚轮切相邻项 |
| r1945 | `set_screen_origin` + `offer_pointer`；屏上坐标减 origin |
| r1946 | `dispatch_event` 焦点前投 `offer_pointer`；`Container` 按行转给 child；键盘仍不 fan-out |

## 明确不做

- page 子组件 / notebook
- 标题条自己 `mark_hit`（调用方仍可用命中区）
- 多行标题条、垂直 tab
- 产品 host 接线、跨端同源新词
- 子 agent 编排
- 标题左右分列、整条隐藏（Q7）
