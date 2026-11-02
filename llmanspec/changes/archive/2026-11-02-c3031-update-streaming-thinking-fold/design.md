# 流式思考折叠（呈现层）

## 默认态

| 阶段 | 无 per-id 覆盖时 |
|---|---|
| 流式 `live:streaming` | 展开（看得到正在写的正文） |
| 已封 Thought | 折叠（与现行 Thought 默认一致） |

`thinking_effective("live:streaming")` 在覆盖表未写时返回 true，不得误用全局 `thinking_expanded`（默认 false），否则第一下点击会「展开」一个已经展开的块。

## 点击与增量

- 块头登记 `FoldTarget::Thinking("live:streaming")`（仅 thinking 的 live 簇头与此同一目标，避免点簇头却折不到正文）。
- 点击走既有 `toggle_thinking`：与 paint 共用 `thinking_effective`。
- `ThinkingDelta` 只追加缓冲。paint 不得每帧改覆盖表，不得因 delta 清覆盖或强制展开。

## 通道结束

`apply_ui_model` 见到 `streaming_think_id` 从 `live:streaming` 变为 None 时：

- 若无覆盖：不写入，落地 Thought 走默认折叠。
- 若有覆盖：迁到新 Thinking id。值为 true 且该簇仅 thinking 时 `expand_live_cluster`，否则只迁覆盖（混合簇走 L1 Thought 有效态，不把工具簇整簇打开）。

## 簇折叠例外（保留）

簇默认折叠。若簇折叠就藏 live thinking，正在输出的思考会被工具簇默认态吞掉。因此：簇折叠时 live thinking 仍以独立行呈现；该行正文只服从 thinking 有效态。
