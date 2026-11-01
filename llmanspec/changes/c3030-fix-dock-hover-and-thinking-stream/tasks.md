# Tasks — c3030

## 1. 引擎：dock 行悬停

- [x] 1.1 `pointer.rs` 新增 `HoverRowSource` 与 `hover_row_source`，把命中区按空间与 screen_row 分流到 transcript 行 / dock 行 / offscreen。
- [x] 1.2 `apply_hover_tint` 改为接收 transcript 与 dock 两个切片加 `PointerView`，去掉原先过滤 Dock 的分支。
- [x] 1.3 `application_owned_runtime.rs` 的 `paint_visible` 在拼接 dock 行之前对副本施加 tint。
- [x] 1.4 单测：dock 命中高亮 dock 行、双向不串色、终端高度变化后映射正确、越界 Chrome 落入 dock。

## 2. 产品端：固定区悬停命中

- [x] 2.1 `UiRoot::overlay_hover_hits` 登记队列条与待办栏摘要行 hover 区，均不捕获指针。
- [x] 2.2 待办栏摘要行的 dock 绝对行由 `last_todo_hits[0].row`（含新增空白分隔行）推导。
- [x] 2.3 `HostSession::sync_pointer_hits` 把上述 hover 区并入 `replace_hover_hits`。
- [x] 2.4 单测：dock 同时含队列与待办时两类 hover 区都在、空间为 Dock、不捕获；空 dock 不登记。

## 3. 产品端：chip 行取色

- [x] 3.1 新增 `UiRoot::operation_zone_paint()`，统一 bash 强调色与 thinking 档位边框色。
- [x] 3.2 `append_border_chips` 的分隔虚线与未激活 chip 改走该 painter；激活 chip 保持 accent 反色。
- [x] 3.3 BDD mat33 断言改为对照同行的边框色与 chip 色，并断言 chip 色不等于 muted。

## 4. 产品端：待办栏上方空白行

- [x] 4.1 `layout_todo_bar` 在摘要行前插入 `TODO_BAR_SPACER_ROWS` 行空白；摘要命中行号随之下移。
- [x] 4.2 可见行总数仍受 `min(6, term_rows/4)` 上限约束，空白行计入占位。
- [x] 4.3 单测：空表 0 行；收起为 1 行摘要加 1 行空白；展开仍守行上限，且首行为空白。

## 5. Provider：推理摘要通道

- [x] 5.1 `map_responses_sse_event` 同时接受 `response.reasoning_text.delta` 与 `response.reasoning_summary_text.delta`。
- [x] 5.2 单测：两条通道各自映射为 `XyChunk::ThinkingDelta`。
- [x] 5.3 真实网关验证：xhigh 档流式出现 `Thinking`，结束后变 `Thought` 且正文可见。

## 6. 验收

- [x] 6.1 `just qa quiet` 全绿。
- [x] 6.2 `llman-sdd validate --all` 60/60 通过。
- [ ] 6.3 人工真终端观感确认（悬停高亮、chip 变色、待办空白行）。
