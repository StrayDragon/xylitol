# Verify — c2045-add-tui-fold-target-remaining（Wave A only）

**Date:** 2026-08-12
**Branch:** `sdd/c2045-add-tui-fold-target-remaining`
**base_sha (proposal):** `839990764660a6b9f9014d1a88ee2d35aafeccd6`
**HEAD (pre-verify-report):** `a119f380` — `feat(tui): Wave A Compaction and OutputViewport fold mouse hits`
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached
**Scope of this report:** **Wave A only**（att29 / att30 / att32）。Wave B（att31 / Segment / c2050 docs-only）**deferred**，非本闸 CRITICAL。

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c2045-add-tui-fold-target-remaining` |
| `llman sdd show … --output json` | `readyToImplement=true` · `specsLanded=true` · `attached=true` |
| `llman sdd validate c2045 --strict --no-interactive --no-check` | **PASS**（INFO: depends_on archived c2040） |
| `llman sdd validate c2045 --strict --no-interactive`（含 check） | **PASS** |
| `llman sdd validate app-tui-transcript --strict --no-check` | **PASS**；Staleness OK |
| `cargo test --lib harness_mouse` | **7 passed**（含 compaction + output viewport） |
| `compaction_and_viewport_toggle_miss_bound` | **PASS**（ath25 上界） |
| c1760 | `stage=designed` · `readyToImplement=false` · **未归档** → Wave B blocked |
| `change finalize` / push | **未做**（本 verify 禁止） |

## Wave 切分结论

| Wave | 合约 | 实现 | Verify |
|---|---|---|---|
| **A** | att29 Compaction；att30 OutputViewport；att32 统一表（A 目标） | ✅ `FoldTarget::{Compaction,OutputViewport}` + hit 登记 + harness | **PASS** |
| **B** | att31 Segment；att32 Segment 段；吸收 c2050 | ⬜ `[blocked-by: c1760]` | **deferred / blocked**（**非 CRITICAL**） |

**Archive：** 不可仅凭 Wave A 全量 archive 本 change id——须等 Wave B（c1760 归档后 Segment + c2050 docs-only）闭合后再 `llman-sdd-archive` / `finalize`。Wave A 本闸可视为 apply 收口；变更整体仍 open。

---

## 合约轴（Spec）

审查对象：live `att29`–`att32`（`app-tui-transcript`）；实现对照 `widgets/{fold_hit,scrollback}`、`layout/root`、`host` hit_priority、harness。

### CRITICAL

（无 — Wave A）

### BLOCKED（非 CRITICAL；Wave B / 实现闸 c1760）

1. **att31 `segment-fold-marker-mouse`**：specs 已先写；`FoldTarget::Segment` **未**落地；c1760 仍 `designed` / 未 attach。按 proposal/design/tasks：`[blocked-by: c1760]`。**不得**标 CRITICAL 挡 Wave A 收口。
2. **Wave B tasks 5.* / 6.* / 7.***：plain list（无 checkbox，避免 strict Pending）；c1760 归档后改回 checkbox 再 apply。
3. **att32 中 Segment 登记**：A 目标（Compaction / OutputViewport）已进同一 `FoldHitTable`；Segment 行待 Wave B。

### WARNING

（无 Wave A 行为缺口）

### SUGGESTION

1. Wave B 落地时补同屏 L1+L2 harness（tasks 6.1–6.2）；勿另开第二 hit 管道。
2. c2050 docs-only supersede 留到 Wave B 收口（tasks 7.2），本闸不碰。

### Covered requirements（Wave A）

| Req | Verdict | Evidence |
|---|---|---|
| **att29** compaction-fold-triangle-mouse | **PASS** | Compaction Complete 头行三角列 hit；`harness_mouse_triangle_toggles_compaction_fold`：三角 flip `compaction_expanded`；正文不 toggle；tools overrides 不变；ath25 `compaction_and_viewport_toggle_miss_bound` |
| **att30** output-viewport-hint-mouse | **PASS** | Bash 长输出 hint 带 → `FoldTarget::OutputViewport`；`harness_mouse_hint_toggles_output_viewport`：hint 与 Ctrl+O 同 `tools_output_expanded`；Bash **无** L1 Tool 三角 |
| **att32** remaining-fold-targets-unified-hit（A） | **PASS** | Compaction / OutputViewport 经既有 `FoldHitTable` + `install_fold_triangle_hit_priority`；无第二管道；拖选 latch 复用 ath33 / `harness_mouse_drag_across_triangle…` |
| **att31** segment-fold-marker-mouse | **blocked** | 见上；非 CRITICAL |
| c2040 回归 att20–22 三角 | **PASS** | harness Tool / Thinking / Diff / Ask 仍绿 |

### Out-of-scope（正确未做 / deferred）

- `FoldTarget::Segment` / L2·L3 摘要点击（Wave B）
- c1760 段状态机本身
- c2050 独立 apply / docs-only archive（Wave B 后）
- 改 Alt+E×compaction **键盘**连带；Bash L1 三角；整行可点（Viewport 仅 hint 带）

---

## 标准轴（Standards）

审查对象：`base_sha...HEAD` 应用 diff（`fold_hit` / `scrollback` / `root` / harness）；对照 `AGENTS.md` 分层与开闭。

### CRITICAL

（无）

### WARNING

1. **`scrollback.rs` 体量继续上涨**（c2040 verify 已记）——本波把 Compaction 三角 + viewport hit helper 仍堆在同文件；后续 Segment 宜考虑外提 hit/emit，非行为缺口。

### SUGGESTION

1. `push_expandable_with_viewport_hit` 把 hint 登记收成局部 helper，避免各块复制——可读性好，无分层越界。
2. Host 仍复用 `set_transcript_hit_priority`；`app-tui-host` skip（ath33）与 design D9 一致，无新管道。
3. `toggle_fold_target` Compaction 分支不清 `tools_overrides`——与 D4 / att29 对齐；注释已钉。

### 坏味（判断性，非硬违规）

- **Divergent Change** 风险：`scrollback.rs` 继续因 fold 族多原因改动——Wave B 前可规划拆 hit 登记。
- **无** Speculative Generality：未预埋假 `Segment` 变体（正确；闸 c1760）。

---

## Dual-axis summary

| Axis | CRITICAL | Wave A | Wave B |
|---|---|---|---|
| Spec | 0 | PASS att29/30/32(A) | att31 **blocked**（非 CRITICAL） |
| Standards | 0 | PASS（软 WARNING：scrollback 体量） | n/a |

## Verdict

**Wave A：PASS**（无 CRITICAL；harness_mouse + ath25 miss 上界 + validate 全绿）。
**Wave B：deferred / blocked-by c1760**（标 blocked，**非 CRITICAL**）。

**可否 archive：** **须等 Wave B** 后再 archive / finalize 本 change；**不可**仅凭 Wave A 全量封存 `c2045`。Wave A 实现与本报告可留在分支上，待 c1760 → Wave B → 再 verify 全量。
