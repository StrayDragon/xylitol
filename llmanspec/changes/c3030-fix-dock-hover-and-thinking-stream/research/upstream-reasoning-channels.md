# 上游 Responses 推理通道实测

目标：判定「halogen 在 xhigh 档没有 thinking 块」是上游网关缺陷还是本仓映射缺失。

## 探针方法

直接向网关 http://tufa:8731/v1/responses 发流式请求，统计 SSE 事件类型与推理字符数。四种组合：带 / 不带 reasoning 字段、effort=xhigh、带 / 不带 summary="auto"。

## 实测结果

| 请求体 | reasoning_text.delta 字符 | reasoning_summary_text.delta 字符 |
|---|---|---|
| 无 reasoning 字段 | 58 | 0 |
| reasoning.summary="auto" | 0 | 58 |
| reasoning.effort="xhigh" + summary="auto" | 0 | 58 |
| reasoning.effort="xhigh"（无 summary） | 58 | 0 |

四种组合的 response.output_item.done 都带完整 reasoning item（summary 与 content 两个数组都有正文），所以思考正文本身从未丢失，丢的是流式增量。

## 结论

不是上游 bug。网关按是否请求摘要切换通道：带 summary 只发摘要增量，不带则发原始推理增量。

本仓 OpenAiResponsesAdapter 只映射了 response.reasoning_text.delta，而 thinking.apply_thinking_openai_responses 在非 off 档恒注入 reasoning.summary = "auto"。于是：

- off 档：不注入 reasoning，走原始推理通道，有流式 thinking。
- xhigh 档：注入 summary，只走摘要通道，映射缺失，流式期间无 thinking，直到 output_item.done 才整块落地。

这正好解释用户观察到的「off 有、xhigh 没有」。

## 修复

map_responses_sse_event 同时接受两条增量通道，均映射为 XyChunk::ThinkingDelta。
