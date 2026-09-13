# Design — c25-fix-compact-overflow-placeholder

## D1 估算口径：固定开销怎么折算、何时计入

- `FixedRequestContext { system_prompt: Option<String>, tool_schemas: Vec<XyToolSchema> }` 进入 `EstimateOpts`；估算法把 system prompt 折成一条伪 user 文本行、tool schemas 折成一条序列化文本行，与历史消息一起过既有 `estimate_context`（Heuristic / LocalTokenizer 后端天然可用）。
- **Api 锚点优先且不叠加**：`last_usage`（usage.input）是 provider 对完整请求（system + tools + 历史）的权威计数，有锚点时 MUST NOT 加 overhead——否则双计。实现上按 provenance 分流：Api → 原值 + trailing；其余 → 历史 + overhead。
- 折算单位仍是 chars/4（与 c8 切点口径同源），偏保守（低估固定开销）方向可接受：overhead 真实值 ~9k、chars/4 约 6-7k，clamp 预算相应略宽，仍远优于现状（视 overhead 为 0）。

## D2 切点预算 clamp 的单位处理

- 预算：`effective_keep = min(keep_recent_tokens, window − reserve − overhead_real_estimate)`；overhead 取 D1 的 chars/4 折算值（保守低估 → 预算偏宽 → 不至于把保留窗切得过碎）。
- **不做 ratio 缩放**（real/chars4 比例修正）：比例内容相关、抖动大、难测试；clamp 已消除「预算 ≥ 阈值 → 全保留」的退化路径，残余误差只表现为小窗口下每轮多 compact 一次（有界、单调收缩），不是溢出。
- `window == 0`（未知窗口）或 overhead 未注入（纯 store 调用方）→ clamp 退化为 `keep_recent_tokens`，与现状完全一致；force 路径同步拿 window（capabilities 已有 `ctx_window`）。
- clamp 在 `prepare_compaction` 与 `compact_session` 共享同一 helper，保证 prepare 判定与实际切点一致。

## D3 wire CompactionEnd 兼容性

- `Event::CompactionEnd` 字段全部对齐 `XyEvent::CompactionEnd`（Option/缺省 serde），旧客户端（只认无字段形态）读新载荷：serde 默认忽略未知字段不 panic；新客户端读旧载荷得 None → 现状行为。同仓库同版本发布，无双版本窗口，不做 `migrations/`。
- 反序列化侧不再重建「伪成功」事件：`aborted/error_message` 原样还原，bridge 的 Complete/Failed/Aborted 分流自此在 remote 路径与本地路径一致。

## D4 resume / footer 同源

- host 在会话激活（attach / switch / resume 完成）时发一次 `ContextTokenSettlement(reason=leaf_changed)`——fill 既有的无人消费 variant；live footer 直接吃事件，免拉全量条目。
- 刷新类路径（`refresh_footer_tokens`）走新 unary `Command::EstimateContext`：host 侧 `estimate_from_session_entries` + `fixed_context`（host 才有 system prompt 与 tokenizer 映射）。remote 客户端本地估算代码删除。
- 「不主动发送」为既有结构事实（threshold compact 后 run 结束，重试仅 c22 overflow 一次），本 change 不改 c22；占位值只是让下一次（用户发起的）请求前的 footer / 闸看到真实尺寸。

## D5 明确不做

- 不改 `find_cut_point` 的 chars/4 走查本体（c8 口径保持 pi 对齐）。
- 不改 overflow compact-and-retry（c22）。
- 不动 `keep_recent_tokens` 默认值（20000 在大窗口模型上不变；小窗口由 clamp 兜底）。
