# c370 Design — TUI 多轮 tool-turn 截断修复

## 问题回顾（代码事实）

react loop（`src/agent/runtime/react.rs`）对**一次用户 prompt** 跑多轮 ReAct 迭代，每轮结束发 `XyEvent::TurnEnd`：

- `react.rs:357`：无 tool call → 发 `TurnEnd` + break（纯文本 turn，单轮）。
- `react.rs:436`：每个 tool 执行完发 `ToolExecutionEnd`。
- **`react.rs:453`：tool 执行循环结束后再发一个 `TurnEnd`**，随后 `if done { break }`。`done` 仅当 `XyChunk::Done`（行 309）时置 true。

即带 tool 的多轮 turn，react loop 在「tool 执行完、准备让模型续轮」时也发 `TurnEnd`——语义是「一个 ReAct 迭代边界」，不是「整个用户 turn 结束」。真正结束看流返回 `None`。

**对照证据**：print 模式（`print.rs:33`）`while let Some(event) = stream.next()` 消费整个流到 `None`，把每个 `TurnStart`/`TurnEnd` 当单轮边界打印（`[Turn N]` / `[Turn N end]`），带 tool 的多轮 turn 工作正常。证实多 `TurnEnd` 协议是设计意图，bug 在 TUI 侧。

## Bug 根因（TUI 侧）

`app.rs::spawn_drain`：

```rust
Some(ev) => {
    let is_end = matches!(ev, XyEvent::TurnEnd { .. });
    if tx.send(ev).is_err() { break; }
    if is_end { break; }   // ← 收到第一个 TurnEnd（react.rs:453 那个）就停 drain
}
```

带 tool 的多轮 turn，第一个 `TurnEnd`（tool 执行完、准备续轮）触发 `break`，drain task 结束。后续续轮事件（`TextDelta`/`ThinkingDelta`/最终 `TurnEnd`）**全部丢失**。主循环（`mod.rs`）收不到续轮事件，tail 停在 tool 执行后的状态，表现「REPL 卡住/退出」。

`app.rs::turn_done` = `matches!(event, TurnEnd)` 同样把中间 `TurnEnd` 当整个 turn 结束，`mod.rs` 据此调 `end_stream()` 清 tail——续轮还没开始就清了。

纯文本 turn 正常：唯一一个 `TurnEnd`（`react.rs:357`）恰好是真正结束。

## 方案选择

### 方案 A（采纳）—— 改 TUI drain 侧

`spawn_drain` 不在 `TurnEnd` break，改为 stream 返回 `None` 时 break（与 print 模式 `while let Some` 对齐）。`TurnEnd` 仍正常转发给主循环。

`turn_done` 语义重定义：**整个 turn 结束 = stream `None`（drain task 自然结束）**，而非「收到 TurnEnd」。

**问题**：drain task 自己 break 时如何通知主循环「流结束了」？当前架构里 drain task → forward task（`mod.rs:182`）→ 主循环 `Msg::Xy`。drain break 后 forward task 的 `while let Some(ev) = xy_rx.recv()` 也会结束（channel 空）。需要 forward task 在结束时发一个显式的「流结束」信号给主循环，触发 `end_stream`。

**机制**：在 `Msg` 加 `XyDone`（或复用一个 sentinel），forward task 退出前发之；主循环收到 `Msg::XyDone` 调 `end_stream`。这样：

- 中间 `TurnEnd`：正常转发，主循环 `handle_xy_event` 处理（flush 该轮 stream buffer 残留），**不**调 `end_stream`（tail 保留，续轮继续写入）。
- 流真正结束（`None`）：forward task 发 `Msg::XyDone`，主循环调一次 `end_stream`。

### 方案 B（否决）—— 改 react loop 不发中间 TurnEnd

让 react loop 只在 `done` 时发唯一 `TurnEnd`。否决理由：改变 `TurnEnd` 协议语义，影响 rpc/server 等所有消费 `XyEvent` 的面；且 print 模式依赖中间 `TurnEnd` 打印 `[Turn N end]` 单轮边界。回归面大、收益小。

