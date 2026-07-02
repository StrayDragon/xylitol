# c375-fix-react-loop-tool-continuation — Tasks

> 修复 ReAct loop 在 tool call 后被 XyChunk::Done 误终止。provider 在 FunctionCall 后必发 Done（单次模型流结束），react loop 误把 Done 当 turn 结束 → for 循环 break → 模型从不续轮。删 `if done { break }`，终止交给 `tool_calls.is_empty()`。

## 0. 规划工件

- [x] proposal.md（含 c370 误诊说明）
- [x] specs/agent-runtime/spec.toon（delta：add ar17 react-loop-continues-after-tool-call）
- [x] `llman sdd validate c375 --strict` 通过

## 1. 复现测试（红→绿）

- [x] 新增 `StatefulMockModel`（每次 generate_stream 返回不同 chunks）
- [x] `tool_call_then_continuation_round_reaches_final_text`：[FunctionCall+Done, TextDelta+Done]，断言续轮文字到达 + 恰好两个 TurnEnd（修复前红，修复后绿）

## 2. 修复（react.rs）

- [x] 删 `:455` 的 `if done { break }`（加注释说明为何不能在 Done 时终止）
- [x] 删 `:283` 的 `let mut done = false`
- [x] `:309` `XyChunk::Done` 分支改 no-op（match 仍覆盖变体）

## 3. 校验

- [x] 复现测试转绿
- [x] `cargo test --lib` 476 全过
- [x] `cargo test --features tui --lib tui` 73 全过
- [x] `cargo test --test bdd` 87 全过
- [x] `cargo fmt --check` + `cargo clippy --lib` 干净
- [x] 同步本 tasks.md 勾选 + 归档
