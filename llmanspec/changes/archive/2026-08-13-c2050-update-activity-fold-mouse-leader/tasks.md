# Tasks: c2050-update-activity-fold-mouse-leader

> **路径 A（默认）已收口**：被 [`c2045`](../../c2045-add-tui-fold-target-remaining/proposal.md) 吸收 → **docs-only archive**。
> 路径 B（独立 apply）**未启用**（c2045 design 未排除 Segment）。

## 0. Review 门 / 阻塞

- [x] 0.1 design 边界表与 c2045 吸收方案对齐（路径 A 默认 / B fallback）
- [x] 0.2 Open Questions 钉入 design（含 docs-only 路径）
- [x] 0.3 research 矩阵去掉 leader 编号；对齐 c2040 手势/三角列
- [x] 0.4 c1760 已归档（`archive/2026-08-12-c1760-…`）
- [x] 0.5 读 c2045 design：仍为路径 A（Segment 在 Wave B）

### Start readiness

| 项 | 值 |
|---|---|
| `ready_for_start` | **false**（docs-only；不 start） |
| 路径 | **A — absorbed_by_c2045** |

## 1–5. Specs / Apply（路径 B）

> **整节跳过** — 未启用。

## 6. 路径 A — docs-only 收口 — ✅

- [x] 6.1 确认 c2045（+c1760）已兑现：Segment hit、分层 A、性能 MUST、矩阵行为
- [x] 6.2 本目录保留 design/tasks/research 作决策史；**不**改 live specs
- [x] 6.3 docs-only archive；proposal 标注 `absorbed_by: c2045-…`
- [x] 6.4 Web 同源板：无新增动作 id 缺口（沿用 c1760/c2045 指针）
