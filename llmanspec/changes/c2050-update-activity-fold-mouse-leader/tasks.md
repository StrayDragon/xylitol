# Tasks: c2050-update-activity-fold-mouse-leader

> **阶段**：Designed / pre-start（**未分支**）。
> **Start readiness**：`blocked_on_c1760`（见 §0 / design.md）。
> **默认路径**：被 [`c2045`](../c2045-add-tui-fold-target-remaining/proposal.md) 吸收 → **docs-only**（无 Specs / 无 apply）。
> **Fallback**：仅当 c2045 design 明文排除 Segment 时启用下方 Specs + Apply。

## 0. Review 门 / 阻塞

- [x] 0.1 design 边界表与 c2045 吸收方案对齐（路径 A 默认 / B fallback）
- [x] 0.2 Open Questions 钉入 design（含 docs-only 路径）
- [x] 0.3 research 矩阵去掉 leader 编号；对齐 c2040 手势/三角列
- [ ] 0.4 **硬阻塞**：等待 [`c1760`](../c1760-add-tui-activity-fold/proposal.md) 归档（段 level + 行距缝）
- [ ] 0.5 读 c2045 design：确认仍为路径 A；若改钉 B → 勾选下方 1–5 并开闸

### Start readiness（钉死）

| 项 | 值 |
|---|---|
| `ready_for_start` | **false** |
| 状态码 | **`blocked_on_c1760`** |
| 路径 A（默认）就绪条件 | c1760 归档 ∧ c2045 吸收 Segment 兑现 → **不 start**，走 §6 docs-only |
| 路径 B 就绪条件 | c1760 归档 ∧ c2045 明文排除 Segment → `ready_for_start=true` 后 §1 |

---

## 1. Specs landing（**仅路径 B**；绑定分支后）

> 路径 A：**整节跳过**。

- [ ] 1.1 Branch binding：`change start`（干净默认分支；`sdd/c2050-…`）
- [ ] 1.2 `app-tui-transcript`：Segment 目标 + 摘要三角命中 + 深挖 A 不穿透（新 req；场景 `feature: false` 优先）
- [ ] 1.3 host / hit_priority：复用 ath33 管道；段目标进同一 `FoldHitTable`（无第二 hook）
- [ ] 1.4 commit Specs landing → `readyToImplement`

## Apply backlog（**仅路径 B**）

### 2. 目标模型

- [ ] 2.1 扩展既有 `FoldTarget`（或 c2045 广义枚举）：`Segment(id)`；禁止平行类型平面
- [ ] 2.2 点摘要三角 → 该段一级升/降；与栈键正交

### 3. Hit + 分层

- [ ] 3.1 render 登记可见 L2/L3 摘要三角列（绑 paint gen）
- [ ] 3.2 L2/L3 段：`Alt+E`/`Ctrl+T`/`Ctrl+O` 不穿透外观（深挖 A）
- [ ] 3.3 拖选 latch 忽略 fold（对齐 c2040）

### 4. 性能

- [ ] 4.1 hit O(可见)；段 toggle 局部 invalidate；ath25 不回退

### 5. 验证

- [ ] 5.1 harness：L1+L2 同屏正确 `FoldTarget`；误点正文不折
- [ ] 5.2 harness：深挖 A；人验最短路径（design / proposal）
- [ ] 5.3 `llman sdd validate c2050 --strict`；`just fmt` + 相关测

---

## 6. 路径 A — docs-only 收口（默认）

- [ ] 6.1 确认 c2045（+c1760）已兑现：Segment hit、分层 A、性能 MUST、矩阵行为
- [ ] 6.2 本目录保留 design/tasks/research 作决策史；**不**改 live specs
- [ ] 6.3 docs-only archive（无实现 commit）；proposal 标注 absorbed_by_c2045
- [ ] 6.4 若有 Web 同源板动作 id 缺口 → 文档补一句（非本票代码）

## 实现顺序

```text
blocked_on_c1760
  → 读 c2045 design
       ├─ 路径 A → §6 docs-only archive
       └─ 路径 B → §1 Specs → §2–5 apply → validate
```

## Open Questions（已钉）

见 [`design.md`](./design.md)「Open Questions（已钉）」；无未决项挡 **文档齐**。
挡 **start** 的唯一硬项：§0.4 `c1760` 未归档。
