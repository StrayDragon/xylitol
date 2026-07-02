# c375 Design — ReAct loop 续轮修复

## 问题（代码事实）

react loop（`react.rs`）for 循环用 `done` 标志终止：

```rust
let mut done = false;                 // :283
...
XyChunk::Done { .. } => { done = true; }   // :309
...
yield XyEvent::TurnEnd { ... };
if done { break; }                    // :455  ← bug
```

`XyChunk::Done` 只表示「**单次模型流**结束」。provider 在 tool call 后必发 Done（openai.rs:156-176：finish_reason 时先 yield FunctionCall(s) 再 yield Done；anthropic/openai-responses 同理）。于是带 tool 的 turn：`done=true` → `:455 break` → 模型从不续轮。

正确终止条件 `:356 if tool_calls.is_empty() { break }` 已存在且正确——某轮无 tool call = 做完。

## 修复

1. 删 `:455` 的 `if done { break }`（加注释说明 Done 不是 turn-end）。
2. 删 `:283` 的 `let mut done`；`:309` 的 `XyChunk::Done` 分支改 no-op（match 仍需覆盖变体）。

修复后 loop：tool 执行 → TurnEnd → for 下一轮 → 模型看到 tool 结果 → 若无 tool 则 `:356` break。

## 测试基础设施改进

既存 `MockModel`（react.rs:677）无状态（每次返回相同 chunks），无法表达多轮。新增 `StatefulMockModel`（`Mutex<Vec<Vec<XyChunk>>>`，每次 `generate_stream` pop 一个序列），构造 `[FunctionCall+Done, TextDelta+Done]` 两轮，断言续轮文字到达 + 两个 TurnEnd。修复前测试红，修复后绿。

## 边界与风险

- **纯文本 turn**：第一轮 `tool_calls.is_empty()` → `:356` break，行为不变。
- **无限循环兜底**：max_iterations=50（builder.rs:60），即使模型反复调 tool 也会终止。
- **Done chunk 仍被 match**：exhaustive 覆盖保留，只是不再设标志。
- **print 模式**：同样受益（print 也消费同一 react loop），带 tool 的多轮 turn 现在能完整打印。

## 不在范围

- c370 的 drain 修复（已归档，保留）——它修的是 TUI 事件循环另一层，与本次 react loop 修复正交。
- TUI Spinner 独立状态行——单独提案。
