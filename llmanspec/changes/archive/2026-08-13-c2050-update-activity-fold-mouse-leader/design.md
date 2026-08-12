# Design: c2050 多级 ActivityFold × 鼠标 / 覆盖适配

> **阶段**：**docs-only superseded**（2026-08-13；`absorbed_by` c2045）。
> **对照**：[`c1760`](../2026-08-12-c1760-add-tui-activity-fold/proposal.md) · [`c2045`](../2026-08-12-c2045-add-tui-fold-target-remaining/proposal.md) · [`c2040`](../2026-08-12-c2040-add-tui-mouse-click-fold-triangle/) · [`research/multilevel-fold-interaction-matrix.md`](./research/multilevel-fold-interaction-matrix.md)

## 一句话

在 **c1760 段状态已存在** 的前提下，让 L2/L3 摘要标记与 L1 块三角共用 **同一 `FoldTarget` / hit 管道**；本票默认 **被 c2045 吸收实现**，自身以语义矩阵 + 性能 MUST 为 SSOT，必要时才独立 Specs/apply。

## 与 c2045 边界（对齐 Q6=B）

| 职责 | **c2050（本票）** | **c2045** |
|---|---|---|
| 广义 `FoldTarget` 总装（Entry kind + Segment + 剩余） | **不**另起平行 API | **拥有**（扩展 c2040 `Tool/Diff/Ask/Thinking`） |
| Bash / Compaction / Ctrl+O viewport 命中 | 非目标 | **拥有** |
| L2/L3 **段**点击命中接线 | 默认 **让渡**给 c2045（吸收） | 吸收路径下 **拥有** `FoldTarget::Segment` + hit |
| 段 × L1 分层语义（c1760 深挖 A） | **SSOT**（矩阵 + 本 design） | 实现时 MUST 遵守本票矩阵 |
| 性能 MUST（可见 hit / 局部 invalidate / ath25） | **SSOT 条款** | 实现时 MUST 遵守 |
| 跨面动作语义笔记（Web 同源板） | MAY 文档同步 | 不独占 |
| L2/L3 文案 / 自动降级 / 栈键 | 非目标（属 c1760） | 非目标 |

### 吸收路径（默认钉死）

```text
路径 A — absorbed_by_c2045 → docs-only（默认）
  c1760 归档（段 level + 行距缝）
  → c2045 在广义 FoldTarget 中纳入 Segment + 段摘要三角 hit
  → 本票校验矩阵/性能条款已被 c2045(+c1760) 兑现
  → 本票 docs-only archive（无 Branch binding / 无 Specs landing / 无 apply）

路径 B — independent_apply（仅 fallback）
  仅当 c2045 design **明文排除** Segment（只做剩余块）时启用
  → 本票在 c1760 归档后 start → Specs → apply 段命中
```

**禁止**：本票与 c2045 **并行**落地两套段命中 / 两张 hit 表。

## 目标边界

| 在范围（语义 / 或 fallback 实现） | 不在范围 |
|---|---|
| `FoldTarget::Segment(id)` 与 L1 Entry 目标同构 | 重做 c1760 L2/L3 文案、自动降级、栈键 |
| 点摘要行**三角列** → 对该段 **升/降一级**（单目标） | keyboard fold-leader / 数字编号（已废） |
| 继承 c1760 深挖 A：L2/L3 段内 L1 全局/覆盖不穿透外观 | Bash / Compaction / viewport → c2045 |
| hit 表 O(可见头/摘要)；toggle 局部 invalidate | 实现 c1505 / c1370 / c1535 |
| 动作语义与 Web 同源板对齐（物理点击分面） | 块焦点常驻 EditorSlot；整行可点 |

## 依赖与时序

```text
Wave 0 ✓  c2020 mouse · c2070/c2071 AO
Wave 1 ✓  c2040 L1 三角 + FoldHitTable（已归档）
Wave 1'   c1760 多级折叠 MVP（段状态；本波可不点鼠标）  ← 硬阻塞
Wave 2    c2045 FoldTarget 总装 + 剩余块（+ 默认吸收 Segment）
          c2050：默认 docs-only 收口；仅 fallback 时独立 apply
```

