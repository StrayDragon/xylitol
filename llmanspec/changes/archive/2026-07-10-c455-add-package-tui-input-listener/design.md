# Design — c455 InputListener

## 背景

pi 在 focus 路由前有 `addInputListener`；xylitol-tui 目前 `dispatch_event` 直接进 overlay/root。`agent_demo` 把 Ctrl+C/Esc 塞进根 `Component::handle_input`，无法在 Editor 焦点前可靠拦截，也挡不住后续 c460/c480。

## 决策

### D1 — `InputEvent` 原生，不做 VT 字符串

对齐 PI_DELTAS D03：listener 签名吃 `InputEvent::{Key,Paste}`，不引入 KeyEvent→VT 路径。

### D2 — 注册 API

`TUI::add_input_listener` 注册回调，返回 `u64` id；`remove_input_listener(id)` 注销。回调类型：

```rust
FnMut(InputEvent) -> InputListenerResult + 'static
```

**不**把 `&mut TUI` 传入回调（避免自借用）；app/demo 用 `Arc`/`Rc`/`Cell` 闭包捕获可变状态。

### D3 — 调用顺序

注册顺序 FIFO；第一个返回 `Consumed` 的 listener 终止链，且 **不** 再路由到 overlay/focus。

### D4 — `dispatch_event` 管道

```text
listeners → focused overlay（若 capturing）→ focused root child
```

Paste 同样走 listener。

### D5 — 结果枚举（v1）

```rust
enum InputListenerResult { Continue, Consumed }
```

`Replace(InputEvent)` **延后**（可在后续 change 加）；v1 只保证拦截/放行。

### D6 — 渲染

消费事件并改 UI 的 listener **应** `request_render()`。`start()` 循环在 dispatch 后仍会 render；host-driven（c460）依赖显式 request。

### D7 — 引擎无内置语义

Esc/Ctrl+C 语义由 demo/app 注册；引擎保持键语义无关。

### D8 — demo 迁移

`agent_demo` 在 `main` 注册 Ctrl+C / Esc listener；根 `handle_input` 去掉对应全局键分支。Ctrl+C：编辑器非空则清空，否则 quit；Esc：有活动流则 abort，否则保留 palette/popup 既有行为（可由 listener 或 Editor 分担）。
