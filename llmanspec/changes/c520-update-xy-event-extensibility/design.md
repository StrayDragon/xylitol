# design — c520 XyEvent 防宽表

## 问题

若把 OpenAI/Anthropic/未来厂商的每一种 SSE/`response` 事件都提升为 `XyEvent` 变体：

- enum 无限膨胀（宽表）
- 应用面 match 爆炸
- 仍无法覆盖厂商私有字段与实验事件
- 与「精选导出契约」冲突

## 两层模型（采纳）

```text
Provider SSE / HTTP 流
    → infra adapter 映射
    → XyChunk（TextDelta / ThinkingDelta / FunctionCall / Done）
    → ReAct 循环
    → XyEvent（Agent/Turn/Message/Tool/Compaction/Queue… 闭集）
    → 应用面 / protocol::Event
```

| 层 | 职责 | 可变性 |
|---|---|---|
| **XyChunk** | 模型流的最小公分母 | 随「我们需要的流能力」缓慢增，不随厂商事件名增 |
| **XyEvent** | agent 生命周期与 UI/多面可消费语义 | **闭集**：只加真正跨面语义（如 QueueUpdate），不加 `OpenAiFoo` |
| **AgentMessage 载荷** | 结构化内容（tool call args、usage…） | 字段可扩展；不是事件宽表 |
| **adapter 私有** | 厂商原始类型 | 关在 infra，可随便变 |

对齐直觉：pi 也是 `AssistantMessageEvent`（流内）⊂ `AgentEvent`（循环），而不是把所有 provider 事件抬到同一层。

## 厂商大变时怎么办

1. **先改 adapter**：把新 SSE 映射到现有 `XyChunk` / Message 字段。
2. **若现有 Chunk 表达不了「我们产品需要的能力」**（例如新的可展示流类型）：扩展 `XyChunk`（仍是公分母），再由 ReAct 决定是否发射已有 `XyEvent`（如 `ThinkingDelta`）或丰富 `MessageUpdate`。
3. **若只是厂商调试/计费私有事件**：丢弃或 tracing，**不**进 `XyEvent`。
4. **可选逃生舱（future，本变更不强制实现）**：`XyEvent::Extension { namespace, payload: Value }`；所有面默认 ignore + log。用于极少数跨面实验，不是主路径。

## 明确禁止

- `XyEvent::OpenAi*` / `XyEvent::Anthropic*` 等厂商前缀变体
- `protocol::Event` 为厂商事件平行加变体
- 应用面直接依赖 adapter 或 vendor 类型

## 与未知事件

bridge / print / server：未识别变体 → tracing 降级，不 panic（与 app-tui-bridge 意向一致）。核心循环不依赖未知变体即可完成工具轮次。

## 验收

- 规范 ar08 / ar-ev1 / ip10 通过 validate
- `domain/lifecycle.rs` 模块注释指向本 design（短指针）
- 无代码强制新增 Extension（除非实现时发现必要）
