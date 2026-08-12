# Research: 多级折叠 × 鼠标 / 块级覆盖交互矩阵

> Change: `c2050-update-activity-fold-mouse-leader`
> 对照：`c1760` proposal 深挖 A–D；`c2040` archive（L1 三角）；`c2045`（广义 FoldTarget，可吸收本票）。
> **2026-08-11**：原 `c2030` leader 编号已废弃；矩阵去掉 digit 行。
> **2026-08-12**：与 c2045 Q6=B 对齐——段命中默认被 c2045 吸收；本票 SSOT=本矩阵 + design 性能 MUST。

## 相关 changes

| id | 主题 | 与本矩阵 |
|---|---|---|
| **c1760** | Activity L2/L3 多级折叠 | 主语义；**硬阻塞**本票 start |
| **c2020** ✓ | 包鼠标地基 | 点击前提 |
| **c2040** ✓ | 点击三角 + L1 覆盖表 + 字形 | 手势/几何/ hit 管道样板 |
| **c2045** | 广义 FoldTarget + 剩余块 | **默认吸收** Segment 命中；本票 docs-only |
| **c1505** / **c1370** / **c1535** | 性能并列 | 非硬依赖 |

## 交互矩阵（意向 = 产品 SSOT）

| 用户动作 | L0/L1 细账块 | L2 摘要行 | L3 Worked for |
|---|---|---|---|
| 全局 tools 翻转（`Alt+E`） | 改 default；**清空**覆盖表 | 不穿透（深挖 A） | 同左 |
| 点击**三角列** | toggle entry 覆盖 | **对该段一级**升/降 | 同左 |
| `Alt+Shift+E` | 无折叠段：静默 | `expandNearest` | 同左 |
| `Ctrl+Alt+Shift+E` | 无 Activity：静默 | `collapseNearest` | 同左 |
| Ctrl+T / Ctrl+O | 近窗口细账有效 | 段内不可见 → 外观不变 | 同左 |

手势：**Left Down** + 拖选 latch（对齐 c2040）；几何：**仅三角列**。

## 目标 ID 空间

```text
FoldTarget::Tool|Diff|Ask|Thinking(id)   // c2040
FoldTarget::Segment(SegmentId)           // 段级；实现默认归 c2045 广义枚举
```

点击命中与覆盖/level 共用 target；**无** keyboard digit 编号平面。
**禁止**本票另起第二套段命中 API。

## 性能检查表（适配 MUST）

| 项 | 要求 |
|---|---|
| 建 hit 表 | O(可见头行)，非 O(全历史) |
| toggle | 失效目标段/条目 paint；禁止全量 MD 重解析 |
| 鼠标 move | 忽略 |
| 与 ath25 | miss 计数不回退 |
| 与 c1505 | 若已切片，表只扫 slice；未切片则扫当前 upper 可见带 |
| 与 c1370 | 不放宽热缓冲；少画是副作用不是依赖 |

## 落地策略（与 design 一致）

1. **默认（路径 A）**：c1760 归档 → c2045 吸收 `Segment` + 兑现本矩阵 → 本 id **docs-only archive**。
2. **Fallback（路径 B）**：c2045 design 明文排除 Segment → 本 change 独立 Specs/apply（仍硬等 c1760）。
3. **禁止**：在 `c1760` 未有段状态前，用全局 bool 假装段级点击；与 c2045 双写两套 hit。

## Open（已钉 → design）

| 原 Open | 钉 |
|---|---|
| 点 L2 标记：toggle 显隐 vs 升一级 | **对该段一级** |
| Thinking 是否可点 | 摘要行否；L0/L1 走 c2040 |
| `Alt+E` vs `Alt+Shift+E` 分层 | 上表 SSOT |
| 被 c2045 吸收？ | **默认是** → docs-only |
