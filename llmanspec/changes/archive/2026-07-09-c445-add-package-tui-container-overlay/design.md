# Design — c445 Container + OverlayHandle

## Context

pi `TUI extends Container`：根就是垂直栈。xy 的 `TUI` 直接持有 `components: Vec<Box<dyn Component>>`，应用面若要「chat + status + editor」只能自写一个大 `Component`（`agent_demo` 现状）或等 `Container`。

`show_overlay` 已有合成与 `hidden` 字段，但无对外句柄——host 无法关闭 overlay。

## Decisions

### D1 — `Container` 是普通 `Component`，不是 `TUI` 基类

Rust 没有 TS 式 `TUI extends Container`。保持 `TUI` 拥有根 `components` 列表；另提供 `Container` 供应用面嵌套。根仍可 `add_child(Box::new(Container::…))`。

### D2 — OverlayHandle 用索引/世代，不用裸指针

`Box<dyn Component>` 所有权在 `TUI`。句柄持有 `overlay_id: u64`（单调递增），操作时在栈中查找；hide 后 id 失效（后续调用 no-op 或返回 false）。避免自引用结构。

### D3 — 输入仍只路由到 focused 根组件

`Container` 不自动把 `InputEvent` 扇出到子节点（与 pi Container 一致：Container 本身无 handleInput）。焦点组件若是 Container 内的子组件，需由应用面把焦点设在可 `Focusable` 的叶子，或由叶子所在的自定义根组件转发——本变更不引入自动焦点树遍历。

### D4 — 图片裁剪策略

应用面与 `agent_demo` 不依赖 Kitty/iTerm 图片。保留引擎宽度豁免所需的 `is_image_line`（及最小探测若测试依赖）；删除或 `cfg` 掉 `Image` 组件与大块 encode API。若测试仅覆盖 encode，一并删测。

### D5 — focus-restore 延后

pi 的 eligible/blocked overlay focus restore 复杂。本变更只保证：hide 当前 overlay 时焦点回到 `pre_focus` 或下一可见 capturing overlay。嵌套互抢焦点的完整状态机进 `future.md`。

## Alternatives considered

| 方案 | 为何不选 |
|---|---|
| 让 `TUI` 继承式变成 Container | Rust 组合更清晰；现有 API 已是 `add_child` |
| OverlayHandle = `Rc<RefCell<…>>` | 与现有所有权模型冲突，生命周期难测 |
| 本变更就做 App Shell | 范围过大；先补包 API |

## Risks

- OverlayHandle 与现有 `overlays: Vec<(Component, Options, Entry)>` 重构时易回归合成顺序 → 用 harness 锁 hide/show 帧。
- 裁剪 `terminal_image` 时误删 `is_image_line` → 宽度不变量测试失败。
