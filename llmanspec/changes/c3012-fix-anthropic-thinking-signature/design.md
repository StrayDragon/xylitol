# 设计：Anthropic thinking 签名保真

## 采集

- 流式状态机按 block index 保存 thinking 文本与 signature：`content_block_start` 的 thinking block 可能直接带 `signature`；`signature_delta` 增量拼接。
- `content_block_stop` 对 thinking block 发 `ThinkingEnd`，`thinking` 为完整累积文本（runtime 以该字段覆盖 `thinking_acc`，见 `react/mod.rs`）。
- 非流式按 content 顺序：thinking block → `ThinkingDelta(thinking)` + `ThinkingEnd{thinking, signature}`。

## 回放

- 有签名且 `redacted=false`：`{"type":"thinking","thinking","signature"}`，保持原始顺序（thinking 位于 text / tool_use 之前）。
- `redacted=true` 且有签名：`{"type":"redacted_thinking","data": signature}`，忽略 thinking 文本。
- 无签名：维持 `{"type":"text","text":thinking}` 降级并 `log::debug!` 记诊断；不伪造签名。

## 为什么不补 redacted 生产者

`AgentPart::Thinking.redacted` 在 runtime 恒为 false，且 `XyChunk::ThinkingEnd` 没有 redacted 位。补齐需要改 SDK 枚举与 runtime 累积状态，超出本切片；记入 proposal 未决问题。

## 风险

- 无签名历史在 thinking 开启且末轮 assistant 含 tool_use 时仍可能被 API 拒绝；降级路径只保证不伪造签名，不保证通过。
- 多 thinking block 交错仍折叠为单一 Thinking part（runtime 既有形状）。
