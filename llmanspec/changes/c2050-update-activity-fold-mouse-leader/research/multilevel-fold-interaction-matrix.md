# Research: 多级折叠 × 鼠标/Leader 交互矩阵

> Change: `c2050-update-activity-fold-mouse-leader`
> 对照：`c1760` proposal 深挖 A–D；并行 `c2020`–`c2040`；性能候补 `c1505`/`c1370`/`c1535`。

## 相关 active changes（2026-08-11 快照）

| id | 主题 | 与本矩阵 |
|---|---|---|
| **c1760** | Activity L2/L3 多级折叠 | 主语义；本 change 适配交互 |
| **c2020** | 包鼠标地基 | 点击前提 |
| **c2030** | L1 leader 数字 toggle | 块级覆盖 + 编号 UX |
| **c2040** | 点击三角 + 字形 | 点击触发 |
| **c1505** | scrollback viewport 切片 | 视口可见编号/hit 更便宜；非硬依赖 |
| **c1370** | previous_lines 热缓冲封顶 | L2/L3 少画缓解压力；非替代 |
| **c1535** | 流式 wrap 尾优化 | ROI 低；与 fold 交互无关 |
| c1895–98 等 | status bar | 正交 |
| c1770 | worktree snapshot | 正交（proposal 曾提 mouse 为远景，非本波） |

## 交互矩阵（意向）

| 用户动作 | L0/L1 细账块 | L2 摘要行 | L3 Worked for |
|---|---|---|---|
| 全局 tools 翻转（若仍保留） | 改 default；覆盖表策略钉 | 不穿透（深挖 A） | 同左 |
| Leader 编号 + digit | toggle 该 **entry** 覆盖 | toggle/升一级该 **segment** | 同 L2 或只升到 L2 |
| 点击标记 | toggle entry | 对该段升/降一级 | 同左 |
| `Alt+Shift+E` | 无折叠段：静默 | `expandNearest` | 同左 |
| `Ctrl+Alt+Shift+E` | 无 Activity：静默 | `collapseNearest` | 同左 |
| Ctrl+T / Ctrl+O | 近窗口细账有效 | 段内不可见 → 外观不变 | 同左 |

## 目标 ID 空间

```text
FoldTarget::Entry(UiEntryId)      // L1 tool/diff/compaction/thinking?
FoldTarget::Segment(SegmentId)    // L2/L3 activity
```

Leader 建表时合并两类可见头行，统一近→远编号。点击命中同表。

## 性能检查表（适配 MUST）

| 项 | 要求 |
|---|---|
| 建 hit/编号表 | O(可见头行)，非 O(全历史) |
| toggle | 失效目标段/条目 paint；禁止全量 MD 重解析 |
| 鼠标 move | 忽略 |
| 与 ath25 | miss 计数不回退 |
| 与 c1505 | 若已切片，表只扫 slice；未切片则扫当前 upper 可见带 |
| 与 c1370 | 不放宽热缓冲；少画是副作用不是依赖 |

## 落地策略

1. **理想**：`c1760` apply 末段直接吸收本矩阵 → 本 id docs-only archive。
2. **现实**：`c1760` 先按原 MVP（无鼠标）落地 → 本 change 补适配。
3. **禁止**：在 `c1760` 未有段状态前，让 leader「假装」段级（会退回全局 bool）。

## Open（交 propose）

- 点 L2 标记：toggle 显隐 vs 升一级（与栈键对称性）。
- Thinking 是否进编号平面。
- `c2030` 与 `c1760` 对 `Alt+E` / `Alt+Shift+E` 的最终和弦表（一张表钉死）。
