# Design · rail 默认皮肤

## 决策摘要

| 项 | 决定 |
|---|---|
| 默认皮肤 | **rail**（产品唯一默认；无 runtime wash 开关） |
| 布局 | `[1 bg rail][1 plain gutter][content…]`；User/Assistant flush |
| 成败 | 轨色 = pending→accent / success / error（可 soft-mix surface）；thinking→muted mix |
| Diff 本轮 | **解绑** tool-*-bg 一体洗底；保留「无 diff-*-bg 行底」；不做新 diff 皮肤 |
| 包 vs 产品 | 包提供 `paint_left_rail_line`（宽、rgb、line）；产品 `render_scrollback` 换调用 |
| 对照 | `agent_demo` MAY 保留 wash；产品不暴露 |

## 合约迁移图

```text
att4  tool-*-bg 洗底     →  status 轨 + gutter（无整行 tool-*-bg）
att5  apply_background   →  改：rail MUST 走包 paint_left_rail_line（wash helper 可留作非默认）
att9  bang tool-*-bg     →  bang 块同 rail
att10 整行洗底半句       →  删除；保留 ≥1 空行
att11 bang apply_bg      →  同 att5（rail helper）
att14 write 一体洗底     →  write header+正文共用同一轨色块（无整行 bg）
atc8  user-message-bg    →  默认关闭 / 改写为 MUST NOT 全行淡底
```

## 为何包一层 helper

产品与 demo 已各写一遍轨；缺公共 API 会分叉宽预算（1 vs 2 列）与复位 SGR。`apply_background_to_line` 仍服务 Markdown code bg 等，**不**删除；rail 是正交 paint。

## Diff 边界（防 scope creep）

本 change **禁止**：

- 为 diff 增删行各画独立色轨
- 改 `package-tui-diff` 默认 polarity 语义（除产品不再外包 wash）
- 引入第三套「classic」产品皮肤

后续 change 可在「无洗底信封」上自由试 diff 行级表达。

## 风险

| 风险 | 缓解 |
|---|---|
| 窄终端轨+gutter 吃 2 列 | `saturating_sub(2).max(1)`；单测窄宽 |
| 复制仍夹轨空格 | 已知终端依赖；语义复制另案（M2） |
| BDD 场景大批改写 | 垂直切片：先 helper+单测 → 改 scrollback → 改 feature |
