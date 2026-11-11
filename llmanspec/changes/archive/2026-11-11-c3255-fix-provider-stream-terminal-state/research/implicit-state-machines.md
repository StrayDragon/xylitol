# Provider 流状态扫描证据（2026-11-10）

## P1 Responses 流状态（高，implicit）

- 位置：plugins/xylitol-official-plugin-provider-openai/src/provider/native/openai_responses.rs:403-413（Lead 已复核状态结构：function_calls HashMap<String,(String,String,bool)> 第三元为 started、done_emitted、terminal_error）
- 状态集合：streaming / done-emitted / errored / 每工具调用未开始或已开始
- 转移点：:450-453 insert started=true；:462-473 参数增量补发 ToolCallStart；:513-540 output_item.done 消费累积器；:556/:565 finish_responses_terminal；:577-601 写 terminal_error；:661 置 done_emitted；:384-386 take() 带外传错误；:395-399 EOF 兜底
- 缺失：finish_responses_terminal 不检 done_emitted（重复 Done）；terminal_error 走 take() 带外，返回类型无法表达失败；流中断时 function_calls 不 flush

## P2 Completions 流累积器（中，implicit）

- 位置：openai.rs:237-238
- 载体：tool_accumulators HashMap<u32,(id,name,args,started)> + pending_usage，无 done_emitted
- 转移点：:281-329 增量与 started 翻转；:331-368 finish_reason 时 drain 并 yield Done（choices 循环内，一个 chunk 两个 choice 发两次 Done）；:264-270 usage 暂存
- 缺失：EOF 无终态不报错（openai.rs:395 与 anthropic :637 两个兄弟实现都报）

## P3 XyChunk 序列协议跨三实现（中，implicit）

- 位置：openai.rs:227-372、openai_responses.rs:348-401、anthropic_messages.rs:359-643
- 载体：chunk 枚举顺序本身（Start 先于 Delta、End 唯一、Done 唯一最后）
- 缺失：无共享 validator；三实现对非法转移判定不同（anthropic 容忍乱序 delta、responses 容忍重复 Done、completions 允许双 Done）

## P4 Anthropic 流分块累积器（中，implicit）

- 位置：anthropic_messages.rs:370-374
- 载体：current_block_index Option<usize> + tool/thinking 两张 accumulator map + done_emitted
- 转移点：:411-439 start；:442-508 delta（:464-467 delta 早于 start 静默自建累积器）；:510-540 stop；:542-593 message_delta drain 并 Done；:601-613 message_stop 兜底；:637-641 EOF 报错
- 缺失：同 index 重复 content_block_start 覆盖累积器（已发内容丢失）；非法序列被接受不报错

## P5 请求追踪句柄（低，implicit）

- openai trace.rs:25、:143-158、:203-207（anthropic 同构同行号）：finalized CAS 幂等，但 finalize 后 emit_mapped_chunk 仍可追加 output_buf；两 crate 各一份相同实现，修复需双改。

## P6 fake provider 游标（低，implicit）

- fake/mod.rs:79-80：cursor AtomicUsize；Cyclic 回绕 CAS 到 1 跳过第 0 步；resolve_next 无限循环风险（全 Delay 脚本）。仅测试替身。
