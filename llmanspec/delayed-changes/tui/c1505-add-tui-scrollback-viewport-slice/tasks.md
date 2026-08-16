# Tasks: c1505-add-tui-scrollback-viewport-slice

> **pre-start / P9-deferred**：当前无 branch binding、无 specs landing、无代码任务可执行。以下仅是 promote 闸；在 ROI 或 seam 未钉前不 start。

## 已完成的审计证据

- [x] T0a–c：B 假阴性、微基准 O(n) 路径
- [x] T0d：`E-hist-stream` long-stream 80/400/800 — `scroll_render` 约 27–30%，不随历史明显涨 → **未证明当前真实瓶颈**
- [x] T0e：核对 post-c2070/c2071 ApplicationOwned full-content + cheap reproject 现实
- [x] T0f：核对 post-c2040 `FoldHitTable` 完整 content-row 命中约束

## Promote 闸（未执行）

- [ ] G1（≤2h）：用真实 AO 长历史 profile 复测帧耗时 / CPU / `component_lines` 的历史增长；不得以 B-scroll samply 代替
- [ ] G2（≤2h）：在人确认 ROI 后，选择 package lazy/viewport-aware seam 或明确否决；不得直接新增孤立 `scroll_line_offset`
- [ ] G3（≤2h）：写出不变量验收矩阵：完整逻辑内容、cheap reproject、AO 上滚/回底、拖选、c2040 fold hit、fold 高度变化
- [ ] G4（人决）：根据 G1–G3 选择 `ready_for_start` 或继续 defer；在选择前不执行 `change start`

## 当前决策

**继续暂缓（defer）**。G1–G4 未满足前，不把 T1–T7 恢复为实现任务，也不绑定分支或编辑 live specs。
