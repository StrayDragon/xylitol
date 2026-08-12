# Tasks: c2045-add-tui-fold-target-remaining

> **前置**：c2040 已归档。决策见 `design.md` 决策表；Open Questions 已清。
> **门禁**：`change start` → Specs landing → `readyToImplement=true` 后再 apply。
> **Wave B** 标 `[blocked-by: c1760]`：c1760 未归档前勿实施 Segment。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 Designed 壳 | ✅ | proposal + design + tasks |
| 1 Specs landing | ⬜ | Branch binding 后 |
| 2–4 Wave A | ⬜ | Compaction + OutputViewport |
| 5–6 Wave B | ⬜ | Segment；吸收 c2050 |
| 7 收口 | ⬜ | validate + c2050 docs-only |

---

## 1. Specs landing

- [ ] 1.1 Branch binding：`llman sdd change start c2045-add-tui-fold-target-remaining`（干净树 + 默认分支）
- [ ] 1.2 `app-tui-transcript`：Compaction 三角 + OutputViewport hint 命中（产品级 MUST）
- [ ] 1.3 `app-tui-transcript`：Segment 点击 + 深挖 A 不穿透（可同批写下；apply 仍闸 c1760）
- [ ] 1.4 场景：`feature: false` unit + harness 指针（对齐 att20–22）
- [ ] 1.5 `app-tui-host`：仅当 latch/路由缺口需要时补；否则 skip
- [ ] 1.6 commit Specs landing → `readyToImplement`

## Apply backlog（实施时勾选）

### 2. FoldTarget 扩 Wave A

- [ ] 2.1 `FoldTarget` 增 `Compaction`、`OutputViewport`（保留既有四变体）
- [ ] 2.2 `toggle_fold_target`：Compaction → flip `compaction_expanded`；OutputViewport → flip `tools_output_expanded`
- [ ] 2.3 确认点 Compaction **不清** tools overrides、不翻 `tools_expanded`

### 3. Hit 登记 Wave A

- [ ] 3.1 Compaction 头行：Unicode/Ascii 三角列（同 GlyphSet）；登记 hit
- [ ] 3.2 Tool/Bash/Diff 等可见 Ctrl+O hint 行：登记 `OutputViewport` 命中带
- [ ] 3.3 复用 paint gen / 可见带；拖选 latch 不变

### 4. 验证 Wave A

- [ ] 4.1 harness：Compaction 三角 Mouse toggle；点正文不变
- [ ] 4.2 harness：OutputViewport hint Mouse toggle；与 Ctrl+O 同 bool
- [ ] 4.3 回归：Tool/Thinking/Diff/Ask 三角行为不变
- [ ] 4.4 ath25 miss 上界不回退；`just fmt` + 相关测

### 5. Wave B Segment `[blocked-by: c1760]`

- [ ] 5.1 确认 c1760 已归档且段 id↔行映射可用
- [ ] 5.2 `FoldTarget::Segment(id)` + toggle = 该段一级（非 nearest）
- [ ] 5.3 摘要行标记登记 hit（三角列-only）
- [ ] 5.4 分层：L2/L3 段内 L1 外观不穿透（深挖 A）

### 6. 验证 Wave B `[blocked-by: 5.*]`

- [ ] 6.1 harness：同屏 L1 + L2 各打中正确 target
- [ ] 6.2 harness：L2 段内 L1 全局/覆盖不改变该段外观
- [ ] 6.3 ath25 / 可见 hit O(可见) 抽检

### 7. 收口

- [ ] 7.1 `llman sdd validate c2045 --strict`（及改动 capability）
- [ ] 7.2 c2050：docs-only supersede → archive（无应用代码；指针回本 change）
- [ ] 7.3 确认未膨胀进 c1760 主交付 / 未复活 fold-leader

## 实现顺序

```text
1 Specs → 2 enum/toggle → 3 hit Wave A → 4 harness A
  ─┬─ c1760 archive
   └→ 5 Segment → 6 harness B → 7 validate + c2050 docs-only
```

## Open Questions

无。见 `proposal.md`（已钉）。
