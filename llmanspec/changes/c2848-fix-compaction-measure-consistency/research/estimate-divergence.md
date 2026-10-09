# 三口径分歧量化（T1 校准对拍）

## 数据（深会话 0664dab6，2026-10-08）

探针 `cargo run --example lab_compaction_measure`（parsed=201 unique=201，含 active+segments）：

| 口径 | 数值 | 相对 provider |
|---|---|---|
| A 切点 chars/4 累计（`estimate_tokens_entry_for_cut`） | **92,657** | **5.31x** |
| B 统一估算（`estimate_from_session_entries` · EstimateOpts::default） | **34,872** | 2.00x |
| C provider llm.request input（Langfuse 15:41Z） | **17,441** | 1.00x |

参考：effective_keep_budget = window(33,792) − reserve(16,384) = **17,408**。footer 观测 35k ≈ B。

## 结论

1. **chars/4 对中文会话系统性高估 ~5.3x**（1 汉字 ≈ 1 token，chars/4 相当于 4 token/汉字）。切点路径把 92k 当上下文，远超窗口——决策度量严重失真。
2. **统一估算（tokenizer 家族）相对 provider 仍 2x**：本地/映射 tokenizer 对中文内容的计数高于 openai-responses 服务端实测；且 EstimateOpts::default 不含 fixed_context，差异纯在消息体计数口径。
3. provider 17.4k 为模型实际见到的 input（服务端权威计数），应作为真值锚。

## 对 T2 的约束

- 切点判定并入统一估算入口族后，估算仍 2x provider → r1924 的「对拍可解释」要求：文档化 tokenizer-vs-provider 偏差（建议把此 2x 记录为已知偏差，并长期以 provider 实测校准展示百分比基准）。
- 决策预算（keep_budget=17k）是针对「统一估算 > provider 2x」的保守放大；真实 17.4k(provider) 恰好压在预算线上——即当前「不压缩」是保守正确的，但展示(35k=103%)却误报超窗。优先级：**统一决策与展示的度量源 > 向 provider 对齐**（后者留校准系数）。
