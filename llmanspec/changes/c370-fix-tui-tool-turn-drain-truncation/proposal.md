---
change_id: c370-fix-tui-tool-turn-drain-truncation
title: 修复 TUI 下带 tool-call 的多轮 turn 被 TurnEnd 提前截断 drain 的问题
status: proposed
priority: 370
depends_on:
  - c365-add-streaming-mutable-tail
author: agent
---

# c370-fix-tui-tool-turn-drain-truncation

## 状态说明

**Draft（仅记录，不实施）**。本变更由 c365/future.md 登记的 known bug 派生。
当前重心是 TUI 流式渲染主体，功能性 bug 暂不处理；待 TUI 主体稳定后启动。
下面是代码级调查结论（已核对真实代码，非推测），实施时直接据此推进。

## Why（bug 现象与根因）

c365 落地后真终端冒烟发现：**带 tool 调用的 turn 结束后，REPL 不再回到输入提示**
（表象是「REPL 退出/卡死」），而纯文本 turn 正常。c365 把它划在 scope 外
（流式渲染），登记在 c365/future.md。

### 根因（代码事实，已核对）

react loop（`src/agent/runtime/react.rs`）对「一次用户 prompt」会跑**多轮**模型调用，
每轮结束都发 `XyEvent::TurnEnd`：

- 行 357：`tool_calls.is_empty()` 时发 `TurnEnd` + break（纯文本 turn，单轮）。
- 行 436：每个 tool 执行完发 `ToolExecutionEnd`。
- **行 453：tool 执行循环结束后再发一个 `TurnEnd`**，然后 `if done { break }`。
  这里 `done` 仅在 `XyChunk::Done`（行 309-311）时置 true——即模型明确不再产内容/tool。

也就是说：**带 tool 的多轮 turn，react loop 会在「tool 执行完、准备让模型续轮」时
也发一个 `TurnEnd`**（行 453），它语义上是「一个 react 迭代结束」，不是「整个用户
turn 结束」。真正结束要看 `done`（`XyChunk::Done`）。

但 TUI 的 `spawn_drain`（`src/app/tui/app.rs:142-170`）当前实现：

```rust
match item {
    Some(ev) => {
        let is_end = matches!(ev, XyEvent::TurnEnd { .. });
        if tx.send(ev).is_err() { break; }
        if is_end { break; }   // ← 收到第一个 TurnEnd 就停止 drain
    }
    None => break,
}
```

收到**第一个** `TurnEnd`（即 tool 执行完那一个，行 453）就 `break` drain task。
后果：带 tool 的多轮 turn，模型续轮的事件（后续 `TextDelta`/`ThinkingDelta`/
最终 `TurnEnd`）**全部丢失**，TUI 在 tool 执行后就停止接收事件；主循环
（`mod.rs:218 Msg::Xy`）因收不到后续事件，tail 停留在 tool 执行后的状态，
表现为「REPL 卡住/退出」。

### 为什么纯文本 turn 正常

纯文本 turn：模型一次返回文本、无 tool → 走行 357 分支，发唯一一个 `TurnEnd` +
`break`。这个 `TurnEnd` 恰好就是真正的 turn 结束，drain 在此处 break 行为正确。
所以 bug 只在「带 tool 的多轮 turn」暴露。

## What Changes（实施方向）

1. **区分「单轮结束」与「整个 turn 结束」**。两个可选方案（实施时评估）：
   - 方案 A（改 drain 语义）：`spawn_drain` 不在第一个 `TurnEnd` break，改为
     在 stream 返回 `None`（Driver 流真正结束）时 break。`TurnEnd` 仍正常转发
     给主循环用于 `end_stream`/tail 清理——但需注意多轮 turn 的中间 `TurnEnd`
     不应触发 `app.end_stream()`（否则 tail 提前清空，续轮文字无处可显）。
   - 方案 B（改事件语义）：让 react loop 在中间轮不发 `TurnEnd`，只在真正
     `done` 时发唯一的 `TurnEnd`。但这会改变 `TurnEnd` 的协议语义，影响 rpc/server
     等所有消费 `XyEvent` 的面，回归面更大，风险更高。

   **倾向方案 A**：bug 在 TUI drain 侧的解读错误，修复应局限在 TUI；不动协议语义。

2. **`app.rs::turn_done()`（行 244-246）当前 `matches!(event, TurnEnd)`**：方案 A 下
   需要重新定义「turn 真正结束」的判定——可能改为「stream `None`（drain task 自然
   结束）」而非「收到 TurnEnd」。需配套调整 `end_stream` 调用时机（mod.rs:233-238）。

3. **回归覆盖**：用 `FakeModel` 构造一个发 `TextDelta → FunctionCall →（tool 执行）
   → TextDelta → Done` 的多轮序列，断言 TUI 收到续轮的 `TextDelta` 并最终
   `end_stream` 恰好一次（非两次）。

## Capabilities

- `app-tui`（修改）：TUI 的 turn 结束判定从「收到 TurnEnd」改为「Driver 流结束」，
  或等价的、能区分中间轮与最终轮的机制。

## Impact

- **受影响代码**：`src/app/tui/app.rs`（`spawn_drain` + `turn_done` + `end_stream`
  时机）、`src/app/tui/mod.rs`（`Msg::Xy` 分支的 `is_end` 处理）。
- **受影响规范**：`app-tui`。
- **风险**：中。触及 TUI 事件循环热路径。缓解：FakeModel 多轮序列回归测试；
  既有 27 个 TUI 场景 + BDD 必须回归通过。

## 调查笔记（实施时复核）

- react loop 多轮结构：`src/agent/runtime/react.rs:280-458`（每轮发一个 TurnEnd，
  `done` 才是真终止）。
- drain 提前 break：`src/app/tui/app.rs:156`（`is_end` 后 break）。
- 主循环 turn_done：`src/app/tui/mod.rs:219`（`app.turn_done(&ev)`）+ `:233-238`
  （`is_end` 时 `end_stream`）。
- print 模式不受影响：print 直接消费整个 stream 到结束，不在中间 TurnEnd 停。
