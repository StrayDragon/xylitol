# Design: c1720-add-otel-turn-terminal-status

## 现状

| 层 | 已有 | 缺口 |
|---|---|---|
| 面 | `Error("aborted")`、`stop_reason=aborted` | — |
| OTEL generation | otel17 abort finalize | — |
| OTEL turn | `agent.turn` drop 无终态 | ✅ 本 change |

## 目标

```text
agent.turn
  └─ … iteration / llm / tool …
# drop/finish:
#   ok      → 无 ERROR level
#   aborted → level=ERROR, status_message=aborted
```

实现倾向：`AgentTurnSpan` 显式 `finish(end_reason)` 或 Drop 前由 ReAct 写入；cancel token 触发路径标 aborted。

## 测试 seam

| Seam | 覆盖 |
|---|---|
| CollectingReporter | abort 路径 turn 属性；正常完成无 ERROR |
| 关闸 | start/finish noop |
| 不扩 BDD step | 对齐 otel18/19（feature:false + 单测） |

## 非目标

完整 error 分类矩阵 · wire 新事件 · compact（c1710）· generation I/O 档位变更
