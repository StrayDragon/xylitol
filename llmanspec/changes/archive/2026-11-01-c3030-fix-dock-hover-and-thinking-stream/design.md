# Design — c3030

## 1. 为什么改引擎而不是在应用端算坐标

悬停高亮由引擎的 `apply_hover_tint` 在 `paint_visible` 内完成，输入是命中的 `HitRegion`。原来它只处理 transcript 空间：`HitSpace::Dock` 被 filter 掉，`visible_row_for` 对 Dock 返回 None。应用端若自己重绘会被下一帧盖掉，所以修在引擎。

**关键决定**：把「命中区落在哪一行」抽成 `HoverRowSource`，而不是给 Dock 单独写一套坐标算术。三种结果——Transcript(i) / Dock(i) / Offscreen——把空间判别收敛到一处，`apply_hover_tint` 只负责按结果取切片。

### Chrome 越界的处理

队列条注册的是 `HitSpace::Chrome` 加屏幕绝对行（历史原因：它原本只为点击捕获）。Chrome 的 content_row 在 transcript 内按屏幕行解释，越界则落到 dock。这条规则同时满足既有 transcript 内 Chrome 行为与队列条高亮，不需要改队列条的注册方式。

## 2. 为什么待办栏 hover 用 Dock 而队列条用 Chrome

待办栏是新登记的 hover 区，没有历史包袱，直接用 `Hit::at_screen` + `HitSpace::Dock`，语义最直白。队列条保留 Chrome 注册（点击捕获仍依赖它），靠第 1 节的越界规则获得高亮。两者最终落到同一 dock 行映射。

## 3. chip 行如何取到 thinking 档位色

编辑区边框色由 `apply_thinking_border` 写进 `Editor::set_border_color` 的闭包，chip 行在 `append_border_chips` 里独立绘制，拿不到那个闭包。

**决定**：在 `UiRoot` 上开 `operation_zone_paint()`，按 `bash_mode` 与 `thinking_border_level()` 返回同源 painter，编辑区与 chip 行都从它取色。这样只有一处判断档位，bash 强调色的覆盖优先级也自然一致。

激活态不复用该 painter：反色语义是 accent 加粗，与档位色是不同维度，混在一处会让「激活」与「高档位」看起来一样。

## 4. 待办栏空白行为什么计入占位

待办栏行数由 `layout_todo_bar` 产出、`apply_fixed_zone_footprint` 计入下缘预留。空白行若不计入，固定区会少算一行，editor 与 status 就上移一格压到内容。所以空白行做成 layout 的第一行，命中行号随之下移，行上限仍是 `min(6, term_rows/4)` 覆盖整栏（含空白行）。

摘要命中行不再写死 0：`summary_hit.row` 由 `SPACER_ROWS` 决定，测试与 `click_todo_bar_local` 都从这里取值，避免再次出现「点击落在空白行」。

## 5. provider 侧为什么在适配器而不是端上兜

摘要通道丢的是流式增量，`output_item.done` 仍会整块补一次 `ThinkingEnd`。若在端上做「等 done 再显示」，思考块会退化成结束后才出现，与 transcript 的流式默认展开契约冲突。修在 `map_responses_sse_event` 是唯一能让正文在流式期间可见的位置。

两条通道都映射为 `ThinkingDelta`，不区分来源：端上只需要知道「思考通道还在继续」，摘要与原始推理对它等价。

## 6. 返工：悬停解析与滚动锚点

首轮只把 dock hover 区加进 `replace_hover_hits`，但悬停 id 解析走的是点击表 `regions`，固定区元素不在其中，所以永远悬不到。真机回报后改为：先查 hover 表，未命中再回落点击表。回落必不可少——折叠块正文的悬停区只在左侧轨道列，点击表才是全宽的。

染色残留是第二个缺陷：`emphasize_hover_columns` 不幂等，同一行被重复施加会叠加 SGR。改为已染色行直接返回，并补「移出后逐字节还原」的断言。

滚动跳动定位在历史前置页：`prepend_history` 重建滚动区后没有补偿 `scroll_top`，锚点行被顶离原位。修法是记录前置前首个可视行的纯文本，前置重绘后按该文本重新定位，再重绘一次。为什么按文本而不是按行号：前置页的行数取决于折叠渲染，无法预先算出偏移。

## 7. 风险与取舍

- 若上游同时发两条通道，思考正文会拼接两遍。实测网关是二选一（见 `research/upstream-reasoning-channels.md`），且 `ThinkingEnd` 会用最终 item 覆盖 `thinking_acc`，最坏情况只在流式期间短暂重复。
- 空白行让待办栏在 6 行上限下少显示 1 条条目。已确认这是版式优先的取舍。