- **硬依赖**：`c1760` 归档前 **禁止** `change start` / Specs / apply（无段状态则段点击无真值）。
- **软依赖**：`c2045` 吸收决策以本 design 默认 A 为准；若其 design 改钉 B，本票改走独立 apply（仍等 c1760）。

## 交互语义（钉死）

继承 [`research/multilevel-fold-interaction-matrix.md`](./research/multilevel-fold-interaction-matrix.md)：

| 用户动作 | L0/L1 细账块 | L2/L3 摘要行 |
|---|---|---|
| 点**三角列** | toggle Entry 覆盖（c2040） | **对该段一级** toggle（朝细/朝粗各一步；与单目标 `expandNearest`/`collapseNearest` 对称，非全局栈扫） |
| `Alt+E` / `Ctrl+T` / `Ctrl+O` | 改 default + 清族 overrides（c2040） | **不穿透**段外观（深挖 A） |
| `Alt+Shift+E` / `Ctrl+Alt+Shift+E` | 无折叠段：静默 | 最近段栈（c1760）；与定点点击正交 |
| Thinking | c2040 per-id；点三角 | **摘要行不点 Thinking**；仍全局/块键 |

手势 / 几何：**复用 c2040** — `Left Down` + latch；**仅三角列**；拖选中忽略 fold。

## 目标模型（意向；实现归 c2045 或 fallback 本票）

```text
// c2040 已有
FoldTarget::{ Tool | Diff | Ask | Thinking }(id)

// 段级（吸收进 c2045 广义枚举；禁止第二套类型名平面）
FoldTarget::Segment(SegmentId)
```

点击命中与（若有）段级覆盖/level 变更共用同一 target；**无** digit 编号平面。

## 性能 MUST（适配条款）

| 项 | 要求 |
|---|---|
| 建 hit 表 | O(可见头/摘要)，非 O(全历史) |
| toggle 段 | 失效该段行范围 paint；**禁止**全历史 MD 重解析 |
| 鼠标 move | 忽略 |
| ath25 | miss 计数不因段 toggle 回退上界 |
| c1505 / c1370 | 并列；少画是副作用，不替代封顶 |

## 测试 seam（fallback 独立 apply 时；吸收路径由 c2045 覆盖）

1. Harness：同屏 L1 块 + L2 摘要 — 点击各打中正确 `FoldTarget`
2. Harness：L2 段内 `Alt+E` 不改变该段外观（深挖 A）
3. Paint：混合屏段 toggle 不回退 ath25 miss 上界
4. 跨面文档：Web 同源板动作 id 与 TUI 一致（docs 检查）

## Open Questions（已钉）

| # | 钉 |
|---|---|
| 1 点 L2/L3 标记 | **对该段一级**升/降（非纯显隐 bool；非整行） |
| 2 Thinking | 摘要行**不可点**；L0/L1 仍走 c2040 per-id + `Ctrl+T` |
| 3 手势/几何 | **对齐 c2040**：Down 吞按 + latch；仅三角列 |
| 4 与 c2045 | **默认路径 A**：被吸收 → **本票 docs-only archive**（无 start/Specs/代码） |
| 5 路径 B 触发 | **仅当** c2045 design 明文「Segment 归 c2050」 |
| 6 与 c1760 | **硬阻塞**：未归档 → `blocked_on_c1760`；禁止假装段级点击 |
| 7 分层表 | `Alt+E` 族 vs `Alt+Shift+E` 栈 — 一张 SSOT=本 research 矩阵 |

## Start readiness

| 字段 | 值 |
|---|---|
| `ready_for_start` | **false** |
| `Start readiness` | **`blocked_on_c1760`** |
| 条件就绪（路径 A） | c1760 归档 ∧ c2045 已吸收 Segment 并兑现矩阵 → **跳过 start**，走 docs-only archive |
| 条件就绪（路径 B） | c1760 归档 ∧ c2045 明文排除 Segment → 方可 `change start` → Specs landing |

**当前**：c1760 未归档 → 保持 Designed / pre-start 文档齐；**不** Branch binding。
