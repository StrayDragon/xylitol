# Design

## Context

侧栏与覆盖弹出框由 `layout/root/render.rs::overlay_viewport` 直接画进 `lines`，不经过 `xylitol-tui` 的 overlay 组件注册；因此 `PointerPipeline` 看不到它们的矩形。现有 `HitSpace` 只有 `Content` / `Viewport`，捕获区在 mouse-up 时不会产生 `pending_click`，适合“组件自己处理点击”的 overlay，不适合产品端自绘 chrome。

## Decision

### A. Chrome 命中空间

- `HitSpace` 增加 `Chrome`。
- `Hit::at_cell(...).space(HitSpace::Chrome)`（或等价 builder）把命中区标记为 chrome。
- `PointerPipeline::on_mouse` 在 mouse-up 时：
  - 若本次按下命中 chrome 捕获区且未拖拽 → 写入 `pending_click`，由 host 派发。
  - 其它捕获区（组件 overlay）保持现状，不产生 `pending_click`。
- chrome 命中区同时 `capturing()`：按下即 `skip_selection`，选区不穿透。

### B. 侧栏宽度档

- 档位 `SESSION_SIDEBAR_WIDTHS = [24, 32, 40]`，默认 24。
- `session_sidebar_layout(width)` 取当前档位，但保证 `content_w >= 40`；否则回退到能放下的最大档位。
- `session_sidebar_width_cycle()` 在档位间循环；关闭侧栏时保留档位（会话内记忆）。
- 键位 `app.session.sidebarWidth`（默认 `alt+shift+u`）；点击分隔条（内容区右缘）也循环档位。

### C. 三态循环

- 新增键位 `app.session.detailsCycle`（默认 `alt+o`）。
- 状态机：
  - 无 → `request_session_popup()`
  - 仅弹出框 → 关闭弹出框，`request_session_sidebar()`；窄档不可用时 effect 回落为弹出框
  - 仅侧栏 → `close_session_sidebar()`
  - 两者同时开（经独立键位）→ 关闭两者
- 不改变既有 `app.session.popup` / `app.session.sidebar` 的独立切换语义。

### D. 侧栏滚动与滚轮分流

- `UiRoot` 增加 `session_sidebar_scroll`；`scroll_session_sidebar(delta, visible_rows)` 按占位内容长度（当前 3 行）夹取。
- 侧栏内容超出可视高度时才可滚；当前占位内容不溢出，滚轮保持 0。
- host 在滚轮事件里先判断列是否落在侧栏带：落在侧栏带 → 只调用侧栏滚动，不得滚动内容区；否则走既有 transcript 滚动。

## Alternatives

- **B1. 用组件 overlay 注册侧栏/弹出框**：需要把自绘 chrome 改成组件，改动面大，且与固定区词汇不一致。
- **B2. 用 `HitPriorityFn` 阻止选区**：产品端 AGENTS 明确禁止产品路径使用 `HitPriorityFn`；且无法派发点击。
- **C1. 复用 `app.session.sidebar` 做三态循环**：会破坏既有 alt+u 直接开关语义与 BDD，用户配置也受影响。
- **D1. 拖拽分隔条**：需要新增 drag 状态与 host 事件路径；本轮先用键位 + 点击分隔条，拖拽留待后续。

## Risks

- chrome 捕获区会吞掉侧栏/弹出框内的所有鼠标事件；当前侧栏与弹出框没有内部可点元素，风险可控。
- 宽度档上限 40 列在 80 列终端恰好留下 40 列内容；更窄终端仍回落为弹出框。
- 侧栏滚动状态在占位内容阶段不可见；保留 plumbing 是为 c2912 业务字段回归时直接可用。
