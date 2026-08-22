# Design — 合约同句（零代码）

## 决策

- **退役而非改写 atc19**：attach 是唯一产品 TUI 拓扑，cue 在产品面不可达（`active_turn()` 默认 None，Remote 未覆写）；保留「不渲染」的 MUST 无验证价值。
- **cue 机械码保留**：`status_next_turn_cue_text` / `set_status_next_turn_cue` 由 harness 合成驱动与 embed-InProcess 组合合法使用；产品 attach 路径自然短路。非死码，不清。

## 措辞基准（代码真值）

- busy 时 footer 立即反映 selected（B2 手测）；本轮生成用开跑绑定由 Host 保证（server-core run 绑定）。
- 词汇表：词条删除 + 弃用表标注退役，不留防复活清单（llmanspec AGENTS 删除条款规则）。

## 验证

结构门禁（toon/feature 计数与 @req 一致性）+ lib/bdd 快门禁防回归；无行为断言变化。
