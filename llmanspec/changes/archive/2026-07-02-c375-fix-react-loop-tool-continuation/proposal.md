---
change_id: c375-fix-react-loop-tool-continuation
title: 修复 ReAct loop 在 tool call 后被 XyChunk::Done 误终止，导致模型从不续轮
status: proposed
priority: 375
depends_on:
  - c370-fix-tui-tool-turn-drain-truncation
author: agent
---

# c375-fix-react-loop-tool-continuation

## Why（真根因）

真终端测试发现：**带 tool 的 turn 在 tool 执行后停止，模型从不续轮**——agent 调一次 tool 就结束，不把 tool 结果喂回模型继续。

### 根因（代码事实）

`react.rs` 的 for 循环用 `done` 标志控制终止：

- `react.rs:283` `let mut done = false;`
- `react.rs:309-311` 收到 `XyChunk::Done` → `done = true`
- `react.rs:455` `if done { break }`

**`done` 的语义错了**。`XyChunk::Done` 只表示「**这一次模型流**结束」，不是「整个 turn 结束」。而 provider 在 tool call 后**必然**发 `Done`（openai.rs:156-176：finish_reason 到达时先 yield 所有 FunctionCall，再无条件 yield Done；anthropic/openai-responses 同理）。

所以带 tool 的 turn：模型调 tool → provider 发 `FunctionCall` + `Done` → `done=true` → react loop 执行 tool → `TurnEnd` → `if done { break }` **提前退出 for 循环**。模型从没机会看到 tool 结果、从没续轮。

正确的 ReAct 终止条件（react.rs:356 已正确实现）：**模型本轮没调 tool（`tool_calls.is_empty()`）= 做完了**。`:455` 的 `if done { break }` 是冗余且错误的——它让带 tool 的 turn 永远只跑一轮。

### 调研证据

- **provider 行为**：openai.rs:156-176 / anthropic_messages.rs:257 / openai_responses.rs:241 都在 finish_reason 时 yield FunctionCall(s) 后立即 yield Done。`Done` 是「单次模型流结束」的通用语义，不是「turn 结束」。
- **正确的多轮 ReAct**：模型调 tool → agent 执行 tool → 把 tool 结果加入 history → **再调模型**（for 循环下一轮）→ 模型看到结果决定续调 tool 或给最终回复。终止 = 某轮无 tool call。
- **为何长期未发现**：测试基础设施 `MockModel`（react.rs:677）无状态——每次 `generate_stream` 返回相同 chunks，无法表达「第一轮 tool、第二轮 text」。且 fake provider 的 `ScenarioStep::ToolResult` 被 `continue` 跳过（fake.rs:147）。所以从没有测试覆盖「tool → 续轮」完整序列。

## 与 c370 的关系（c370 是误诊，但其修复保留）

c370 把「tool-call turn 卡住」误诊为 TUI drain 问题（spawn_drain 在第一个 TurnEnd break）。**这是误诊**：

- c370 的诊断假设「react loop 的中间 TurnEnd 是正常的单轮边界，TUI drain 不该提前停」。
- 真相：react loop 因 `if done { break }` **根本没产生续轮**，stream 在第一轮 TurnEnd 后就 `AgentEnd` + None 了。TUI drain（即使修了）也收不到续轮事件——因为续轮从未发生。
- **但 c370 的修复本身正确且保留**：drain 跑到 stream None（而非第一个 TurnEnd）是对的防御性行为；Msg::XyDone + 每 turn cancel token 也是合理改进。c370 不是浪费——它修复了 TUI 事件循环的另一层隐患，只是没解决用户报告的 bug。本变更（c375）才是用户 bug 的真正修复。

## What Changes

1. **删 `react.rs:455` 的 `if done { break }`**。终止逻辑完全交给 `:356` 的 `if tool_calls.is_empty() { break }`（无 tool = 做完）。
2. **清理 `done` 变量**：删 `:283` 声明；`:309-311` 的 `XyChunk::Done` 分支改为 no-op（match 仍需覆盖变体，但不再设标志）。
3. **加回归测试**：新增有状态 `StatefulMockModel`（每次 `generate_stream` 返回不同 chunks），构造 `[FunctionCall+Done, TextDelta+Done]` 两轮序列，断言续轮文字到达 + 恰好两个 TurnEnd。

## Capabilities

- `agent-runtime`（修改）：ReAct loop 终止条件修正——turn 在「某轮无 tool call」时结束，不在「收到 XyChunk::Done」时结束。

## Impact

- **受影响代码**：`src/agent/runtime/react.rs`（删 `if done { break }` + 清理 `done` + 新增 StatefulMockModel 与回归测试）。
- **受影响规范**：`agent-runtime`（r1 execution-loop 眰在真正成立）。
- **风险**：低。改动让 loop **多跑**（续轮），而非少跑；纯文本 turn 行为不变（仍 `tool_calls.is_empty()` 第一轮就 break）。max_iterations=50 兜底防无限循环。

## 反降级护栏

- [x] 带 tool 的 turn MUST 续轮：tool 执行后 for 循环进入下一轮，把 tool 结果喂回模型。
- [x] 终止条件 MUST 是「某轮 `tool_calls.is_empty()`」，MUST NOT 是「收到 `XyChunk::Done`」。
- [x] 纯文本 turn（无 tool）行为 MUST 不变（第一轮即结束）。
- [x] 回归测试 MUST 用有状态 mock 覆盖「tool → 续轮文字」序列。
