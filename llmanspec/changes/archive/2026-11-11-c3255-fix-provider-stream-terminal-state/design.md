# Design — Provider 流终态语义（全部已决 2026-11-11）

## 0. 上游术语 gate

无上游依赖（XyChunk 词表现状即基线）。若 Done / ToolCallStart / ToolCallEnd 词形有变，先查 infra-provider spec 与 c3240 是否改动事件词表。

## 1. 已核实缺陷矩阵（三方言）

| 方言 | 缺陷（核证状态） |
|---|---|
| Responses | done_emitted 仅 EOF 兜底、finish 不查重复 → 双 Done；中断 function_calls 不 flush（状态结构已核证） |
| Completions | Done 在 choices 循环内逐 choice 发射（openai.rs:331-368 本轮核证）——一个 chunk 多 choice = 多个 Done；EOF 无终态不报错 |
| Anthropic | delta 无 start 静默自建累积器（:455-475 thinking 已核证、tool 对称）——非法 SSE 序列被接受 |

## 2. 修复（六项全定稿）

~~~mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Text: TextDelta
  Idle --> ToolOpen: ToolCallStart
  Text --> ToolOpen: ToolCallStart
  ToolOpen --> ToolOpen: ToolCallDelta
  ToolOpen --> Text: ToolCallEnd
  Text --> Done: Done（唯一且最后）
  ToolOpen --> Done: 违例（校验器报错）
  Done --> Done: 违例（第二个 Done）
~~~

1. chunk_stream_check：sdk 纯函数校验器（上图状态机）+ 三适配器共享测试宏；runtime 消费侧 debug_assert。
2. StreamOutcome { chunks, terminal } 三适配器统一；终态后零产出内聚。
3. 中断 flush：已开 ToolCall → 空参 ToolCallEnd + 流错误。
4. 乱序（delta 无 start / 重复 start / 终态后事件）→ 显式流错误。
5. Completions：Done 移出 choices 循环；EOF 无终态 → 错误。
6. rider：trace finalize 后属性写拦截（两 crate 同步）；fake Cyclic 回绕含第 0 步。

## 3. spec delta（landing 时）

infra-provider 增终态约束条款（Done 恰一次且最后；ToolCallStart/End 严格配对；EOF 无终态 = 错误；终态后零产出）+ 三方言 SSE fixture 矩阵 BDD。

## 4. 测试边界（seam）

单测：三方言 SSE 回放矩阵（双 Done / 无 Done / 悬挂 ToolCall / 乱序 delta / 终态后事件 / 中断 flush）；校验器全转移。回归：错误显式化后 is_retryable 分类不变。

## 5. 风险与回滚

- 风险：中断 flush 的空参 End 需下游工具结果处理接受空参错误结果。
- 回滚：校验器 / 适配器统一 / completions 修正三片独立。

## 6. 场景走查附录（新实现者必读）

场景 A（Completions 双 Done，openai.rs:331-368 已核证）：一个 data 分片含两个带 finish_reason 的 choice → 逐 choice 循环内各发一次 Done → ReAct turn 收尾跑两遍。修复后 Done 是流级动作（移出循环），EOF 无终态报错。

场景 B（Responses 悬挂工具调用）：function_calls 中 started=true 的调用已发 ToolCallStart，流中断 → 现状无 End → 上游半开。修复后中断路径 drain 累积器：补空参 ToolCallEnd + 流错误，ReAct 以失败工具结果闭合。

场景 C（Anthropic 乱序容忍，:455-475 thinking 已核证）：content_block_delta 先于 content_block_start 到达 → 现状静默自建累积器 → 上游见 End 无 Start。修复后为显式流错误，不自建不覆盖。

协调注：c3240 T4（RetryState 每 step 重建）与本单共享 is_retryable 回归矩阵——任意先后实施，双方回归都必须跑。

样例对照（测试夹具骨架）：

~~~text
Completions 双 Done（修复前 → 后）：
  in : 一个分片 choices[0..1] 均带 finish_reason=stop
  前 : Done{Stop}, Done{Stop}            后 : Done{Stop, usage}（流级一次）
  EOF 在 [DONE] 前断开：前=静默结束，后=流错误

Responses 悬挂（中断）：
  in : output_item.added(fc_1) + arguments.delta 后连接中断
  前 : ToolCallStart, ToolCallDelta, （无 End）
  后 : ToolCallStart, ToolCallDelta, ToolCallEnd{args:{}, id:fc_1}, 流错误

Anthropic 乱序：
  in : content_block_delta(index=2) 先于 content_block_start(index=2)
  前 : 静默自建累积器，随后发 End（上层见 End 无 Start）
  后 : 流错误 delta without start (index=2)；重复 start 同理

校验器判定：
  TextDelta,Done ✓ | Start,Delta,End,Done ✓ | Start,Done ✗未闭合 | Done,Done ✗ | Done,TextDelta ✗终态后产出
~~~