## 设计细节

### `Msg` 枚举（`mod.rs`）

新增变体标记 drain 流结束：

```rust
enum Msg {
    Key(Event),
    KeyEof,
    Tick,
    Xy(Box<XyEvent>),
    XyDone,   // ← 新增：drain task 结束（stream None 或 cancel）
}
```

### `spawn_drain`（`app.rs`）

移除 `is_end` 检查 + 提前 break。改为只在 `cancel` 或 `None` 时退出：

```rust
loop {
    tokio::select! {
        _ = cancel.cancelled() => break,
        item = stream.next() => match item {
            Some(ev) => { if tx.send(ev).is_err() { break; } }
            None => break,   // stream 真正结束
        }
    }
}
```

### forward task（`mod.rs`）

drain task 结束后，forward task 检测到 `xy_rx.recv()` 返回 `None`（drain 关闭了 `xy_tx`），发 `Msg::XyDone`：

```rust
tokio::spawn(async move {
    while let Some(ev) = xy_rx.recv().await {
        if main_tx.send(Msg::Xy(Box::new(ev))).is_err() { return; }
    }
    // drain 关闭了 xy_tx（stream 结束）→ 通知主循环
    let _ = main_tx.send(Msg::XyDone);
});
```

> 关键：drain task 持有 `xy_tx`，它退出时 drop `xy_tx`，forward task 的 `recv()` 返回 `None`。这个 drop 时序天然成立（drain task move 了 `xy_tx`）。

### 主循环 `Msg::Xy`（`mod.rs`）

移除 `is_end`（中间 TurnEnd）触发的 `end_stream`。`TurnEnd` 仍经 `handle_xy_event` 处理（flush 该轮 buffer 残留到 scrollback——这是期望的，每个 ReAct 迭代的完整行应固化）。

新增 `Msg::XyDone` 分支：调 `app.end_stream()` + `draw_tail`。

### `turn_done`（`app.rs`）

此函数语义不再是「收到 TurnEnd」。两个选项：

- **选项 1**：删除 `turn_done`，改由 `Msg::XyDone` 驱动 `end_stream`。主循环不再需要 `is_end` 局部变量。
- **选项 2**：保留 `turn_done` 但改实现为常 `false`（占位，未来若需细粒度轮边界再用）。

倾向**选项 1**（删 dead function，符合「不加向后兼容 shim」原则）。

## 回归测试

用 `infra/provider/fake.rs` 的 `FakeModel` 构造多轮 tool-call 序列：

1. `TextDelta("Let me check")` → `FunctionCall(bash)` → tool 执行 → `TextDelta("The answer is 42")` → `Done`。
2. 断言：TUI 收到续轮 `TextDelta("The answer is 42")` 并 commit 到 scrollback。
3. 断言：`end_stream` 恰好调用一次（流结束时），非两次（不在中间 TurnEnd）。
4. 断言：纯文本 turn（单 `TurnEnd`）行为不变。

测试落点：`src/app/tui/app.rs` 的 `#[cfg(test)]`（drain 行为）+ `mod.rs` 测试（主循环 XyDone）。参考既有 `spawn_drain` 相关测试。

## 影响面

- `src/app/tui/mod.rs`：`Msg` 加 `XyDone`；主循环 `Msg::Xy` 移除 `is_end`→`end_stream`；新增 `Msg::XyDone` 分支；forward task 加退出通知。
- `src/app/tui/app.rs`：`spawn_drain` 移除 `is_end` 提前 break；删 `turn_done`。
- 不动 `react.rs`、不动 `print.rs`、不动 protocol/domain。
- 既有 27 TUI 场景 + BDD 必须回归通过。

## 风险

低-中。改动局限在 TUI 事件循环，不动协议/agent。主要风险是 cancel 路径：`cancel.cancelled()` 触发 drain break 时，也要正确发 `XyDone`（或主循环已有 abort 分支处理）。需测试覆盖 cancel 场景。
